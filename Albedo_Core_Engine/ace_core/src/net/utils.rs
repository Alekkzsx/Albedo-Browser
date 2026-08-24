//! # Utilitários de Rede, Decodificação e URIs
//!
//! Funções auxiliares para decodificação de `data:` URIs (RFC 2397), percent-decoding e validação de esquemas de rede.

use super::MimeType;

/// Analisa uma `data:` URI (RFC 2397) e extrai o `MimeType` e o buffer decodificado de bytes (suporta Base64 e texto).
pub fn parse_data_uri(data_uri: &str) -> Option<(MimeType, Vec<u8>)> {
    let trimmed = data_uri.trim();
    if !trimmed.starts_with("data:") {
        return None;
    }

    let without_prefix = &trimmed[5..];
    let comma_pos = without_prefix.find(',')?;

    let header = &without_prefix[..comma_pos];
    let body = &without_prefix[comma_pos + 1..];

    let is_base64 = header.ends_with(";base64");
    let mime_header = if is_base64 {
        &header[..header.len() - 7]
    } else {
        header
    };

    let mime = if mime_header.is_empty() {
        MimeType::new("text", "plain")
    } else {
        MimeType::parse(mime_header).unwrap_or_else(|_| MimeType::new("text", "plain"))
    };

    let decoded_bytes = if is_base64 {
        decode_base64(body.trim())?
    } else {
        percent_decode(body).into_bytes()
    };

    Some((mime, decoded_bytes))
}

/// Decodifica uma sequência codificada por percentual (ex: `Hello%20World` -> `Hello World`),
/// preservando bytes em sequências com `%` malformado (ex: `100%_concluido` -> `100%_concluido`).
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(d1), Some(d2)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                output.push((d1 << 4) | d2);
                i += 3;
                continue;
            }
        }
        output.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&output).into_owned()
}


/// Valida se um esquema de protocolo é seguro e padrão para navegação na web.
#[inline]
pub fn is_safe_url_scheme(scheme: &str) -> bool {
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "data" | "blob" | "about" | "file"
    )
}

#[inline]
fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Decodificador Base64 autônomo sem dependências externas.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    let mut output = Vec::with_capacity((input.len() * 3) / 4);

    for &b in input.as_bytes() {
        if b.is_ascii_whitespace() || b == b'=' {
            continue;
        }

        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };

        buffer = (buffer << 6) | (val as u32);
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }

    Some(output)
}
