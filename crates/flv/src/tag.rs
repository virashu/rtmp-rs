use anyhow::{Result, ensure};

pub struct FlvTag {
    /// MessageTypeID
    tag_type: u8,
    /// byte order: 1 2 3 0
    timestamp: u32,
    data: Box<[u8]>,
}

impl FlvTag {
    /// Note:
    /// `timestamp` argument is regular, in order: 0, 1, 2, 3
    /// and transformed into flv-form later
    pub fn new(tag_type: u8, timestamp: u32, data: &[u8]) -> Result<Self> {
        ensure!(data.len() <= u32::MAX as usize, "Payload is too long");

        Ok(Self {
            tag_type,
            timestamp,
            data: Box::from(data),
        })
    }

    pub fn size(&self) -> usize {
        11 + self.data.len()
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut buf = Vec::new();

        // TagType
        buf.push(self.tag_type);
        // DataSize
        buf.extend(&(self.data.len() as u32).to_be_bytes()[1..4]);
        // Timestamp
        buf.extend(self.timestamp.rotate_left(8).to_be_bytes());
        // StreamID
        buf.extend([0, 0, 0]);
        // Data
        buf.extend(&self.data);

        buf.into_boxed_slice()
    }
}
