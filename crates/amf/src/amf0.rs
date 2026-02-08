pub mod constants;

mod number;
mod object;
mod string;
mod value;

pub use number::AmfNumber;
pub use object::AmfObject;
pub use string::AmfString;
pub use value::Value;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_read_string() {
        let data: &[u8] = &[0x02, 0x00, 0x04, 0x4d, 0x69, 0x6b, 0x65];

        let value = Value::deserialize(&mut data.iter().copied()).unwrap();

        assert_eq!(value.as_string().unwrap().as_str(), "Mike");
    }

    #[test]
    fn test_value_read_object() {
        let data: &[u8] = &[
            0x03, // Object
            0x00, 0x04, 0x6e, 0x61, 0x6d, 0x65, // Key "name"
            0x02, 0x00, 0x04, 0x4d, 0x69, 0x6b, 0x65, // String
            0x00, 0x03, 0x61, 0x67, 0x65, // Key "age"
            0x00, 0x40, 0x3e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Number
            0x00, 0x05, 0x61, 0x6c, 0x69, 0x61, 0x73, // Key "alias"
            0x02, 0x00, 0x04, 0x4d, 0x69, 0x6b, 0x65, // String
            0x00, 0x00, // Empty key
            0x09, // Object end
        ];

        let v = Value::deserialize(&mut data.iter().copied()).unwrap();

        println!("{v:#?}");
    }

    #[test]
    fn test_read_seq() {
        let data: &[u8] = &[
            0x02, 0x00, 0x0C, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x53, 0x74, 0x72, 0x65, 0x61,
            0x6D, // String
            0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Number
            0x05, // Null
        ];

        let mut iter = data.iter().copied();

        assert_eq!(
            Value::deserialize(&mut iter)
                .unwrap()
                .as_string()
                .unwrap()
                .as_str(),
            "createStream"
        );
        assert_eq!(
            Value::deserialize(&mut iter)
                .unwrap()
                .as_number()
                .unwrap()
                .to_float(),
            2.0
        );
        assert_eq!(Value::deserialize(&mut iter).unwrap(), Value::Null);
    }
}
