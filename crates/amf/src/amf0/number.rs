use anyhow::{Context, Result};
use itertools::Itertools;

#[derive(Clone, Debug, PartialEq)]
pub struct AmfNumber(f64);

impl AmfNumber {
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        let bytes = iter.next_array::<8>().context("Not enough items")?;
        Ok(Self(f64::from_be_bytes(bytes)))
    }

    pub fn serialize(&self) -> Box<[u8]> {
        Box::new(self.0.to_be_bytes())
    }

    pub fn to_float(&self) -> f64 {
        self.0
    }
}
