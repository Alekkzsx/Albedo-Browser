//! # ACE-Base64 (RFC 4648)
//!
//! Implementação nativa de codificação/decodificação Base64 para a engine ACE.
//! Suporta os alfabetos Standard e URL-Safe com tratamento rigoroso de padding.

const STANDARD_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Opções de configuração para codificação Base64.
#[derive(Debug, Clone, Copy)]
pub enum Base64Config {
    Standard,
    UrlSafe,
}

/// Codifica dados em uma string Base64 usando o alfabeto padrão.
pub fn encode(data: &[u8]) -> String {
    encode_config(data, Base64Config::Standard)
}

/// Codifica dados em uma string Base64 usando o alfabeto URL-Safe.
pub fn encode_url_safe(data: &[u8]) -> String {
    encode_config(data, Base64Config::UrlSafe)
}

fn encode_config(data: &[u8], config: Base64Config) -> String {
    let alphabet = match config {
        Base64Config::Standard => STANDARD_ALPHABET,
        Base64Config::UrlSafe => URL_SAFE_ALPHABET,
    };

    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let chunks = data.chunks_exact(3);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let n = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | (chunk[2] as u32);
        result.push(alphabet[((n >> 18) & 0x3F) as usize] as char);
        result.push(alphabet[((n >> 12) & 0x3F) as usize] as char);
        result.push(alphabet[((n >> 6) & 0x3F) as usize] as char);
        result.push(alphabet[(n & 0x3F) as usize] as char);
    }

    match remainder.len() {
        1 => {
            let n = (remainder[0] as u32) << 16;
            result.push(alphabet[((n >> 18) & 0x3F) as usize] as char);
            result.push(alphabet[((n >> 12) & 0x3F) as usize] as char);
            result.push('=');
            result.push('=');
        }
        2 => {
            let n = (remainder[0] as u32) << 16 | (remainder[1] as u32) << 8;
            result.push(alphabet[((n >> 18) & 0x3F) as usize] as char);
            result.push(alphabet[((n >> 12) & 0x3F) as usize] as char);
            result.push(alphabet[((n >> 6) & 0x3F) as usize] as char);
            result.push('=');
        }
        _ => {}
    }

    result
}

/// Decodifica uma string Base64 (Standard ou URL-Safe) em um vetor de bytes.
pub fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let input = input.trim_end_matches('=');
    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits_accumulator = 0u8;

    for c in input.chars() {
        if c.is_whitespace() {
            continue;
        }

        let val = match c {
            'A'..='Z' => c as u8 - b'A',
            'a'..='z' => c as u8 - b'a' + 26,
            '0'..='9' => c as u8 - b'0' + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            _ => return Err("Invalid character in Base64 input"),
        };

        buffer = (buffer << 6) | (val as u32);
        bits_accumulator += 6;

        if bits_accumulator >= 8 {
            bits_accumulator -= 8;
            result.push(((buffer >> bits_accumulator) & 0xFF) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_encode_url_safe() {
        // Alfabeto Standard: + / -> URL-Safe: - _
        // Exemplo: binary [0xFB, 0xFF, 0xBF] -> standard "+/+/ " -> urlsafe "-_-_"
        let data = [251, 255, 191];
        assert_eq!(encode(&data), "+/+/");
        assert_eq!(encode_url_safe(&data), "-_-_");
    }

    #[test]
    fn test_decode() {
        assert_eq!(decode("").unwrap(), b"");
        assert_eq!(decode("Zg==").unwrap(), b"f");
        assert_eq!(decode("Zm8=").unwrap(), b"fo");
        assert_eq!(decode("Zm9v").unwrap(), b"foo");
        assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn test_roundtrip() {
        let cases = [
            "Albedo Browser",
            "Rust lang 🦀",
            "Base64 is cool!",
            "1234567890",
            "!@#$%^&*()_+",
        ];
        for case in cases {
            let encoded = encode(case.as_bytes());
            let decoded = decode(&encoded).unwrap();
            assert_eq!(case.as_bytes(), decoded.as_slice());
        }
    }

    #[test]
    fn test_invalid_decode() {
        assert!(decode("Zg?=").is_err());
    }
}
