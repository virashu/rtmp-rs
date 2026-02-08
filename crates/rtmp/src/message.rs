mod header;

pub use self::header::MessageHeader;

#[derive(Debug)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Box<[u8]>,
}
