use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn bytes_to_smol(bytes: &[u8]) -> SmolStr {
    SmolStr::new(String::from_utf8_lossy(bytes).as_ref())
}
