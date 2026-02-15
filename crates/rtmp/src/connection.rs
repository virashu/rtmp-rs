use std::io::{Read, Write};

use anyhow::{Context, Result, bail, ensure};
use tracing::trace;

use crate::{
    chunk::{
        ChunkStreamManager,
        header::{ChunkHeader, ChunkMessageHeader},
    },
    constants::DEFAULT_MAX_CHUNK_PAYLOAD_SIZE,
    message::Message,
};

pub struct ConnectionConfig {
    pub max_chunk_payload_size: u32,
}

pub struct Connection<'s, S: Read + Write> {
    pub config: ConnectionConfig,

    stream: &'s mut S,
    chunking_state: ChunkStreamManager,
}

impl<'s, R: Read + Write> Connection<'s, R> {
    pub fn new(stream: &'s mut R) -> Self {
        Self {
            config: ConnectionConfig {
                max_chunk_payload_size: DEFAULT_MAX_CHUNK_PAYLOAD_SIZE,
            },
            stream,
            chunking_state: ChunkStreamManager::new(),
        }
    }

    fn receive_one_chunk(&mut self) -> Result<Option<Message>> {
        let _span = tracing::info_span!("inbound::chunk").entered();
        let iter = &mut self.stream.bytes().filter_map(Result::ok);

        let chunk_header = ChunkHeader::deserialize(iter)?;
        let chunk_stream_id = chunk_header.chunk_stream_id;

        let chunk_metadata = self.chunking_state.get_mut(chunk_stream_id);

        let chunk_message_header = chunk_header.chunk_message_header;

        let message_type = chunk_message_header
            .message_type()
            .inspect(|value| {
                chunk_metadata.message_type = Some(*value);
            })
            .or(chunk_metadata.message_type)
            .context("No message type ID")?;

        let message_payload_length = chunk_message_header
            .message_length()
            .inspect(|value| {
                chunk_metadata.message_payload_length = Some(*value);
            })
            .or(chunk_metadata.message_payload_length)
            .context("No payload length")?;

        let message_timestamp = chunk_message_header
            .timestamp()
            .inspect(|value| {
                chunk_metadata.message_timestamp =
                    Some(*value + chunk_metadata.message_timestamp.unwrap_or(0));
            })
            .or(chunk_metadata.message_timestamp)
            .context("No timestamp")?;

        let message_stream_id = chunk_message_header
            .message_stream_id()
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

        trace!("Read {read_len} bytes. Metadata: {:?}", chunk_metadata);

        if chunk_metadata.buffer.len() < message_payload_length as usize {
            return Ok(None);
        }

        if chunk_metadata.buffer.len() > message_payload_length as usize {
            bail!("Read too much!");
        }

        // let message_header = MessageHeader {
        //     message_type,
        //     payload_length: message_payload_length,
        //     timestamp: message_timestamp,
        //     stream_id: message_stream_id,
        // };

        // Flush buffer
        let payload = chunk_metadata.buffer.split_off(0).into_boxed_slice();

        let message = Message::new(message_type, message_timestamp, message_stream_id, &payload)?;

        ensure!(message_payload_length == message.header().payload_length);

        Ok(Some(message))
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

    pub fn send_one_chunk(&mut self) -> Result<()> {
        todo!()
    }

    pub fn send(&mut self, chunk_stream_id: u32, message: Message) -> Result<()> {
        let _span = tracing::info_span!("outbound").entered();

        let message_header = message.header();

        let chunk_header = ChunkHeader {
            chunk_stream_id,
            chunk_message_header: ChunkMessageHeader::Type0 {
                timestamp: message_header.timestamp,
                message_length: message_header.payload_length,
                message_type: message_header.message_type,
                message_stream_id: message_header.stream_id,
            },
        };
        trace!(?chunk_header);

        self.send_raw(&chunk_header.serialize())?;

        if message_header.payload_length <= self.config.max_chunk_payload_size {
            self.send_raw(message.payload())?;
        } else {
            let (first, rest) = message
                .payload()
                .split_at(self.config.max_chunk_payload_size as usize);

            self.send_raw(first)?;

            let parts = rest.chunks(self.config.max_chunk_payload_size as usize);

            for part in parts {
                let chunk_header = ChunkHeader {
                    chunk_stream_id,
                    chunk_message_header: ChunkMessageHeader::Type3,
                };
                let header_raw = chunk_header.serialize();

                self.send_raw(&header_raw)?;
                self.send_raw(part)?;
            }
        }

        Ok(())
    }
}
