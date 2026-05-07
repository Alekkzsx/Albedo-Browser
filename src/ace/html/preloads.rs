use std::collections::HashMap;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};

pub fn extract_preloads(document: &HtmlDocument, options: &ParserOptions) -> Vec<PreloadRequest> {
    let mut out = Vec::new();
    for node in &document.children {
        collect_preloads(node, options, &mut out);
    }
    out
}

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

pub fn absolutize_url(url: &str, base_url: Option<&str>) -> String {
    if url.contains("://") || url.starts_with('/') || base_url.is_none() {
        url.to_string()
    } else {
        let base = base_url.unwrap();
        if base.ends_with('/') {
            format!("{base}{url}")
        } else {
            format!("{base}/{url}")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreloadTagKind {
    Link,
    Script,
}

static PRELOAD_TAGS: phf::Map<&'static str, PreloadTagKind> = phf_map! {
    "link" => PreloadTagKind::Link,
    "script" => PreloadTagKind::Script,
};

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

pub fn collect_preloads_fast(html: &str, options: &ParserOptions) -> Vec<PreloadRequest> {
    if !might_have_preload_tags(html.as_bytes()) {
        return Vec::new();
    }

    scan_preloads_fast(html, options)
}

fn might_have_preload_tags(bytes: &[u8]) -> bool {
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            return false;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            return false;
        }

        match bytes[tag_open].to_ascii_lowercase() {
            b'l' if ascii_tag_starts_with(&bytes[tag_open..], b"link") => return true,
            b's' if ascii_tag_starts_with(&bytes[tag_open..], b"script") => return true,
            _ => {
                cursor = tag_open;
            }
        }
    }

    false
}

fn ascii_tag_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }

    if !haystack
        .iter()
        .zip(needle.iter())
        .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
    {
        return false;
    }

    haystack.get(needle.len()).map_or(true, |next| {
        next.is_ascii_whitespace() || matches!(*next, b'>' | b'/')
    })
}

fn scan_preloads_fast(html: &str, options: &ParserOptions) -> Vec<PreloadRequest> {
    let bytes = html.as_bytes();
    let mut cursor = 0usize;
    let mut out = Vec::new();

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            break;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            break;
        }

        match bytes[tag_open] {
            b'/' | b'!' | b'?' => {
                cursor = tag_open;
                continue;
            }
            _ => {}
        }

        let mut tag_end = tag_open;
        while tag_end < bytes.len()
            && (bytes[tag_end].is_ascii_alphanumeric() || bytes[tag_end] == b'-')
        {
            tag_end += 1;
        }

        if tag_end == tag_open {
            cursor = tag_open;
            continue;
        }

        let tag_name = ascii_lower_smol(&bytes[tag_open..tag_end]);
        let Some(tag_kind) = PRELOAD_TAGS.get(tag_name.as_str()) else {
            cursor = tag_end;
            continue;
        };

        let end_of_tag = find_tag_end(bytes, tag_end);
        let attrs = collect_relevant_attrs(&bytes[tag_end..end_of_tag]);

        match tag_kind {
            PreloadTagKind::Link => {
                if let Some(request) = preload_from_link_attrs(&attrs, options) {
                    out.push(request);
                }
            }
            PreloadTagKind::Script => {
                if let Some(request) = preload_from_script_attrs(&attrs, options) {
                    out.push(request);
                }
            }
        }

        cursor = end_of_tag.saturating_add(1).min(bytes.len());
    }

    out
}

fn find_tag_end(bytes: &[u8], start: usize) -> usize {
    let mut idx = start;
    let mut quote = None;

    while idx < bytes.len() {
        let byte = bytes[idx];
        if let Some(expected) = quote {
            if byte == expected {
                quote = None;
            }
            idx += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' => quote = Some(byte),
            b'>' => return idx,
            _ => {}
        }

        idx += 1;
    }

    bytes.len()
}

fn collect_relevant_attrs(raw: &[u8]) -> FxHashMap<SmolStr, SmolStr> {
    let mut attrs = FxHashMap::default();
    let mut idx = 0usize;

    while idx < raw.len() {
        while idx < raw.len() && (raw[idx].is_ascii_whitespace() || raw[idx] == b'/') {
            idx += 1;
        }

        if idx >= raw.len() {
            break;
        }

        let name_start = idx;
        while idx < raw.len()
            && !raw[idx].is_ascii_whitespace()
            && raw[idx] != b'='
            && raw[idx] != b'/'
            && raw[idx] != b'>'
        {
            idx += 1;
        }

        if idx == name_start {
            break;
        }

        let name = ascii_lower_smol(&raw[name_start..idx]);
        while idx < raw.len() && raw[idx].is_ascii_whitespace() {
            idx += 1;
        }

        let value = if idx < raw.len() && raw[idx] == b'=' {
            idx += 1;
            while idx < raw.len() && raw[idx].is_ascii_whitespace() {
                idx += 1;
            }

            if idx >= raw.len() {
                SmolStr::default()
            } else if raw[idx] == b'"' || raw[idx] == b'\'' {
                let quote = raw[idx];
                idx += 1;
                let value_start = idx;
                while idx < raw.len() && raw[idx] != quote {
                    idx += 1;
                }
                let value = bytes_to_smol(&raw[value_start..idx]);
                if idx < raw.len() {
                    idx += 1;
                }
                value
            } else {
                let value_start = idx;
                while idx < raw.len()
                    && !raw[idx].is_ascii_whitespace()
                    && raw[idx] != b'/'
                    && raw[idx] != b'>'
                {
                    idx += 1;
                }
                bytes_to_smol(&raw[value_start..idx])
            }
        } else {
            SmolStr::default()
        };

        if PRELOAD_ATTRS.contains(name.as_str()) {
            attrs.entry(name).or_insert(value);
        }
    }

    attrs
}

fn preload_from_link_attrs(
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

fn preload_from_script_attrs(
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

fn ascii_lower_smol(bytes: &[u8]) -> SmolStr {
    let mut lowered = String::with_capacity(bytes.len());
    for byte in bytes {
        lowered.push((*byte as char).to_ascii_lowercase());
    }
    SmolStr::new(lowered)
}

fn bytes_to_smol(bytes: &[u8]) -> SmolStr {
    SmolStr::new(String::from_utf8_lossy(bytes).as_ref())
}
