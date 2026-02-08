use anyhow::{Context, Result, ensure};

use itertools::Itertools;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AmfString(Box<str>);

impl AmfString {
    pub fn new(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        ensure!(value.len() <= (u16::MAX as usize), "Value is too long");

        Ok(Self(Box::from(value)))
    }

    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        let len_bytes = iter.next_array::<2>().context("Not enough items")?;
        let len = u16::from_be_bytes(len_bytes) as usize;

        let string_bytes = iter.take(len).collect::<Vec<_>>();
        ensure!(string_bytes.len() == len);

        let string = String::from_utf8(string_bytes)?;

        Ok(Self(string.into_boxed_str()))
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut buf = Vec::new();

        buf.extend((self.0.len() as u16).to_be_bytes());
        buf.extend(self.0.as_bytes());

        buf.into_boxed_slice()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::fmt::Display for AmfString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for AmfString {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for AmfString {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}
