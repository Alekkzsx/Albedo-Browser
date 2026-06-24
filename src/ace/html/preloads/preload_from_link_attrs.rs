use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn preload_from_link_attrs(
    attrs: &FxHashMap<SmolStr, SmolStr>,
    options: &ParserOptions,
) -> Option<PreloadRequest> {
    let rel = attrs.get("rel")?;
    let rel_lower = rel.to_lowercase();
    if !rel_lower.contains("stylesheet")
        && !rel_lower.contains("preload")
        && !rel_lower.contains("modulepreload")
    {
        return None;
    }

    let href = attrs.get("href").cloned().unwrap_or_default();
    Some(PreloadRequest {
        url: absolutize_url(&href, options.base_url.as_deref()),
        resource_type: if rel_lower.contains("modulepreload") {
            ResourceType::ModulePreload
        } else {
            ResourceType::Stylesheet
        },
        priority: RequestPriority::Auto,
        crossorigin: attrs.get("crossorigin").map(ToString::to_string),
        rel: Some(rel.to_string()),
        as_attribute: attrs.get("as").map(ToString::to_string),
        fetchpriority: attrs.get("fetchpriority").map(ToString::to_string),
        loading: attrs.get("loading").map(ToString::to_string),
        is_module: rel_lower.contains("modulepreload"),
        is_async: false,
        is_defer: false,
    })
}
