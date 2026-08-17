//! # Processador de Data URLs (WHATWG Fetch Standard Section 4.5)
//!
//! Decodificador de URLs no esquema `data:`, suportando payloads binários em Base64
//! e strings percent-encoded com resolução automática de MIME types.

use crate::error::AceError;
use crate::net::mime::MimeType;

/// Representa o registro decodificado de um `data:` URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataUrlRecord {
    /// O tipo MIME associado (padrão: `text/plain;charset=US-ASCII`).
    pub mime_type: MimeType,
    /// O corpo binário decodificado do recurso.
    pub body: Vec<u8>,
    /// Indica se o payload de origem utilizou codificação Base64.
    pub is_base64: bool,
}

/// Analisa e decodifica um `data:` URL de acordo com a especificação WHATWG Fetch Standard.
pub fn parse_data_url(input: &str) -> Result<DataUrlRecord, AceError> {
    let trimmed = input.trim();
    if !trimmed.starts_with("data:") {
        return Err(AceError::net("DataUrl", "URL não inicia com o esquema data:"));
    }

    let raw_content = &trimmed[5..]; // Pula "data:"
    let comma_idx = raw_content.find(',').ok_or_else(|| {
        AceError::net("DataUrl", "URL data: inválida (vírgula delimitadora não encontrada)")
    })?;

    let mut metadata = &raw_content[..comma_idx];
    let raw_data = &raw_content[comma_idx + 1..];

    let mut is_base64 = false;
    if metadata.ends_with(";base64") {
        is_base64 = true;
        metadata = &metadata[..metadata.len() - 7];
    }

    let mime_type = if metadata.trim().is_empty() {
        MimeType::parse("text/plain;charset=US-ASCII")
            .unwrap_or_else(|| MimeType::new("text", "plain"))
    } else {
        MimeType::parse(metadata)
            .unwrap_or_else(|| MimeType::new("text", "plain"))
    };

    let body = if is_base64 {
        decode_base64_whatwg(raw_data)?
    } else {
        decode_percent_encoded(raw_data)
    };

    Ok(DataUrlRecord {
        mime_type,
        body,
        is_base64,
    })
}

/// Decodifica Base64 com tolerância a espaços em branco conforme WHATWG Fetch 4.5.3.
fn decode_base64_whatwg(input: &str) -> Result<Vec<u8>, AceError> {
    // Filtra espaços em branco ASCII
    let filtered: Vec<u8> = input
        .bytes()
        .filter(|&b| !matches!(b, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
        .collect();

    if filtered.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::with_capacity(filtered.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits_collected = 0;

    for &byte in &filtered {
        if byte == b'=' {
            break; // Padding
        }

        let val = decode_base64_char(byte).ok_or_else(|| {
            AceError::net("DataUrl", format!("Caractere Base64 inválido: '{}'", byte as char))
        })?;

        buffer = (buffer << 6) | (val as u32);
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}

#[inline]
fn decode_base64_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decodifica sequências percent-encoded (`%XX`).
fn decode_percent_encoded(input: &str) -> Vec<u8> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h1), Some(h2)) = (from_hex_digit(bytes[i + 1]), from_hex_digit(bytes[i + 2])) {
                output.push((h1 << 4) | h2);
                i += 3;
                continue;
            }
        }
        output.push(bytes[i]);
        i += 1;
    }

    output
}

#[inline]
fn from_hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
