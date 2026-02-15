use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use anyhow::{Context, Result, ensure};
use flv::tag::FlvTag;
use tracing::{debug, info};

use amf::amf0::{AmfObject, Sequence, Value};

use crate::{
    connection::NetConnection,
    constants::{CONTROL_CHUNK_STREAM_ID, CONTROL_MESSAGE_STREAM_ID},
    event::UserControlMessageEvent,
    message::{Message, control_message},
    message_type::MessageType,
};

mod functions;

use self::functions::handshake;

#[derive(Clone, Copy, Debug)]
enum ClientConnectionState {
    BeforeConnect,
    BeforePublish,
    BeforeMetadata,
    Running,
}

// pub struct ClientConnection {
//     state: ClientConnectionState,
// }

type OnConnectHandler = dyn Fn(&str);
type OnDiscnnectHandler = dyn Fn(&str);
type OnDataHandler = dyn Fn(&str, FlvTag);

pub struct Server {
    on_connect: Option<Box<OnConnectHandler>>,
    on_disconnect: Option<Box<OnDiscnnectHandler>>,
    on_data: Option<Box<OnDataHandler>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            on_connect: None,
            on_disconnect: None,
            on_data: None,
        }
    }

    pub fn on_connect(&mut self, f: impl Fn(&str) + 'static) {
        self.on_connect = Some(Box::new(f));
    }

    pub fn on_disconnect(&mut self, f: impl Fn(&str) + 'static) {
        self.on_disconnect = Some(Box::new(f));
    }

    pub fn on_data(&mut self, f: impl Fn(&str, FlvTag) + 'static) {
        self.on_data = Some(Box::new(f));
    }

    fn handle_stream(&self, mut stream: TcpStream) -> Result<()> {
        info!("New inbound");

        handshake(&mut stream)?;

        let mut state = ClientConnectionState::BeforeConnect;
        let mut conn = NetConnection::new(&mut stream);

        let mut last_video_timestamp = 0;
        let mut last_audio_timestamp = 0;

        let mut client_id = String::new();

        loop {
            let msg = conn.recv()?;

            match (state, msg.header().message_type) {
                (ClientConnectionState::BeforeConnect, MessageType::SetChunkSize) => {
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

                (ClientConnectionState::BeforeConnect, MessageType::Command) => {
                    // IN: `Command Message (connect)`

                    let mut iter = msg.payload().iter().copied();

                    let command = Value::deserialize(&mut iter)?.as_string()?.to_string();
                    ensure!(command == "connect", "Unexpected command");

                    let transmission_id = Value::deserialize(&mut iter)?.as_number()?.to_float();

                    let args = Value::deserialize(&mut iter)?.as_object()?.to_hashmap();

                    debug!(?args, "command: connect:");

                    client_id = args
                        .get("app")
                        .context("No `app` field")?
                        .as_string()?
                        .to_string();

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

                    state = ClientConnectionState::BeforePublish;
                    self.on_connect.as_ref().inspect(|f| (f)(&client_id));
                    info!("Connection started");
                }

                (ClientConnectionState::BeforePublish, MessageType::Command) => {
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

                            state = ClientConnectionState::BeforeMetadata;
                        }

                        _ => {
                            debug!(?command, ?transmission_id, "Unhandled command");
                        }
                    }
                }

                (ClientConnectionState::BeforeMetadata, MessageType::Data) => {
                    let mut iter = msg.payload().iter().copied();

                    let data = Sequence::deserialize(&mut iter)?;
                    debug!("{data:#?}");

                    state = ClientConnectionState::Running;
                }

                (ClientConnectionState::Running, MessageType::Command) => {
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

                (ClientConnectionState::Running, MessageType::VideoPacket) => {
                    let timestamp = msg.header().timestamp;
                    if timestamp < last_video_timestamp {
                        continue;
                    }
                    last_video_timestamp = timestamp;

                    let tag =
                        FlvTag::new(MessageType::VideoPacket.into(), timestamp, msg.payload())?;

                    self.on_data.as_ref().inspect(|f| (f)(&client_id, tag));
                }

                (ClientConnectionState::Running, MessageType::AudioPacket) => {
                    let timestamp = msg.header().timestamp;
                    if timestamp < last_audio_timestamp {
                        continue;
                    }
                    last_audio_timestamp = timestamp;

                    let tag =
                        FlvTag::new(MessageType::AudioPacket.into(), timestamp, msg.payload())?;

                    self.on_data.as_ref().inspect(|f| (f)(&client_id, tag));
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

        info!("Connection ended");

        Ok(())
    }

    pub fn run(self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr)?;

        for stream in listener.incoming().filter_map(Result::ok) {
            self.handle_stream(stream)?;
        }

        Ok(())
    }
}
