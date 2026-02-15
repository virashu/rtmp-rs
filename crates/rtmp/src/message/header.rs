use crate::message_type::MessageType;

#[derive(Debug)]
pub struct MessageHeader {
    pub message_type: MessageType,
    pub payload_length: u32,
    pub timestamp: u32,
    pub stream_id: u32,
}

impl MessageHeader {}
