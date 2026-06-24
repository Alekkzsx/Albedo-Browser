use super::*;
use std::collections::{BTreeMap, HashMap};



/// TODO: add docs
pub fn parse_next_attribute(html: &str, idx: &mut usize) -> Option<(String, String)> {
    let bytes = html.as_bytes();
    while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
        *idx += 1;
    }
    if *idx >= bytes.len() {
        return None;
    }
    if bytes[*idx] == b'>' || bytes[*idx] == b'/' {
        return None;
    }

    let name_start = *idx;
    while *idx < bytes.len()
        && !bytes[*idx].is_ascii_whitespace()
        && bytes[*idx] != b'='
        && bytes[*idx] != b'>'
        && bytes[*idx] != b'/'
    {
        *idx += 1;
    }

    if *idx == name_start {
        return None;
    }

    let name = html[name_start..*idx].to_ascii_lowercase();

    while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
        *idx += 1;
    }

    let value = if *idx < bytes.len() && bytes[*idx] == b'=' {
        *idx += 1;
        while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
            *idx += 1;
        }
        if *idx >= bytes.len() {
            String::new()
        } else if bytes[*idx] == b'"' || bytes[*idx] == b'\'' {
            let quote = bytes[*idx];
            *idx += 1;
            let start = *idx;
            while *idx < bytes.len() && bytes[*idx] != quote {
                *idx += 1;
            }
            let value = html[start..*idx].to_string();
            if *idx < bytes.len() {
                *idx += 1;
            }
            value
        } else {
            let start = *idx;
            while *idx < bytes.len()
                && !bytes[*idx].is_ascii_whitespace()
                && bytes[*idx] != b'>'
                && bytes[*idx] != b'/'
            {
                *idx += 1;
            }
            html[start..*idx].to_string()
        }
    } else {
        String::new()
    };

    Some((name, value))
}
