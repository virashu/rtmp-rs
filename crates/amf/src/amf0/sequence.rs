use crate::amf0::Value;

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
