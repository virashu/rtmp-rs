use anyhow::{Context, Result, anyhow};

use crate::amf0::{
    AmfEcmaArray, AmfObject, constants::types, number::AmfNumber, string::AmfString,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(AmfNumber),
    Boolean(bool),
    String(AmfString),
    Object(AmfObject),
    Null,
    EcmaArray(AmfEcmaArray),
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
        let value_type = iter.next().context("Not enough items")?;

        match value_type {
            types::NUMBER => AmfNumber::deserialize(iter).map(Self::Number),
            types::BOOLEAN => {
                let value = iter.next().context("Not enough items")?;
                match value {
                    0x00 => Ok(Self::Boolean(false)),
                    0x01 => Ok(Self::Boolean(true)),
                    _ => Err(anyhow!("Invalid boolean value")),
                }
            }
            types::STRING => AmfString::deserialize(iter).map(Self::String),
            types::OBJECT => AmfObject::deserialize(iter).map(Self::Object),
            types::NULL => Ok(Self::Null),
            types::ECMA_ARRAY => AmfEcmaArray::deserialize(iter).map(Self::EcmaArray),
            types::OBJECT_END => Ok(Self::ObjectEnd),

            t => Err(anyhow!("AMF Value Deserialize Error: Unhandled type: {t}")),
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

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(AmfNumber::new(value))
    }
}

impl From<AmfNumber> for Value {
    fn from(value: AmfNumber) -> Self {
        Self::Number(value)
    }
}
