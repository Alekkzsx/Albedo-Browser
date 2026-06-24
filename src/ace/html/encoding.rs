use std::collections::HashMap;
use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252};
use memchr::memchr;
use super::types::{Encoding, DecodedHtml, ParseError, parse_next_attribute};

pub fn decode_html_bytes(
    bytes: &[u8],
    bom: Option<&[u8]>,
    hint: Option<Encoding>,
) -> Result<DecodedHtml, ParseError> {
    let (encoding, bom_len) = sniff_document_encoding(bytes, bom, hint);
    let payload = &bytes[bom_len.min(bytes.len())..];

    let content = match encoding {
        Encoding::Utf8 => match simdutf8::basic::from_utf8(payload) {
            Ok(valid) => valid.to_string(),
            Err(_) => UTF_8.decode_without_bom_handling(payload).0.into_owned(),
        },
        Encoding::Windows1252 => WINDOWS_1252
            .decode_without_bom_handling(payload)
            .0
            .into_owned(),
        Encoding::Utf16Le => UTF_16LE
            .decode_without_bom_handling(payload)
            .0
            .into_owned(),
        Encoding::Utf16Be => UTF_16BE
            .decode_without_bom_handling(payload)
            .0
            .into_owned(),
    };

    Ok(DecodedHtml { content, encoding })
}

pub fn sniff_document_encoding(
    bytes: &[u8],
    bom: Option<&[u8]>,
    hint: Option<Encoding>,
) -> (Encoding, usize) {
    if let Some(bom_bytes) = bom {
        if let Some((encoding, len)) = detect_bom(bom_bytes) {
            return (encoding, len);
        }
    }

    if let Some((encoding, len)) = detect_bom(bytes) {
        return (encoding, len);
    }

    if let Some(encoding) = hint {
        return (encoding, 0);
    }

    if let Some(meta_encoding) = sniff_meta_charset(bytes) {
        return (meta_encoding, 0);
    }

    if simdutf8::basic::from_utf8(bytes).is_ok() {
        (Encoding::Utf8, 0)
    } else {
        (Encoding::Windows1252, 0)
    }
}

pub fn detect_bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Some((Encoding::Utf8, 3));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Some((Encoding::Utf16Le, 2));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Some((Encoding::Utf16Be, 2));
    }
    None
}

pub fn sniff_meta_charset(bytes: &[u8]) -> Option<Encoding> {
    let head = &bytes[..bytes.len().min(4096)];
    let head_str = String::from_utf8_lossy(head);
    let lower = head_str.to_ascii_lowercase();
    let lower_bytes = lower.as_bytes();
    let mut cursor = 0usize;

    while cursor < lower_bytes.len() {
        let Some(open_rel) = memchr(b'<', &lower_bytes[cursor..]) else {
            break;
        };
        let tag_start = cursor + open_rel;
        if !lower_bytes[tag_start..].starts_with(b"<meta") {
            cursor = tag_start + 1;
            continue;
        }

        let tag_tail = &lower_bytes[tag_start..];
        let tag_end_rel = memchr(b'>', tag_tail).unwrap_or(tag_tail.len().saturating_sub(1));
        let end = (tag_start + tag_end_rel + 1).min(lower.len());
        if end <= tag_start {
            break;
        }
        let tag = &lower[tag_start..end];

        if let Some(charset) = extract_meta_charset(tag) {
            if let Some(enc) = encoding_from_label(&charset) {
                return Some(enc);
            }
        }

        cursor = end;
    }
    None
}

pub fn extract_meta_charset(tag: &str) -> Option<String> {
    let attrs = parse_meta_attributes(tag);
    if let Some(charset) = attrs.get("charset").filter(|value| !value.is_empty()) {
        return Some(charset.clone());
    }

    let content = attrs.get("content")?;
    let lower = content.to_ascii_lowercase();
    let charset_idx = lower.find("charset=")?;
    let raw = &content[charset_idx + "charset=".len()..];
    let trimmed = raw.trim_start();
    let charset = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or("").trim()
    } else if let Some(rest) = trimmed.strip_prefix('\'') {
        rest.split('\'').next().unwrap_or("").trim()
    } else {
        trimmed
            .split(|ch: char| ch == ';' || ch.is_whitespace())
            .next()
            .unwrap_or("")
            .trim()
    };

    if charset.is_empty() {
        None
    } else {
        Some(charset.to_string())
    }
}

pub fn parse_meta_attributes(tag: &str) -> HashMap<String, String> {
    let bytes = tag.as_bytes();
    let mut char_index = 0usize;
    let mut attrs = HashMap::new();

    while char_index < bytes.len() && bytes[char_index] != b' ' && bytes[char_index] != b'>' {
        char_index += 1;
    }

    while char_index < bytes.len() {
        while char_index < bytes.len()
            && (bytes[char_index].is_ascii_whitespace() || bytes[char_index] == b'/' || bytes[char_index] == b'>')
        {
            char_index += 1;
        }
        if char_index >= bytes.len() {
            break;
        }

        if let Some((name, value)) = parse_next_attribute(tag, &mut char_index) {
            attrs.entry(name).or_insert(value);
        } else {
            break;
        }
    }

    attrs
}

pub fn encoding_from_label(label: &str) -> Option<Encoding> {
    let normalized = label.trim().trim_matches('"').trim_matches('\'');
    let canonical = encoding_rs::Encoding::for_label(normalized.as_bytes())?;
    match canonical.name().to_ascii_lowercase().as_str() {
        "utf-8" => Some(Encoding::Utf8),
        "windows-1252" => Some(Encoding::Windows1252),
        "utf-16le" | "utf-16" => Some(Encoding::Utf16Le),
        "utf-16be" => Some(Encoding::Utf16Be),
        _ => None,
    }
}
