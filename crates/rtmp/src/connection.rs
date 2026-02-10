use std::{
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::{Context, Result, bail};
use tracing::trace;

use crate::{
    chunk::{
        ChunkStreamManager,
        header::{ChunkBasicHeader, ChunkHeader, ChunkHeaderType, ChunkMessageHeader},
    },
    constants::DEFAULT_MAX_CHUNK_PAYLOAD_SIZE,
    message::{Message, MessageHeader},
};

pub struct ConnectionConfig {
    pub max_chunk_payload_size: u32,
}

pub struct Connection<'s> {
    pub config: ConnectionConfig,

    stream: &'s mut TcpStream,
    chunking_state: ChunkStreamManager,
}

impl<'s> Connection<'s> {
    pub fn new(stream: &'s mut TcpStream) -> Self {
        Self {
            config: ConnectionConfig {
                max_chunk_payload_size: DEFAULT_MAX_CHUNK_PAYLOAD_SIZE,
            },
            stream,
            chunking_state: ChunkStreamManager::new(),
        }
    }

    fn receive_one_chunk(&mut self) -> Result<Option<Message>> {
        let chunk_header = ChunkHeader::read_from(self.stream)?;
        let chunk_stream_id = chunk_header.basic_header.chunk_stream_id;

        let chunk_metadata = self.chunking_state.get_mut(chunk_stream_id);

        let chunk_message_header = chunk_header.message_header;

        let message_type = chunk_message_header
            .message_type
            .inspect(|value| {
                chunk_metadata.message_type = Some(*value);
            })
            .or(chunk_metadata.message_type)
            .context("No message type ID")?;

        let message_payload_length = chunk_message_header
            .message_length
            .inspect(|value| {
                chunk_metadata.message_payload_length = Some(*value);
            })
            .or(chunk_metadata.message_payload_length)
            .context("No payload length")?;

        let message_timestamp = chunk_message_header
            .timestamp
            .inspect(|value| {
                chunk_metadata.message_timestamp =
                    Some(*value + chunk_metadata.message_timestamp.unwrap_or(0));
            })
            .or(chunk_metadata.message_timestamp)
            .context("No timestamp")?;

        let message_stream_id = chunk_message_header
            .message_stream_id
            .inspect(|value| {
                chunk_metadata.message_stream_id = Some(*value);
            })
            .or(chunk_metadata.message_stream_id)
            .context("No message stream ID")?;

        let read_len = ((message_payload_length as usize) - chunk_metadata.buffer.len())
            .min(self.config.max_chunk_payload_size as usize);

        let mut payload_fragment = vec![0; read_len];
        self.stream.read_exact(&mut payload_fragment)?;

        chunk_metadata.buffer.extend(payload_fragment);

        trace!("Read {read_len} bytes");
        trace!(?chunk_metadata);

        if chunk_metadata.buffer.len() < message_payload_length as usize {
            return Ok(None);
        }

        if chunk_metadata.buffer.len() > message_payload_length as usize {
            bail!("Read too much!");
        }

        let message_header = MessageHeader {
            message_type,
            payload_length: message_payload_length,
            timestamp: message_timestamp,
            stream_id: message_stream_id,
        };

        let payload = chunk_metadata.buffer.split_off(0).into_boxed_slice();

        Ok(Some(Message {
            header: message_header,
            payload,
        }))
    }

    pub fn recv(&mut self) -> Result<Message> {
        loop {
            if let Some(message) = self.receive_one_chunk()? {
                return Ok(message);
            }
        }
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
