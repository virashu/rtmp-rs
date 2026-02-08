use std::io::BufRead;

mod header;

pub use self::header::MessageHeader;

#[derive(Debug)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Box<[u8]>,
}

impl Message {
    pub fn read_from(stream: &mut impl BufRead) -> anyhow::Result<Self> {
        let header = MessageHeader::read_from(stream)?;

        let mut payload = vec![0; header.payload_length as usize];
        stream.read_exact(&mut payload)?;

        Ok(Self {
            header,
            payload: payload.into_boxed_slice(),
        })
    }
}
