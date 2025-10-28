use std::{collections::HashMap, io::BufRead};

use crate::amf0::{
    read::{read_number, read_object, read_string},
    types,
};

#[derive(Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    String(String),
    Object(HashMap<String, Value>),
    Null,
    EcmaArray,
    ObjectEnd,
    StrictArray,
    Date,
    LongString,
    Xml,
    TypedObject,
    Upgrade,
}

impl Value {
    pub fn read(stream: &mut impl BufRead) -> anyhow::Result<Self> {
        let mut type_buf: [u8; 1] = [0];
        stream.read_exact(&mut type_buf[..1])?;

        match type_buf[0] {
            types::NUMBER => read_number(stream).map(Self::Number),
            types::STRING => read_string(stream).map(Self::String),
            types::OBJECT => read_object(stream).map(Self::Object),
            types::NULL => Ok(Self::Null),
            types::OBJECT_END => Ok(Self::ObjectEnd),

            t => Err(anyhow::anyhow!("Unhandled type: {t}")),
        }
    }
}
