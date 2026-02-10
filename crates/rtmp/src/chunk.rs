use std::{
    collections::{BTreeMap, btree_map::Entry},
    io::Read,
};

use anyhow::Result;

pub mod header;

use crate::message_type::MessageType;

use self::header::ChunkHeader;

#[derive(Debug)]
pub struct ChunkStream {
    pub message_type: Option<MessageType>,
    pub message_payload_length: Option<u32>,
    pub message_timestamp: Option<u32>,
    pub message_stream_id: Option<u32>,

    pub buffer: Vec<u8>,
}

impl ChunkStream {
    pub fn new() -> Self {
        Self {
            message_type: None,
            message_payload_length: None,
            message_timestamp: None,
            message_stream_id: None,

            buffer: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ChunkStreamManager {
    streams: BTreeMap<u32, ChunkStream>,
}

impl ChunkStreamManager {
    pub fn new() -> Self {
        Self {
            streams: BTreeMap::new(),
        }
    }

    pub fn get_mut(&mut self, id: u32) -> &mut ChunkStream {
        match self.streams.entry(id) {
            Entry::Vacant(e) => e.insert(ChunkStream::new()),
            Entry::Occupied(e) => e.into_mut(),
        }
    }
}

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
