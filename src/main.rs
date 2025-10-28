use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    time::SystemTime,
};

use rtmp::packet::Packet;

// https://en.wikipedia.org/wiki/Real-Time_Messaging_Protocol#Handshake
fn handshake(stream: &mut TcpStream) {
    let mut buf = [0; 1528];
    let mut reader = BufReader::new(&mut *stream);

    //
    // Inbound
    //

    // C0
    reader.read_exact(&mut buf[..1]).unwrap();
    let init = buf[0];
    if init == 0x03 {
        println!("Got init byte!");
    }

    // C1 (client time)
    reader.read_exact(&mut buf[..4]).unwrap();
    let client_send_timestamp = buf[..4].to_vec();
    let client_receive_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_be_bytes();

    // C1 (server time - zeroes)
    reader.consume(4);

    // C1 (random)
    reader.read_exact(&mut buf[..]).unwrap();

    //
    // Outbound
    //

    // S0
    stream.write_all(&[0x03]).unwrap();

    let server_send_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_be_bytes();

    // S1
    stream.write_all(&server_send_timestamp).unwrap();
    stream.write_all(&[0, 0, 0, 0]).unwrap();
    stream.write_all(&buf).unwrap();

    // S2
    stream.write_all(&client_send_timestamp).unwrap();
    stream.write_all(&client_receive_timestamp).unwrap();
    stream.write_all(&buf).unwrap();

    // C2
    let mut reader = BufReader::new(&mut *stream);
    reader.consume(1536);

    println!("Handshake Done!");
}

fn handle(mut stream: TcpStream) {
    handshake(&mut stream);

    loop {
        let mut reader = BufReader::new(&mut stream);

        match Packet::read_from(&mut reader) {
            Ok(packet) => println!("Packet: {packet:#?}"),
            Err(e) => eprintln!("Error: {e}"),
        }
    }
}

pub fn serve() {
    let listener = TcpListener::bind("0.0.0.0:1935").unwrap();

    for stream in listener.incoming().filter_map(Result::ok) {
        handle(stream);
    }
}

fn main() {
    serve();
}
