use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};


/// TODO: add docs
pub fn extract_preloads(document: &HtmlDocument, options: &ParserOptions) -> Vec<PreloadRequest> {
    let mut out = Vec::new();
    for node in &document.children {
        collect_preloads(node, options, &mut out);
    }
    out
}
