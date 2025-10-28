use std::io::BufRead;

use crate::header::Header;

// fn amf(stream: &mut impl BufRead) {
//     stream.consume(1);
//     let command_name = amf_string(stream);

//     stream.consume(1);
//     let transaction_id = amf_number(stream);

//     stream.consume(1);
// }

#[derive(Debug)]
pub struct Packet {
    pub header: Header,
    pub body: Option<Vec<u8>>,
}

impl Packet {
    pub fn read_from(stream: &mut impl BufRead) -> anyhow::Result<Self> {
        let header = Header::read_from(stream)?;

        let body = if let Some(size) = header.packet_length {
            let mut buf = Vec::new();
            stream.read_exact(&mut buf[..(size as usize)])?;
            Some(buf)
        } else {
            None
        };

        Ok(Self { header, body })
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    #[test]
    fn test_packet() {
        // ␃ ␀ @ I ␀ ␀ ␙ ␔ ␀ ␀ ␀ ␀ ␂ ␀ ␌ c r e a t e S t r e a m ␀ @ ␀ ␀ ␀ ␀ ␀ ␀ ␀ ␅
        let mut data: &[u8] = &[
            0x03, 0x00, 0x0B, 0x68, 0x00, 0x00, 0x19, 0x14, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x0C, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x53, 0x74, 0x72, 0x65, 0x61, 0x6D, 0x00,
            0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05,
        ];

        let mut reader = BufReader::new(&mut data);
        let packet = Packet::read_from(&mut reader).unwrap();
    }
}
