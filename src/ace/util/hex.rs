//! # ACE-Hex (Hexadecimal Codec)
//!
//! Implementação nativa de codificação/decodificação hexadecimal para a engine ACE.

const CHAR_TABLE_LOWER: &[u8; 16] = b"0123456789abcdef";
const CHAR_TABLE_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Codifica dados em uma string hexadecimal (lowercase).
pub fn encode(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len() * 2);
    for &byte in data {
        result.push(CHAR_TABLE_LOWER[(byte >> 4) as usize] as char);
        result.push(CHAR_TABLE_LOWER[(byte & 0x0F) as usize] as char);
    }
    result
}

/// Codifica dados em uma string hexadecimal (uppercase).
pub fn encode_upper(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len() * 2);
    for &byte in data {
        result.push(CHAR_TABLE_UPPER[(byte >> 4) as usize] as char);
        result.push(CHAR_TABLE_UPPER[(byte & 0x0F) as usize] as char);
    }
    result
}

/// Decodifica uma string hexadecimal em um vetor de bytes.
pub fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
    if input.len() % 2 != 0 {
        return Err("Hex string must have an even length");
    }

    let mut result = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();

    for i in (0..bytes.len()).step_by(2) {
        let h = decode_nibble(bytes[i])?;
        let l = decode_nibble(bytes[i + 1])?;
        result.push((h << 4) | l);
    }

    Ok(result)
}

fn decode_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("Invalid hexadecimal character"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(&[0x01, 0x02, 0x0F, 0xFF]), "01020fff");
        assert_eq!(encode_upper(&[0x01, 0x02, 0x0F, 0xFF]), "01020FFF");
    }

    #[test]
    fn test_decode() {
        assert_eq!(decode("").unwrap(), b"");
        assert_eq!(decode("01020fff").unwrap(), &[0x01, 0x02, 0x0F, 0xFF]);
        assert_eq!(decode("01020FFF").unwrap(), &[0x01, 0x02, 0x0F, 0xFF]);
    }

    #[test]
    fn test_invalid_decode() {
        assert!(decode("010").is_err()); // impar
        assert!(decode("010g").is_err()); // invalido
    }

    #[test]
    fn test_roundtrip_hex() {
        let cases = ["Albedo", "Rustlang", "1234567890"];
        for case in cases {
            let encoded = encode(case.as_bytes());
            let decoded = decode(&encoded).unwrap();
            assert_eq!(case.as_bytes(), decoded.as_slice());
        }
    }
}
