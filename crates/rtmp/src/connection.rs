use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::Result;

use crate::{
    chunk::{
        ChunkingState,
        header::{ChunkBasicHeader, ChunkHeader, ChunkHeaderType, ChunkMessageHeader},
        make_message_header,
    },
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

    pub fn send(&mut self, chunk_stream_id: u32, message: Message) -> Result<()> {
        // TODO: Optimize
        // Sending full header for now
        let chunk_header = ChunkHeader {
            basic_header: ChunkBasicHeader {
                chunk_header_type: ChunkHeaderType::Type0,
                chunk_stream_id,
            },
            message_header: ChunkMessageHeader {
                timestamp: Some(message.header.timestamp),
                message_length: Some(message.header.payload_length),
                message_type: Some(message.header.message_type),
                message_stream_id: Some(message.header.stream_id),
            },
        };

        self.send_raw(&chunk_header.serialize())?;
        self.send_raw(&message.payload)?;

        Ok(())
    }
}
