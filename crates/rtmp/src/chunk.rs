use std::{collections::BTreeMap, io::Read};

use anyhow::{Context, Result};

pub mod header;

use crate::{message::MessageHeader, message_type::MessageType};

use self::header::ChunkHeader;

pub struct ChunkingState {
    pub message_types: BTreeMap<u32, MessageType>,
    pub payload_lengths: BTreeMap<u32, u32>,
    pub timestamps: BTreeMap<u32, u32>,
    pub stream_ids: BTreeMap<u32, u32>,
}

pub fn make_message_header(
    state: &mut ChunkingState,
    header: ChunkHeader,
) -> Result<MessageHeader> {
    let chunk_stream_id = header.basic_header.chunk_stream_id;

    // TODO: Add timestamp to the past one, if new is present in
    // some header type (?)

    let message_type = header
        .message_header
        .message_type
        .inspect(|value| {
            state.message_types.insert(chunk_stream_id, *value);
        })
        .or_else(|| state.message_types.get(&chunk_stream_id).copied())
        .context("No message type ID")?;

    let payload_length = header
        .message_header
        .message_length
        .inspect(|value| {
            state.payload_lengths.insert(chunk_stream_id, *value);
        })
        .or_else(|| state.payload_lengths.get(&chunk_stream_id).copied())
        .context("No payload length")?;

    let timestamp = header
        .message_header
        .timestamp
        .inspect(|value| {
            state.timestamps.insert(chunk_stream_id, *value);
        })
        .or_else(|| state.timestamps.get(&chunk_stream_id).copied())
        .context("No timestamp")?;

    let stream_id = header
        .message_header
        .message_stream_id
        .inspect(|value| {
            state.stream_ids.insert(chunk_stream_id, *value);
        })
        .or_else(|| state.stream_ids.get(&chunk_stream_id).copied())
        .context("No message stream ID")?;

    Ok(MessageHeader {
        message_type,
        payload_length,
        timestamp,
        stream_id,
    })
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
