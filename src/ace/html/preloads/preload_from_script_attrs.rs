use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn preload_from_script_attrs(
    attrs: &FxHashMap<SmolStr, SmolStr>,
    options: &ParserOptions,
) -> Option<PreloadRequest> {
    let src = attrs.get("src")?;
    let type_attr = attrs
        .get("type")
        .map(ToString::to_string)
        .unwrap_or_default();

    Some(PreloadRequest {
        url: absolutize_url(src, options.base_url.as_deref()),
        resource_type: ResourceType::Script,
        priority: RequestPriority::Auto,
        crossorigin: attrs.get("crossorigin").map(ToString::to_string),
        rel: None,
        as_attribute: None,
        fetchpriority: attrs.get("fetchpriority").map(ToString::to_string),
        loading: None,
        is_module: type_attr.eq_ignore_ascii_case("module"),
        is_async: attrs.contains_key("async"),
        is_defer: attrs.contains_key("defer"),
    })
}
