use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn might_have_preload_tags(bytes: &[u8]) -> bool {
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            return false;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            return false;
        }

        match bytes[tag_open].to_ascii_lowercase() {
            b'l' if ascii_tag_starts_with(&bytes[tag_open..], b"link") => return true,
            b's' if ascii_tag_starts_with(&bytes[tag_open..], b"script") => return true,
            _ => {
                cursor = tag_open;
            }
        }
    }

    false
}
