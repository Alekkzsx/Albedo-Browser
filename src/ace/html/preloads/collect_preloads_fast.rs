use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



/// TODO: add docs
pub fn collect_preloads_fast(html: &str, options: &ParserOptions) -> Vec<PreloadRequest> {
    if !might_have_preload_tags(html.as_bytes()) {
        return Vec::new();
    }

    scan_preloads_fast(html, options)
}
