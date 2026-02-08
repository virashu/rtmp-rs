use std::{convert::TryFrom, io::Read};

use anyhow::{Context, Result};

use crate::{
    message_type::MessageType,
    util::{raw_u32_be, raw_u32_le},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChunkHeaderType {
    Type0,
    Type1,
    Type2,
    Type3,
}

/// Serialized size: 1B
#[derive(Debug)]
pub struct ChunkBasicHeader {
    pub chunk_header_type: ChunkHeaderType,
    pub chunk_stream_id: u32,
}

impl ChunkBasicHeader {
    pub fn read_from(stream: &mut impl Read) -> Result<Self> {
        let mut buf: [u8; _] = [0; 1];
        stream.read_exact(&mut buf[..1])?;

        let chunk_header_type = match (buf[0] & 0b1100_0000) >> 6 {
            0b00 => ChunkHeaderType::Type0,
            0b01 => ChunkHeaderType::Type1,
            0b10 => ChunkHeaderType::Type2,
            0b11 => ChunkHeaderType::Type3,

            _ => unreachable!(),
        };

        let chunk_stream_id = match buf[0] & 0b0011_1111 {
            0 => {
                // 2-Byte ID
                let mut buf: [u8; _] = [0; 1];
                stream.read_exact(&mut buf)?;
                u32::from(buf[0]) + 64
            }
            1 => {
                // 3-Byte ID
                let mut buf: [u8; _] = [0; 2];
                stream.read_exact(&mut buf)?;
                u32::from(u16::from_be_bytes(buf)) + 64
            }
            x => u32::from(x),
        };

        Ok(Self {
            chunk_header_type,
            chunk_stream_id,
        })
    }
}

#[derive(Debug)]
pub struct ChunkMessageHeader {
    pub timestamp: Option<u32>,
    pub message_length: Option<u32>,
    pub message_type: Option<MessageType>,
    pub message_stream_id: Option<u32>,
}

impl ChunkMessageHeader {
    fn read_type_0(stream: &mut impl Read) -> Result<Self> {
        let mut buf = [0u8; 4];

        // Timestamp
        stream.read_exact(&mut buf[..3])?;
        let timestamp = raw_u32_be(&buf[..3]);

        // Message length
        stream.read_exact(&mut buf[..3])?;
        let message_length = raw_u32_be(&buf[..3]);

        // Message type ID
        stream.read_exact(&mut buf[..1])?;
        let message_type_id = buf[0];
        let message_type =
            MessageType::try_from(message_type_id).context("Failed to parse message type ID")?;

        // Message stream ID
        stream.read_exact(&mut buf[..4])?;
        let message_stream_id = raw_u32_le(&buf[..4]);

        Ok(Self {
            timestamp: Some(timestamp),
            message_length: Some(message_length),
            message_type: Some(message_type),
            message_stream_id: Some(message_stream_id),
        })
    }

    fn read_type_1(stream: &mut impl Read) -> Result<Self> {
        let mut buf: [u8; _] = [0; 3];

        // Timestamp
        stream.read_exact(&mut buf[..3])?;
        let timestamp = raw_u32_be(&buf[..3]);

        // Message length
        stream.read_exact(&mut buf[..3])?;
        let message_length = raw_u32_be(&buf[..3]);

        // Message type ID
        stream.read_exact(&mut buf[..1])?;
        let message_type_id = buf[0];
        let message_type =
            MessageType::try_from(message_type_id).context("Failed to parse message type ID")?;

        Ok(Self {
            timestamp: Some(timestamp),
            message_length: Some(message_length),
            message_type: Some(message_type),
            message_stream_id: None,
        })
    }

    fn read_type_2(stream: &mut impl Read) -> Result<Self> {
        let mut buf: [u8; _] = [0; 3];

        // Timestamp
        stream.read_exact(&mut buf[..3])?;
        let timestamp = raw_u32_be(&buf[..3]);

        Ok(Self {
            timestamp: Some(timestamp),
            message_length: None,
            message_type: None,
            message_stream_id: None,
        })
    }

    fn read_type_3() -> Self {
        Self {
            timestamp: None,
            message_length: None,
            message_type: None,
            message_stream_id: None,
        }
    }

    pub fn read_from(stream: &mut impl Read, header_type: ChunkHeaderType) -> Result<Self> {
        match header_type {
            ChunkHeaderType::Type0 => Self::read_type_0(stream),
            ChunkHeaderType::Type1 => Self::read_type_1(stream),
            ChunkHeaderType::Type2 => Self::read_type_2(stream),
            ChunkHeaderType::Type3 => Ok(Self::read_type_3()),
        }
    }
}

#[derive(Debug)]
pub struct ChunkHeader {
    pub basic_header: ChunkBasicHeader,
    pub message_header: ChunkMessageHeader,
}

impl ChunkHeader {
    pub fn read_from(stream: &mut impl Read) -> Result<Self> {
        let basic_header = ChunkBasicHeader::read_from(stream)?;
        let mut message_header =
            ChunkMessageHeader::read_from(stream, basic_header.chunk_header_type)?;

        // Extended Timestamp
        if message_header.timestamp == Some(0xFF_FF_FF) {
            let mut buf: [u8; _] = [0; 4];
            stream.read_exact(&mut buf)?;
            message_header.timestamp = Some(u32::from_be_bytes(buf));
        }

        Ok(Self {
            basic_header,
            message_header,
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
        let header = ChunkHeader::read_from(&mut reader).unwrap();

        assert_eq!(
            header.basic_header.chunk_header_type,
            ChunkHeaderType::Type0
        );
        assert_eq!(header.message_header.timestamp, Some(0x00_0b_68));
        assert_eq!(
            header.message_header.message_type,
            Some(MessageType::Command)
        );
        assert_eq!(header.message_header.message_length, Some(25));
    }
}
