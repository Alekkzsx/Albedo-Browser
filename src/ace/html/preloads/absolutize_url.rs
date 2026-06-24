use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



/// TODO: add docs
pub fn absolutize_url(url: &str, base_url: Option<&str>) -> String {
    if url.contains("://") || url.starts_with('/') || base_url.is_none() {
        url.to_string()
    } else {
        let base = base_url.expect("Albedo Engine: internal invariant violated");
        if base.ends_with('/') {
            format!("{base}{url}")
        } else {
            format!("{base}/{url}")
        }
    }
}
