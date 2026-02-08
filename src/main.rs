use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::SystemTime,
};

use anyhow::{Result, bail, ensure};
use tracing::{debug, info};

use amf::amf0::{AmfNumber, AmfObject, AmfString, Value};
use rtmp::{chunk::Chunk, message_type::MessageType};

fn get_timestamp_u32() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Invalid system time")
        .as_secs() as u32
}

/// <https://en.wikipedia.org/wiki/Real-Time_Messaging_Protocol#Handshake>
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

fn connect(stream: &mut TcpStream) -> Result<()> {
    let _span = tracing::info_span!("connect").entered();

    // IN: `Set Chunk Size`
    {
        let chunk = Chunk::read_from(stream)?;
        let raw_value: [u8; 4] = chunk.payload.as_ref().try_into()?;
        let value = u32::from_be_bytes(raw_value);

        info!("The client set a chunk size limit: {value} Bytes");
    }

    // IN: `Command Message (connect)`
    {
        let chunk = Chunk::read_from(stream)?;
        let mut iter = chunk.payload.iter().copied();

        let command = Value::deserialize(&mut iter)?.as_string()?.to_string();
        ensure!(command == "connect", "Unexpected command");

        let transmission_id = Value::deserialize(&mut iter)?.as_number()?.to_float();
        ensure!(transmission_id == 1.0, "Unexpected command");

        let args = Value::deserialize(&mut iter)?.as_object()?.to_hashmap();
        debug!(?args);
    }

    // OUT: `Window Acknowledgement Size`
    {
        #[rustfmt::skip]
        let window_acknowledgement_size = [
            0x02, // Basic header (full message header / 12 Bytes)
            0x00, 0x00, 0x00, // Timestamp (ignored)
            0x00, 0x00, 0x04, // Length (4 Bytes)
            MessageType::WindowAcknowledgementSize as u8, // Type
            0x00, 0x00, 0x00, 0x00, // Stream ID
            0x00, 0x00, 0x01, 0x00, // Payload
        ];
        stream.write_all(&window_acknowledgement_size)?;
    }

    // OUT: `Set Peer Bandwidth`
    {
        #[rustfmt::skip]
        let set_peer_bandwidth = [
            0x02, // Basic header (full message header / 12 Bytes)
            0x00, 0x00, 0x00, // Timestamp (ignored)
            0x00, 0x00, 0x05, // Length (5 Bytes)
            MessageType::SetPeerBandwidth as u8, // Type
            0x00, 0x00, 0x00, 0x00, // Stream ID
            0x00, 0x00, 0x01, 0x00, 0x00, // Payload
        ];
        stream.write_all(&set_peer_bandwidth)?;
    }

    // IN: `Window Acknowledgement Size`
    // {
    //     let chunk = Chunk::read_from(stream)?;
    //     let mut iter = chunk.content.iter().copied();
    // }

    // OUT: `User Control Message (StreamBegin)`
    {
        #[rustfmt::skip]
        let user_control_message = [
            0x02, // Basic header (full message header / 12 Bytes)
            0x00, 0x00, 0x00, // Timestamp (ignored)
            0x00, 0x00, 0x05, // Length (5 Bytes)
            MessageType::UserControlMessage as u8, // Type
            0x00, 0x00, 0x00, 0x00, // Stream ID
            0x00, 0x00, // Event (= `StreamBegin`)
            0x00, 0x00, 0x11, 0x11, // Event data
        ];
        stream.write_all(&user_control_message)?;
    }

    // OUT: `Command Message(_result - connect response)`
    {
        let mut payload = Vec::<u8>::new();
        payload.extend(&Value::String(AmfString::new("_result")?).serialize());
        payload.extend(&Value::Number(AmfNumber::new(1.0)).serialize());
        payload.extend(
            &Value::Object(AmfObject::new([(
                String::from("flashVer"),
                Value::String(AmfString::new("FMLE/3.0 (compatible; FMSc/1.0)")?),
            )])?)
            .serialize(),
        );
        payload.extend(&Value::Object(AmfObject::new([])?).serialize());

        let len = &payload.len().to_be_bytes()[5..8];

        #[rustfmt::skip]
        let user_control_message_header = [
            0x02, // Basic header (full message header / 12 Bytes)
            0x00, 0x00, 0x00, // Timestamp (ignored)
            len[0], len[1], len[2], // Length
            MessageType::Command as u8, // Type
            0x00, 0x00, 0x00, 0x00, // Stream ID
        ];
        stream.write_all(&user_control_message_header)?;
        stream.write_all(&payload)?;
    }

    // Listen for any messages
    loop {
        let chunk = Chunk::read_from(stream)?;
        debug!(?chunk);
    }

    // Ok(())
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
        .init();

    let listener = TcpListener::bind("0.0.0.0:1935")?;

    for stream in listener.incoming().filter_map(Result::ok) {
        handle(stream)?;
    }

    Ok(())
}
