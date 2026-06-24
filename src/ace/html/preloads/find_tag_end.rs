use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn find_tag_end(bytes: &[u8], start: usize) -> usize {
    let mut pos = start;
    let mut quote = None;

    while pos < bytes.len() {
        let byte = bytes[pos];
        if let Some(expected) = quote {
            if byte == expected {
                quote = None;
            }
            pos += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' => quote = Some(byte),
            b'>' => return pos,
            _ => {}
        }

        pos += 1;
    }

    bytes.len()
}
