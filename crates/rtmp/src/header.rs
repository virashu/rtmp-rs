use std::{convert::TryFrom, io::BufRead};

use crate::{
    message_type::MessageType,
    util::{raw_u32_be, raw_u32_le},
};

#[derive(Debug, PartialEq)]
pub enum ChunkHeaderType {
    Full,
    Semi,
    BasicTimestamp,
    Basic,
}

fn header(raw: u8) -> ChunkHeaderType {
    // Get two most significant bits
    let value = (raw & 0b1100_0000) >> 6;

    match value {
        0b00 => ChunkHeaderType::Full,
        0b01 => ChunkHeaderType::Semi,
        0b10 => ChunkHeaderType::BasicTimestamp,
        0b11 => ChunkHeaderType::Basic,
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct Header {
    pub header_length: u32,
    pub chunk_header_type: ChunkHeaderType,
    pub chunk_stream_id: u32,
    pub timestamp_delta: Option<u32>,
    pub packet_length: Option<u32>,
    pub message_type: Option<MessageType>,
    pub message_stream_id: Option<u32>,
}

impl Header {
    pub fn read_from(stream: &mut impl BufRead) -> anyhow::Result<Self> {
        let mut buf: [u8; _] = [0; 10];

        // Byte #1 (1 byte)
        stream.read_exact(&mut buf[..1])?;
        let chunk_header_type = header(buf[0]);

        let least = buf[0] & 0b0011_1111;
        let chunk_stream_id: u32 = match least {
            1 => {
                // additional 2 bytes after this one
                stream.read_exact(&mut buf[..2])?;
                todo!()
            }
            2 => {
                // low level messages
                todo!()
            }
            x => x.into(),
        };

        // Byte #2-4 (3 bytes)
        let timestamp_delta = if chunk_header_type == ChunkHeaderType::Basic {
            None
        } else {
            stream.read_exact(&mut buf[..3])?;
            Some(raw_u32_be(&buf[..3]))
        };

        // Byte #5-7 (3 bytes)
        let packet_length = if chunk_header_type == ChunkHeaderType::Full
            || chunk_header_type == ChunkHeaderType::Semi
        {
            stream.read_exact(&mut buf[..3])?;
            Some(raw_u32_be(&buf[..3]))
        } else {
            None
        };

        // Byte #8 (1 byte)
        let message_type = if matches!(
            chunk_header_type,
            ChunkHeaderType::Full | ChunkHeaderType::Semi
        ) {
            stream.read_exact(&mut buf[..1])?;
            Some(MessageType::try_from(buf[0])?)
        } else {
            None
        };

        // Byte #9-12 (4 bytes)
        let message_stream_id = if chunk_header_type == ChunkHeaderType::Full {
            stream.read_exact(&mut buf[..4])?;
            Some(raw_u32_le(&buf[..4]))
        } else {
            None
        };

        let header_length: u32 = match chunk_header_type {
            ChunkHeaderType::Full => 12,
            ChunkHeaderType::Semi => 8,
            ChunkHeaderType::BasicTimestamp => 4,
            ChunkHeaderType::Basic => 1,
        };

        Ok(Self {
            header_length,
            chunk_header_type,
            chunk_stream_id,
            timestamp_delta,
            packet_length,
            message_type,
            message_stream_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::message_type::MessageType;

    use super::*;

    #[test]
    fn test_read_from() {
        // ␃ ␀ @ I ␀ ␀ ␙ ␔ ␀ ␀ ␀ ␀ ␂ ␀ ␌ c r e a t e S t r e a m ␀ @ ␀ ␀ ␀ ␀ ␀ ␀ ␀ ␅
        let mut data: &[u8] = &[
            0x03, 0x00, 0x0B, 0x68, 0x00, 0x00, 0x19, 0x14, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x0C, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x53, 0x74, 0x72, 0x65, 0x61, 0x6D, 0x00,
            0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05,
        ];

        let mut reader = BufReader::new(&mut data);
        let header = Header::read_from(&mut reader).unwrap();

        assert_eq!(header.header_length, 12);
        assert_eq!(header.chunk_header_type, ChunkHeaderType::Full);
        assert_eq!(header.timestamp_delta, Some(0x00_0b_68));
        assert_eq!(header.message_type, Some(MessageType::Command));
        assert_eq!(header.packet_length, Some(25));
    }
}
