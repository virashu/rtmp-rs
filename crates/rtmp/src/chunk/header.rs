use std::convert::TryFrom;

use anyhow::{Context, Ok, Result};
use itertools::Itertools;

use crate::message_type::MessageType;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChunkMessageHeaderType {
    /// Full: `timestamp`, `message_length`, `message_type`, `message_stream_id`
    Type0,
    /// Without stream id: `timestamp`, `message_length`, `message_type`
    Type1,
    /// Only `timestamp`
    Type2,
    /// Empty
    Type3,
}

#[derive(Debug)]
pub enum ChunkMessageHeader {
    Type0 {
        timestamp: u32,
        message_length: u32,
        message_type: MessageType,
        message_stream_id: u32,
    },
    Type1 {
        timestamp: u32,
        message_length: u32,
        message_type: MessageType,
    },
    Type2 {
        timestamp: u32,
    },
    Type3,
}

impl ChunkMessageHeader {
    pub fn deserialize(
        iter: &mut impl Iterator<Item = u8>,
        header_type: ChunkMessageHeaderType,
    ) -> Result<Self> {
        if header_type == ChunkMessageHeaderType::Type3 {
            return Ok(Self::Type3);
        }

        // Timestamp
        let raw = iter.next_array::<3>().context("Not enough items")?;
        let timestamp = u32::from_be_bytes([0, raw[0], raw[1], raw[2]]);

        if header_type == ChunkMessageHeaderType::Type2 {
            return Ok(Self::Type2 { timestamp });
        }

        // Message length
        let raw = iter.next_array::<3>().context("Not enough items")?;
        let message_length = u32::from_be_bytes([0, raw[0], raw[1], raw[2]]);

        // Message type
        let raw = iter.next().context("Not enough items")?;
        let message_type =
            MessageType::try_from(raw).context("Failed to deserialize message type ID")?;

        if header_type == ChunkMessageHeaderType::Type1 {
            return Ok(Self::Type1 {
                timestamp,
                message_length,
                message_type,
            });
        }

        // Message stream ID
        let raw = iter.next_array::<4>().context("Not enough items")?;
        let message_stream_id = u32::from_le_bytes(raw);

        Ok(Self::Type0 {
            timestamp,
            message_length,
            message_type,
            message_stream_id,
        })
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut buf = Vec::new();

        if let Self::Type0 { timestamp, .. }
        | Self::Type1 { timestamp, .. }
        | Self::Type2 { timestamp } = self
        {
            let bytes = timestamp.to_be_bytes();
            buf.extend(&bytes[1..4]);
        }

        if let Self::Type0 { message_length, .. } | Self::Type1 { message_length, .. } = self {
            let bytes = message_length.to_be_bytes();
            buf.extend(&bytes[1..4]);
        }

        if let Self::Type0 { message_type, .. } | Self::Type1 { message_type, .. } = self {
            buf.push(u8::from(*message_type));
        }

        if let Self::Type0 {
            message_stream_id, ..
        } = self
        {
            buf.extend(&message_stream_id.to_le_bytes());
        }

        buf.into_boxed_slice()
    }

    pub fn get_type(&self) -> ChunkMessageHeaderType {
        match self {
            ChunkMessageHeader::Type0 { .. } => ChunkMessageHeaderType::Type0,
            ChunkMessageHeader::Type1 { .. } => ChunkMessageHeaderType::Type1,
            ChunkMessageHeader::Type2 { .. } => ChunkMessageHeaderType::Type2,
            ChunkMessageHeader::Type3 => ChunkMessageHeaderType::Type3,
        }
    }

    pub fn timestamp(&self) -> Option<u32> {
        match self {
            Self::Type0 { timestamp, .. }
            | Self::Type1 { timestamp, .. }
            | Self::Type2 { timestamp } => Some(*timestamp),
            _ => None,
        }
    }

    pub fn message_length(&self) -> Option<u32> {
        match self {
            Self::Type0 { message_length, .. } | Self::Type1 { message_length, .. } => {
                Some(*message_length)
            }
            _ => None,
        }
    }

    pub fn message_type(&self) -> Option<MessageType> {
        match self {
            Self::Type0 { message_type, .. } | Self::Type1 { message_type, .. } => {
                Some(*message_type)
            }
            _ => None,
        }
    }

    pub fn message_stream_id(&self) -> Option<u32> {
        match self {
            Self::Type0 {
                message_stream_id, ..
            } => Some(*message_stream_id),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ChunkHeader {
    pub chunk_stream_id: u32,
    pub chunk_message_header: ChunkMessageHeader,
}

impl ChunkHeader {
    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        let octet_0 = iter.next().context("Not enough items")?;

        let header_type = match (octet_0 & 0b1100_0000) >> 6 {
            0b00 => ChunkMessageHeaderType::Type0,
            0b01 => ChunkMessageHeaderType::Type1,
            0b10 => ChunkMessageHeaderType::Type2,
            0b11 => ChunkMessageHeaderType::Type3,
            _ => unreachable!(),
        };

        let chunk_stream_id = match octet_0 & 0b0011_1111 {
            // 2-Byte ID
            0 => {
                let raw = iter.next().context("Not enough items")?;
                u32::from(raw) + 64
            }
            // 3-Byte ID
            1 => {
                let raw = iter.next_array::<2>().context("Not enough items")?;
                u32::from(u16::from_be_bytes(raw)) + 64
            }
            // 1-Byte ID
            x => u32::from(x),
        };

        let chunk_message_header = ChunkMessageHeader::deserialize(iter, header_type)?;

        Ok(Self {
            chunk_stream_id,
            chunk_message_header,
        })
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut buf = Vec::new();

        // TODO: Support longer formats

        if self.chunk_stream_id > 63 {
            todo!("Chunk stream ID is to big to be stored in 6 bits")
        }

        let chunk_header_type_bits = match self.chunk_message_header.get_type() {
            ChunkMessageHeaderType::Type0 => 0b00,
            ChunkMessageHeaderType::Type1 => 0b01,
            ChunkMessageHeaderType::Type2 => 0b10,
            ChunkMessageHeaderType::Type3 => 0b11,
        };

        buf.push((chunk_header_type_bits << 6) | (self.chunk_stream_id as u8));

        buf.extend(&self.chunk_message_header.serialize());

        buf.into_boxed_slice()
    }
}
