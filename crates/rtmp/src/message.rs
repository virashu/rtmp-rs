use anyhow::{Result, ensure};

mod header;

use crate::message_type::MessageType;

pub use self::header::MessageHeader;

#[derive(Debug)]
pub struct Message {
    header: MessageHeader,
    payload: Box<[u8]>,
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

    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

pub mod control_message {
    use crate::{constants::CONTROL_MESSAGE_STREAM_ID, event::UserControlMessageEvent};

    use super::*;

    #[allow(clippy::unwrap_used, reason = "checked input")]
    pub fn window_acknowledgement_size(value: u32) -> Message {
        Message::new(
            MessageType::WindowAcknowledgementSize,
            0, // Ignored
            CONTROL_MESSAGE_STREAM_ID,
            &value.to_be_bytes(),
        )
        .unwrap()
    }

    #[allow(clippy::unwrap_used, reason = "checked input")]
    pub fn set_peer_bandwidth(value: u32, priority: u8) -> Message {
        let mut bytes = [0u8; 5];
        bytes[0..4].copy_from_slice(&value.to_be_bytes());
        bytes[4] = priority;

        Message::new(
            MessageType::SetPeerBandwidth,
            0, // Ignored
            CONTROL_MESSAGE_STREAM_ID,
            &bytes,
        )
        .unwrap()
    }

    pub fn user_control_message(
        event: UserControlMessageEvent,
        event_data: &[u8],
    ) -> Result<Message> {
        let mut bytes = vec![];
        bytes.extend((event as u16).to_be_bytes());
        bytes.extend(event_data);

        Message::new(
            MessageType::UserControlMessage,
            0, // Ignored
            CONTROL_MESSAGE_STREAM_ID,
            &bytes,
        )
    }
}
