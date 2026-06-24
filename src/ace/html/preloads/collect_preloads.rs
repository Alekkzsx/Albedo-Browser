use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



/// TODO: add docs
pub fn collect_preloads(node: &HtmlNode, options: &ParserOptions, out: &mut Vec<PreloadRequest>) {
    let HtmlNode::Element(element) = node else {
        return;
    };

    if element.tag == "link" {
        if let Some(rel) = element.attributes.get("rel") {
            if rel.contains("stylesheet")
                || rel.contains("preload")
                || rel.contains("modulepreload")
            {
                let href = element.attributes.get("href").cloned().unwrap_or_default();
                out.push(PreloadRequest {
                    url: absolutize_url(&href, options.base_url.as_deref()),
                    resource_type: if rel.contains("modulepreload") {
                        ResourceType::ModulePreload
                    } else {
                        ResourceType::Stylesheet
                    },
                    priority: RequestPriority::Auto,
                    crossorigin: element.attributes.get("crossorigin").cloned(),
                    rel: Some(rel.clone()),
                    as_attribute: element.attributes.get("as").cloned(),
                    fetchpriority: element.attributes.get("fetchpriority").cloned(),
                    loading: element.attributes.get("loading").cloned(),
                    is_module: rel.contains("modulepreload"),
                    is_async: false,
                    is_defer: false,
                });
            }
        }
    }

    if element.tag == "script" {
        if let Some(src) = element.attributes.get("src") {
            out.push(PreloadRequest {
                url: absolutize_url(src, options.base_url.as_deref()),
                resource_type: ResourceType::Script,
                priority: RequestPriority::Auto,
                crossorigin: element.attributes.get("crossorigin").cloned(),
                rel: None,
                as_attribute: None,
                fetchpriority: element.attributes.get("fetchpriority").cloned(),
                loading: None,
                is_module: element
                    .attributes
                    .get("type")
                    .is_some_and(|value| value == "module"),
                is_async: element.attributes.contains_key("async"),
                is_defer: element.attributes.contains_key("defer"),
            });
        }
    }

    for child in &element.children {
        collect_preloads(child, options, out);
    }
}
