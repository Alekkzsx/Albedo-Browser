use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreloadTagKind {
    Link,
    Script,
}
