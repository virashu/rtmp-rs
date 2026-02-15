use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::SystemTime,
};

use anyhow::{Result, bail, ensure};
use flv::tag::FlvTag;
use tracing::{debug, info};

use amf::amf0::{AmfObject, Sequence, Value};
use rtmp::{
    connection::Connection,
    constants::{CONTROL_CHUNK_STREAM_ID, CONTROL_MESSAGE_STREAM_ID},
    event::UserControlMessageEvent,
    message::{Message, control_message},
    message_type::MessageType,
};

fn get_timestamp_u32() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Invalid system time")
        .as_secs() as u32
}

fn write_header(buf: &mut impl Write) -> Result<()> {
    buf.write_all(b"FLV")?;
    buf.write_all(&[0x01])?; // Version
    buf.write_all(&[0x01 | 0x04])?; // Data mask
    buf.write_all(&[0x00, 0x00, 0x00, 0x09])?; // Header size

    buf.write_all(&[0x00, 0x00, 0x00, 0x00])?; // Zero-th tag size (0)

    Ok(())
}

#[allow(clippy::cast_possible_truncation)]
fn handshake(stream: &mut TcpStream) -> Result<()> {
    let _span = tracing::info_span!("handshake").entered();

    let mut buf = [0; 1536];

    // C0
    stream.read_exact(&mut buf[..1])?;
    let init = buf[0];
    if init == 0x03 {
        tracing::info!("Got init byte");
    } else {
        bail!("Wrong version");
    }

    // S0
    stream.write_all(&[0x03])?;

    // S1
    let server_send_timestamp = get_timestamp_u32();
    let server_signature = &[0u8; 1528];

    {
        stream.write_all(&server_send_timestamp.to_be_bytes())?;
        stream.write_all(&[0; 4])?;
        stream.write_all(server_signature)?;
    }

    // C1 -> S2
    {
        stream.read_exact(&mut buf)?;
        let client_recv_timestamp = get_timestamp_u32();

        buf[4..8].copy_from_slice(&client_recv_timestamp.to_be_bytes());
        stream.write_all(&buf)?;
    }

    // C2
    {
        stream.read_exact(&mut buf)?;
        ensure!(&buf[8..] == server_signature, "Server signature mismatch");
    }

    info!("Done");

    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum State {
    BeforeConnect,
    BeforePublish,
    BeforeMetadata,
    Running,
}

fn connect(stream: &mut TcpStream) -> Result<()> {
    let _span = tracing::info_span!("connect").entered();
    let mut conn = Connection::new(stream);

    let mut state = State::BeforeConnect;
    let mut last_video_timestamp = 0;
    let mut last_audio_timestamp = 0;

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("runtime/video.flv")?;
    write_header(&mut file)?;

    // conn.config.max_chunk_payload_size = 128;

    loop {
        let msg = conn.recv()?;

        match (state, msg.header().message_type) {
            (State::BeforeConnect, MessageType::SetChunkSize) => {
                // IN
                {
                    let raw_value: [u8; 4] = msg.payload().as_ref().try_into()?;
                    let value = u32::from_be_bytes(raw_value);
                    conn.config.max_chunk_payload_size = value;
                    info!("The client set a chunk size limit: {value} Bytes");
                }

                // OUT
                {
                    // Set the same for the output
                    conn.send(
                        CONTROL_CHUNK_STREAM_ID,
                        Message::new(
                            MessageType::SetChunkSize,
                            0, // Ignored
                            CONTROL_MESSAGE_STREAM_ID,
                            &conn.config.max_chunk_payload_size.to_be_bytes(),
                        )?,
                    )?;
                }
            }

            (State::BeforeConnect, MessageType::Command) => {
                // IN: `Command Message (connect)`

                let mut iter = msg.payload().iter().copied();

                let command = Value::deserialize(&mut iter)?.as_string()?.to_string();
                ensure!(command == "connect", "Unexpected command");

                let transmission_id = Value::deserialize(&mut iter)?.as_number()?.to_float();

                let args = Value::deserialize(&mut iter)?.as_object()?.to_hashmap();

                debug!(?args, "command: connect:");

                // OUT: `Window Acknowledgement Size`
                conn.send(
                    CONTROL_CHUNK_STREAM_ID,
                    control_message::window_acknowledgement_size(0x10000),
                )?;

                // OUT: `Set Peer Bandwidth`
                conn.send(
                    CONTROL_CHUNK_STREAM_ID,
                    control_message::set_peer_bandwidth(0x10000, 0),
                )?;

                // OUT: `User Control Message (StreamBegin)`
                conn.send(
                    CONTROL_CHUNK_STREAM_ID,
                    control_message::user_control_message(
                        UserControlMessageEvent::StreamBegin,
                        &[0x00, 0x00, 0x00, 0x00],
                    )?,
                )?;

                // OUT: `Command Message(_result - connect response)`
                {
                    let payload = Sequence::from(&[
                        Value::try_from("_result")?,
                        Value::from(transmission_id),
                        Value::Object(AmfObject::new([(
                            String::from("flashVer"),
                            Value::try_from("FMLE/3.0 (compatible; FMSc/1.0)")?,
                        )])?),
                        Value::Object(AmfObject::new([
                            (String::from("level"), Value::try_from("status")?),
                            (
                                String::from("code"),
                                Value::try_from("NetConnection.Connect.Success")?,
                            ),
                            (
                                String::from("description"),
                                Value::try_from("Connection succeeded")?,
                            ),
                            (String::from("clientId"), Value::from(1337.0)),
                            (String::from("objectEncoding"), Value::from(0.0)),
                        ])?),
                    ])
                    .serialize();

                    conn.send(
                        CONTROL_CHUNK_STREAM_ID,
                        Message::new(
                            MessageType::Command,
                            0, // Ignored
                            CONTROL_MESSAGE_STREAM_ID,
                            &payload,
                        )?,
                    )?;
                }

                state = State::BeforePublish;
            }

            (State::BeforePublish, MessageType::Command) => {
                let mut iter = msg.payload().iter().copied();

                let command = Value::deserialize(&mut iter)?.as_string()?.to_string();
                let transmission_id = Value::deserialize(&mut iter)?.as_number()?.to_float();

                match command.as_ref() {
                    "createStream" => {
                        let args = Value::deserialize(&mut iter)?;
                        debug!(?args, "command: createStream:");

                        // OUT: _result
                        {
                            let payload = Sequence::from(&[
                                Value::try_from("_result")?,
                                Value::from(transmission_id),
                                Value::Null,
                                Value::from(100.0),
                            ])
                            .serialize();

                            conn.send(
                                CONTROL_CHUNK_STREAM_ID,
                                Message::new(
                                    MessageType::Command,
                                    0, // Ignored
                                    CONTROL_MESSAGE_STREAM_ID,
                                    &payload,
                                )?,
                            )?;
                        }
                    }

                    "publish" => {
                        _ = Value::deserialize(&mut iter)?; // Null

                        let publishing_name =
                            Value::deserialize(&mut iter)?.as_string()?.to_string();
                        let publishing_type =
                            Value::deserialize(&mut iter)?.as_string()?.to_string();

                        debug!(?publishing_name, ?publishing_type, "command: publish:");

                        // OUT: onStatus
                        {
                            let payload = Sequence::from(&[
                                Value::try_from("onStatus")?,
                                Value::from(0.0),
                                Value::Null,
                                Value::Object(AmfObject::new([
                                    (String::from("level"), Value::try_from("status")?),
                                    (
                                        String::from("code"),
                                        Value::try_from("NetStream.Publish.Start")?,
                                    ),
                                    (
                                        String::from("description"),
                                        Value::try_from("Connection succeeded")?,
                                    ),
                                ])?),
                            ])
                            .serialize();

                            conn.send(
                                CONTROL_CHUNK_STREAM_ID,
                                Message::new(
                                    MessageType::Command,
                                    0, // Ignored
                                    CONTROL_MESSAGE_STREAM_ID,
                                    &payload,
                                )?,
                            )?;
                        }

                        state = State::BeforeMetadata;
                    }

                    _ => {
                        debug!(?command, ?transmission_id, "Unhandled command");
                    }
                }
            }

            (State::BeforeMetadata, MessageType::Data) => {
                let mut iter = msg.payload().iter().copied();

                let data = Sequence::deserialize(&mut iter)?;
                debug!("{data:#?}");

                state = State::Running;
            }

            (State::Running, MessageType::Command) => {
                let mut iter = msg.payload().iter().copied();

                let command = Value::deserialize(&mut iter)?.as_string()?.to_string();
                // let transmission_id = Value::deserialize(&mut iter)?.as_number()?.to_float();
                // let args = Value::deserialize(&mut iter)?;

                if command == "deleteStream" {
                    debug!("command: deleteStream");
                    break;
                }
            }

            (_, MessageType::Command) => {
                let mut iter = msg.payload().iter().copied();

                let command = Value::deserialize(&mut iter)?.as_string()?.to_string();
                let transmission_id = Value::deserialize(&mut iter)?.as_number()?.to_float();
                let args = Value::deserialize(&mut iter)?;

                debug!(?command, ?transmission_id, ?args);
            }

            (State::Running, MessageType::VideoPacket) => {
                let timestamp = msg.header().timestamp;

                if timestamp < last_video_timestamp {
                    continue;
                }
                last_video_timestamp = timestamp;

                let tag = FlvTag::new(MessageType::VideoPacket.into(), timestamp, msg.payload())?;

                file.write_all(&tag.serialize())?;
                file.write_all(&(tag.size() as u32).to_be_bytes())?;
            }

            (State::Running, MessageType::AudioPacket) => {
                let timestamp = msg.header().timestamp;

                if timestamp < last_audio_timestamp {
                    continue;
                }

                last_audio_timestamp = timestamp;

                let tag = FlvTag::new(MessageType::AudioPacket.into(), timestamp, msg.payload())?;

                file.write_all(&tag.serialize())?;
                file.write_all(&(tag.size() as u32).to_be_bytes())?;
            }

            _ => {
                // debug!(
                //     stream = msg.header.stream_id,
                //     type = ?msg.header.message_type,
                //     size = msg.header.payload_length,
                // );
            }
        }
    }

    // IN: `Window Acknowledgement Size`
    // {
    //     let chunk = Chunk::read_from(stream)?;
    //     let mut iter = chunk.content.iter().copied();
    // }

    Ok(())
}

fn handle(mut stream: TcpStream) -> Result<()> {
    info!("New connection");

    handshake(&mut stream)?;
    connect(&mut stream)?;

    info!("Connection over");

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_target(false)
        // .pretty()
        .init();

    let listener = TcpListener::bind("0.0.0.0:1935")?;

    for stream in listener.incoming().filter_map(Result::ok) {
        handle(stream)?;
    }

    Ok(())
}
