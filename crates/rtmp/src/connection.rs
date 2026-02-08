use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::Result;

use crate::{
    chunk::{ChunkingState, header::ChunkHeader, make_message_header},
    message::Message,
};

pub struct Connection<'s> {
    stream: &'s mut TcpStream,
    chunking_state: ChunkingState,
}

impl<'s> Connection<'s> {
    pub fn new(stream: &'s mut TcpStream) -> Self {
        Self {
            stream,
            chunking_state: ChunkingState {
                message_types: BTreeMap::new(),
                payload_lengths: BTreeMap::new(),
                timestamps: BTreeMap::new(),
                stream_ids: BTreeMap::new(),
            },
        }
    }

    pub fn recv(&mut self) -> Result<Message> {
        let chunk_header = ChunkHeader::read_from(self.stream)?;
        let message_header = make_message_header(&mut self.chunking_state, chunk_header)?;

        let mut payload = vec![0; message_header.payload_length as usize];
        self.stream.read_exact(&mut payload)?;

        Ok(Message {
            header: message_header,
            payload: payload.into_boxed_slice(),
        })
    }

    pub fn send_raw(&mut self, buf: &[u8]) -> Result<()> {
        Ok(self.stream.write_all(buf)?)
    }
}
