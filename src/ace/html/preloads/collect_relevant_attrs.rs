use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn collect_relevant_attrs(raw: &[u8]) -> FxHashMap<SmolStr, SmolStr> {
    let mut attrs = FxHashMap::default();
    let mut pos = 0usize;

    while pos < raw.len() {
        while pos < raw.len() && (raw[pos].is_ascii_whitespace() || raw[pos] == b'/') {
            pos += 1;
        }

        if pos >= raw.len() {
            break;
        }

        let name_start = pos;
        while pos < raw.len()
            && !raw[pos].is_ascii_whitespace()
            && raw[pos] != b'='
            && raw[pos] != b'/'
            && raw[pos] != b'>'
        {
            pos += 1;
        }

        if pos == name_start {
            break;
        }

        let name = ascii_lower_smol(&raw[name_start..pos]);
        while pos < raw.len() && raw[pos].is_ascii_whitespace() {
            pos += 1;
        }

        let value = if pos < raw.len() && raw[pos] == b'=' {
            pos += 1;
            while pos < raw.len() && raw[pos].is_ascii_whitespace() {
                pos += 1;
            }

            if pos >= raw.len() {
                SmolStr::default()
            } else if raw[pos] == b'"' || raw[pos] == b'\'' {
                let quote = raw[pos];
                pos += 1;
                let value_start = pos;
                while pos < raw.len() && raw[pos] != quote {
                    pos += 1;
                }
                let value = bytes_to_smol(&raw[value_start..pos]);
                if pos < raw.len() {
                    pos += 1;
                }
                value
            } else {
                let value_start = pos;
                while pos < raw.len()
                    && !raw[pos].is_ascii_whitespace()
                    && raw[pos] != b'/'
                    && raw[pos] != b'>'
                {
                    pos += 1;
                }
                bytes_to_smol(&raw[value_start..pos])
            }
        } else {
            SmolStr::default()
        };

        if PRELOAD_ATTRS.contains(name.as_str()) {
            attrs.entry(name).or_insert(value);
        }
    }

    attrs
}
