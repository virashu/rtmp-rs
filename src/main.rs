use std::{fs::OpenOptions, io::Write};

use anyhow::Result;

use rtmp::server::Server;

fn write_header(buf: &mut impl Write) -> Result<()> {
    buf.write_all(b"FLV")?;
    buf.write_all(&[0x01])?; // Version
    buf.write_all(&[0x01 | 0x04])?; // Data mask
    buf.write_all(&[0x00, 0x00, 0x00, 0x09])?; // Header size

    buf.write_all(&[0x00, 0x00, 0x00, 0x00])?; // Zero-th tag size (0)

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_target(false)
        .init();

    let mut server = Server::new();

    server.on_connect(|client_id| {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(format!("runtime/{client_id}.flv"))
            .expect("Failed to open file");

        write_header(&mut file).expect("Failed to write header");
    });

    server.on_data(|client_id, data| {
        let mut file = OpenOptions::new()
            .append(true)
            .open(format!("runtime/{client_id}.flv"))
            .expect("Failed to open file");

        file.write_all(&data.serialize()).expect("Failed to write");
        file.write_all(&(data.size() as u32).to_be_bytes())
            .expect("Failed to write");
    });

    server.run("0.0.0.0:1935")?;

    Ok(())
}
