use std::collections::HashMap;

use anyhow::{Context, Result};
use itertools::Itertools;

use crate::amf0::{AmfString, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct AmfEcmaArray(HashMap<AmfString, Value>);

impl AmfEcmaArray {
    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        // Skip elements count (32bits) for now
        iter.next_array::<4>().context("Not enough items")?;

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
}
