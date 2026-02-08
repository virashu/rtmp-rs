use std::{convert::TryFrom, io::BufRead};

use crate::{message_type::MessageType, util::raw_u32_be};

#[derive(Debug)]
pub struct MessageHeader {
    pub message_type: MessageType,
    pub payload_length: u32,
    pub timestamp: u32,
    pub stream_id: u32,
}

impl MessageHeader {
    pub fn read_from(stream: &mut impl BufRead) -> anyhow::Result<Self> {
        let mut buf: [u8; _] = [0; 4];

        // Message type
        stream.read_exact(&mut buf[..1])?;
        let message_type_id = buf[0];
        let message_type = MessageType::try_from(message_type_id)?;

        // Payload length
        stream.read_exact(&mut buf[..3])?;
        let payload_length = raw_u32_be(&buf[..3]);

        // Timestamp
        stream.read_exact(&mut buf[..4])?;
        let timestamp = raw_u32_be(&buf[..4]);

        // Stream ID
        stream.read_exact(&mut buf[..3])?;
        let stream_id = raw_u32_be(&buf[..3]);

        Ok(Self {
            message_type,
            payload_length,
            timestamp,
            stream_id,
        })
    }
}
