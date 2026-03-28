//! # ACE-Base64 (RFC 4648)
//!
//! Implementação nativa de codificação/decodificação Base64 para a engine ACE.
//! Suporta os alfabetos Standard e URL-Safe com tratamento rigoroso de padding.

const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

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

/// Decodifica Base64 padrão (RFC 4648, alfabeto `+/`).
pub fn decode_standard(input: &str) -> Result<Vec<u8>, &'static str> {
    decode_config(input, Base64Config::Standard)
}

/// Decodifica Base64 URL-safe (RFC 4648, alfabeto `-_`).
pub fn decode_url_safe(input: &str) -> Result<Vec<u8>, &'static str> {
    decode_config(input, Base64Config::UrlSafe)
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
    // Auto-detect simples de variante para manter compatibilidade da API antiga.
    if input.contains('-') || input.contains('_') {
        decode_url_safe(input)
    } else {
        decode_standard(input)
    }
}

fn decode_config(input: &str, config: Base64Config) -> Result<Vec<u8>, &'static str> {
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return Ok(Vec::new());
    }

    if compact.len() % 4 != 0 {
        return Err("Invalid Base64 length");
    }

    let bytes = compact.as_bytes();
    let mut pad_count = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'=' {
            pad_count += 1;
            if i < bytes.len() - 2 {
                return Err("Invalid Base64 padding position");
            }
        } else if pad_count > 0 {
            return Err("Invalid Base64 padding sequence");
        }
    }
    if pad_count > 2 {
        return Err("Invalid Base64 padding");
    }

    let mut result = Vec::with_capacity((bytes.len() / 4) * 3);
    for chunk in bytes.chunks_exact(4) {
        let mut vals = [0u8; 4];
        let mut chunk_pad = 0usize;

        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                vals[i] = 0;
                chunk_pad += 1;
                continue;
            }
            vals[i] = decode_char(c, config)?;
        }

        if chunk_pad > 0 {
            // Apenas o último chunk pode conter '='
            if chunk.as_ptr() != bytes[bytes.len() - 4..].as_ptr() {
                return Err("Invalid Base64 padding position");
            }
            // '=' só pode aparecer no final do chunk: xx== ou xxx=
            if chunk[2] == b'=' && chunk[3] != b'=' {
                return Err("Invalid Base64 padding sequence");
            }
        }

        let n = ((vals[0] as u32) << 18)
            | ((vals[1] as u32) << 12)
            | ((vals[2] as u32) << 6)
            | (vals[3] as u32);

        result.push(((n >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((n >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((n & 0xFF) as u8);
        }
    }

    Ok(result)
}

fn decode_char(c: u8, config: Base64Config) -> Result<u8, &'static str> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' if matches!(config, Base64Config::Standard) => Ok(62),
        b'/' if matches!(config, Base64Config::Standard) => Ok(63),
        b'-' if matches!(config, Base64Config::UrlSafe) => Ok(62),
        b'_' if matches!(config, Base64Config::UrlSafe) => Ok(63),
        _ => Err("Invalid character in Base64 input"),
    }
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
    fn test_decode_url_safe_explicit() {
        assert_eq!(decode_url_safe("-_-_").unwrap(), [251, 255, 191]);
        assert!(decode_standard("-_-_").is_err());
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
        assert!(decode_standard("Zg=").is_err()); // len inválido
        assert!(decode_standard("Z===").is_err()); // padding inválido
        assert!(decode_standard("=m9v").is_err()); // padding em posição inválida
        assert!(decode_standard("Zm=v").is_err()); // padding interno inválido
    }
}
