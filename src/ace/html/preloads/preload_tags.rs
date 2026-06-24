use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



static PRELOAD_TAGS: phf::Map<&'static str, PreloadTagKind> = phf_map! {
    "link" => PreloadTagKind::Link,
    "script" => PreloadTagKind::Script,
};
