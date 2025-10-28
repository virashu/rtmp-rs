pub fn raw_u32_be(raw: &[u8]) -> u32 {
    raw.iter()
        .rev()
        .enumerate()
        .map(|(i, n)| u32::from(*n) << (i * 8))
        .sum()
}

pub fn raw_u32_le(raw: &[u8]) -> u32 {
    raw.iter()
        .enumerate()
        .map(|(i, n)| u32::from(*n) << (i * 8))
        .sum()
}
