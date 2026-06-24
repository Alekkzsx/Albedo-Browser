use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



static PRELOAD_ATTRS: phf::Set<&'static str> = phf_set! {
    "href",
    "rel",
    "crossorigin",
    "as",
    "fetchpriority",
    "loading",
    "src",
    "type",
    "async",
    "defer",
};
