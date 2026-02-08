use std::io::Read;

use anyhow::Result;

pub mod header;

use self::header::ChunkHeader;

#[derive(Debug)]
pub struct Chunk {
    pub header: ChunkHeader,
    pub payload: Box<[u8]>,
}

impl Chunk {
    pub fn read_from(stream: &mut impl Read) -> Result<Self> {
        let header = ChunkHeader::read_from(stream)?;
        let mut content =
            vec![0u8; header.message_header.message_length.unwrap_or_default() as usize];

        stream.read_exact(&mut content)?;

        Ok(Self {
            header,
            payload: content.into_boxed_slice(),
        })
    }
}
