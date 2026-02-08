use anyhow::{Result, ensure};

mod header;

use crate::message_type::MessageType;

pub use self::header::MessageHeader;

#[derive(Debug)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Box<[u8]>,
}

impl Message {
    pub fn new(
        message_type: MessageType,
        timestamp: u32,
        stream_id: u32,
        payload: &[u8],
    ) -> Result<Self> {
        let payload_length = payload.len();

        // <= u24::MAX
        ensure!(payload_length <= 16_777_215, "Payload is too big");
        let payload_length = payload_length as u32;

        Ok(Self {
            header: MessageHeader {
                message_type,
                payload_length,
                timestamp,
                stream_id,
            },
            payload: Box::from(payload),
        })
    }
}
