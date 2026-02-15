use std::{
    io::{Read, Write},
    time::SystemTime,
};

use anyhow::{Result, bail, ensure};
use tracing::info;

fn get_timestamp_u32() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Invalid system time")
        .as_secs() as u32
}

pub fn handshake<S>(stream: &mut S) -> Result<()>
where
    S: Read + Write,
{
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
