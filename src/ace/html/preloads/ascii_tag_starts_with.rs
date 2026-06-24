use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn ascii_tag_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }

    if !haystack
        .iter()
        .zip(needle.iter())
        .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
    {
        return false;
    }

    haystack.get(needle.len()).map_or(true, |next| {
        next.is_ascii_whitespace() || matches!(*next, b'>' | b'/')
    })
}
