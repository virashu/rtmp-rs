pub mod read;
pub mod types;
mod value;

pub use value::Value;

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::amf0::read::read_string;

    use super::*;

    #[test]
    fn test_read_string() {
        let mut data: &[u8] = &[0x00, 0x04, 0x4d, 0x69, 0x6b, 0x65];

        let mut reader = BufReader::new(&mut data);
        let v = read_string(&mut reader).unwrap();

        assert_eq!(v, String::from("Mike"));
    }

    #[test]
    fn test_value_read_string() {
        let mut data: &[u8] = &[0x02, 0x00, 0x04, 0x4d, 0x69, 0x6b, 0x65];

        let mut reader = BufReader::new(&mut data);
        let v = Value::read(&mut reader).unwrap();

        assert_eq!(v, Value::String(String::from("Mike")));
    }

    #[test]
    fn test_value_read_object() {
        let mut data: &[u8] = &[
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

        let mut reader = BufReader::new(&mut data);
        let v = Value::read(&mut reader).unwrap();

        println!("{v:#?}");
    }

    #[test]
    fn test_read_seq() {
        let mut data: &[u8] = &[
            0x02, 0x00, 0x0C, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x53, 0x74, 0x72, 0x65, 0x61,
            0x6D, // String
            0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Number
            0x05, // Null
        ];

        let mut reader = BufReader::new(&mut data);

        assert_eq!(
            Value::read(&mut reader).unwrap(),
            Value::String(String::from("createStream"))
        );
        assert_eq!(Value::read(&mut reader).unwrap(), Value::Number(2.0));
        assert_eq!(Value::read(&mut reader).unwrap(), Value::Null);
    }
}
