use anyhow::{Context, Result, anyhow};

use crate::amf0::{AmfObject, constants::types, number::AmfNumber, string::AmfString};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(AmfNumber),
    Boolean(bool),
    String(AmfString),
    Object(AmfObject),
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
    /// Detect type and deserialize value
    pub fn deserialize(iter: &mut impl Iterator<Item = u8>) -> Result<Self> {
        let value_type = iter.next().context("")?;

        match value_type {
            types::NUMBER => AmfNumber::deserialize(iter).map(Self::Number),
            types::STRING => AmfString::deserialize(iter).map(Self::String),
            types::OBJECT => AmfObject::deserialize(iter).map(Self::Object),
            types::NULL => Ok(Self::Null),
            types::OBJECT_END => Ok(Self::ObjectEnd),

            t => Err(anyhow::anyhow!("Unhandled type: {t}")),
        }
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let mut buf = Vec::new();

        match self {
            Self::Number(value) => {
                buf.push(types::NUMBER);
                buf.extend(value.serialize());
            }
            Self::String(value) => {
                buf.push(types::STRING);
                buf.extend(value.serialize());
            }
            Self::Object(value) => {
                buf.push(types::OBJECT);
                buf.extend(value.serialize());
            }
            Self::Null => {
                buf.push(types::NULL);
            }

            _ => todo!(),
        }

        buf.into_boxed_slice()
    }

    //
    // Conversions
    //

    pub fn as_string(&self) -> Result<AmfString> {
        match self {
            Self::String(value) => Ok(value.clone()),
            _ => Err(anyhow!("Type mismatch")),
        }
    }

    pub fn as_number(&self) -> Result<AmfNumber> {
        match self {
            Self::Number(value) => Ok(value.clone()),
            _ => Err(anyhow!("Type mismatch")),
        }
    }

    pub fn as_object(&self) -> Result<AmfObject> {
        match self {
            Self::Object(value) => Ok(value.clone()),
            _ => Err(anyhow!("Type mismatch")),
        }
    }
}

// From

impl TryFrom<&str> for Value {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self::String(AmfString::try_from(value)?))
    }
}

impl TryFrom<String> for Value {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self::String(AmfString::try_from(value)?))
    }
}

impl From<AmfString> for Value {
    fn from(value: AmfString) -> Self {
        Self::String(value)
    }
}
