use std::{collections::HashMap, io::BufRead};

use crate::amf0::Value;

pub fn read_number(stream: &mut impl BufRead) -> anyhow::Result<f64> {
    let mut buf: [u8; _] = [0; 8];

    // Content (64 Float)
    stream.read_exact(&mut buf)?;

    Ok(f64::from_be_bytes(buf))
}

pub fn read_string(stream: &mut impl BufRead) -> anyhow::Result<String> {
    // Length
    let mut buf: [u8; _] = [0; 2];
    stream.read_exact(&mut buf)?;
    let len = u16::from_be_bytes(buf) as usize;

    // Content (UTF-8)
    let mut string_buf = vec![0; len];
    stream.read_exact(&mut string_buf[..len])?;
    Ok(String::from_utf8(string_buf)?)
}

pub fn read_object(stream: &mut impl BufRead) -> anyhow::Result<HashMap<String, Value>> {
    let mut res = HashMap::new();

    loop {
        let k = read_string(stream)?;
        let v = Value::read(stream)?;

        if matches!(v, Value::ObjectEnd) {
            break;
        }

        res.insert(k, v);
    }

    Ok(res)
}
