//! # Detecção e Decodificação de Encodings de Texto
//!
//! Implementação da especificação WHATWG Encoding:
//! - Detecção de BOM (Byte Order Mark)
//! - Extração de charset a partir de cabeçalhos `Content-Type`
//! - Decodificação resiliente para UTF-8 via `encoding_rs`

use smol_str::SmolStr;

/// Resultado da detecção de encoding de uma resposta HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedEncoding {
    pub name: SmolStr,
    pub bom_offset: usize,
}

/// Analisa os primeiros bytes da resposta para detectar se há BOM normativo.
pub fn sniff_bom(bytes: &[u8]) -> Option<DetectedEncoding> {
    if bytes.len() >= 3 && bytes[0..3] == [0xEF, 0xBB, 0xBF] {
        return Some(DetectedEncoding {
            name: "utf-8".into(),
            bom_offset: 3,
        });
    }

    if bytes.len() >= 2 {
        if bytes[0..2] == [0xFE, 0xFF] {
            return Some(DetectedEncoding {
                name: "utf-16be".into(),
                bom_offset: 2,
            });
        }
        if bytes[0..2] == [0xFF, 0xFE] {
            return Some(DetectedEncoding {
                name: "utf-16le".into(),
                bom_offset: 2,
            });
        }
    }

    None
}

/// Extrai o parâmetro `charset` de uma string de cabeçalho `Content-Type`.
pub fn extract_charset_from_content_type(content_type: &str) -> Option<SmolStr> {
    for param in content_type.split(';') {
        let trimmed = param.trim();
        if let Some(eq_idx) = trimmed.find('=') {
            let key = trimmed[..eq_idx].trim();
            if key.eq_ignore_ascii_case("charset") {
                let mut val = trimmed[eq_idx + 1..].trim();
                // Remove aspas se houver
                if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                    val = &val[1..val.len() - 1];
                }
                if !val.is_empty() {
                    return Some(val.to_ascii_lowercase().into());
                }
            }
        }
    }
    None
}

/// Decodifica um buffer de bytes para string Rust UTF-8, considerando charset informado ou fallback.
pub fn decode_to_string(bytes: &[u8], explicit_charset: Option<&str>) -> String {
    // 1. Prioridade máxima: BOM detectado no fluxo de bytes
    if let Some(bom) = sniff_bom(bytes) {
        let payload = &bytes[bom.bom_offset..];
        if let Some(encoding) = encoding_rs::Encoding::for_label(bom.name.as_bytes()) {
            let (cow, _, _) = encoding.decode(payload);
            return cow.into_owned();
        }
    }

    // 2. Se houver charset explícito vindo do cabeçalho Content-Type
    if let Some(label) = explicit_charset {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (cow, _, _) = encoding.decode(bytes);
            return cow.into_owned();
        }
    }

    // 3. Fallback normativo da Web: UTF-8 lossy
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_charset() {
        assert_eq!(
            extract_charset_from_content_type("text/html; charset=UTF-8").as_deref(),
            Some("utf-8")
        );
        assert_eq!(
            extract_charset_from_content_type("text/html; CHARSET=UTF-8").as_deref(),
            Some("utf-8")
        );
        assert_eq!(
            extract_charset_from_content_type("text/html; Charset = \"windows-1252\"").as_deref(),
            Some("windows-1252")
        );
        assert_eq!(
            extract_charset_from_content_type("text/html; charset=\"iso-8859-1\"").as_deref(),
            Some("iso-8859-1")
        );
        assert_eq!(
            extract_charset_from_content_type("application/json").as_deref(),
            None
        );
    }

    #[test]
    fn test_sniff_bom() {
        let utf8_with_bom = [0xEF, 0xBB, 0xBF, b'a', b'b'];
        let detected = sniff_bom(&utf8_with_bom).unwrap();
        assert_eq!(detected.name, "utf-8");
        assert_eq!(detected.bom_offset, 3);

        let decoded = decode_to_string(&utf8_with_bom, None);
        assert_eq!(decoded, "ab");
    }

    #[test]
    fn test_decode_latin1() {
        // "olá" em ISO-8859-1: 'o'=0x6F, 'l'=0x6C, 'á'=0xE1
        let latin1_bytes = [0x6F, 0x6C, 0xE1];
        let decoded = decode_to_string(&latin1_bytes, Some("iso-8859-1"));
        assert_eq!(decoded, "olá");
    }
}
