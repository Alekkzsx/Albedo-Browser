use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn ascii_lower_smol(bytes: &[u8]) -> SmolStr {
    let mut lowered = String::with_capacity(bytes.len());
    for byte in bytes {
        lowered.push((*byte as char).to_ascii_lowercase());
    }
    SmolStr::new(lowered)
}
