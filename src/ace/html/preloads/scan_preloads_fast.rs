use super::*;
use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};



pub(crate) fn scan_preloads_fast(html: &str, options: &ParserOptions) -> Vec<PreloadRequest> {
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
