use anyhow::Result;

use crate::amf0::Value;

#[derive(Debug)]
pub struct Sequence {
    inner: Vec<Value>,
}

impl Sequence {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn from(items: &[Value]) -> Self {
        Self {
            inner: Vec::from(items),
        }
    }

    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        let mut iter = iter.peekable();
        let mut items = Vec::new();

        while iter.peek().is_some() {
            let item = Value::deserialize(&mut iter)?;
            items.push(item);
        }

        Ok(Self { inner: items })
    }

    pub fn push(&mut self, item: Value) {
        self.inner.push(item);
    }

    pub fn serialize(&self) -> Box<[u8]> {
        self.inner
            .iter()
            .flat_map(|item| item.serialize())
            .collect()
    }
}
