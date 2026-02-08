use anyhow::Result;

use std::collections::HashMap;

use crate::amf0::{Value, constants::types, string::AmfString};

#[derive(Clone, Debug, PartialEq)]
pub struct AmfObject(HashMap<AmfString, Value>);

impl AmfObject {
    pub fn new(value: impl Into<HashMap<String, Value>>) -> Result<Self> {
        let value = value.into();
        let value = value
            .into_iter()
            .map(|(k, v)| -> Result<_> { Ok((AmfString::new(k)?, v)) })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self(value))
    }

    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        let mut items = HashMap::new();

        loop {
            let k = AmfString::deserialize(iter)?;
            let v = Value::deserialize(iter)?;

            if matches!(v, Value::ObjectEnd) {
                break;
            }

            items.insert(k, v);
        }

        Ok(Self(items))
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut buf = Vec::new();

        for (k, v) in &self.0 {
            buf.extend(k.serialize());
            buf.extend(v.serialize());
        }

        buf.push(types::OBJECT_END);

        buf.into_boxed_slice()
    }

    pub fn to_hashmap(&self) -> HashMap<String, Value> {
        self.0
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }
}
