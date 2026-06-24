use std::collections::HashMap;
use memchr::memchr;
use super::types::{
    HtmlDocument, HtmlElement, HtmlNode, DoctypeToken, ParseResult, ParseStats, ParserOptions,
    Namespace, is_void_element, parse_next_attribute
};
use super::preloads::extract_preloads;

const FAST_PATH_MIN_BYTES: usize = 16 * 1024;

pub fn try_fast_parse_document(html: &str, options: &ParserOptions) -> Option<ParseResult> {
    if !is_fast_path_candidate(html) {
        return None;
    }

    let bytes = html.as_bytes();
    let mut cursor = 0usize;
    let mut document = HtmlDocument {
        doctype: None,
        children: Vec::new(),
    };
    let mut root = Vec::<HtmlNode>::new();
    let mut stack = Vec::<HtmlElement>::new();

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            append_fast_text(&mut root, &mut stack, &html[cursor..]);
            break;
        };

        let tag_start = cursor + relative;
        append_fast_text(&mut root, &mut stack, &html[cursor..tag_start]);

        if tag_start + 1 >= bytes.len() {
            return None;
        }

        match bytes[tag_start + 1] {
            b'!' => {
                let after_bang = &html[tag_start + 2..];
                if after_bang.len() < 7 {
                    return None;
                }
                if !after_bang[..7].eq_ignore_ascii_case("doctype") {
                    return None;
                }

                let Some(tag_end_rel) = memchr(b'>', &bytes[tag_start..]) else {
                    return None;
                };
                let tag_end = tag_start + tag_end_rel;
                let inner = html[tag_start + 2..tag_end].trim();
                let mut parts = inner.split_whitespace();
                let keyword = parts.next()?;
                if !keyword.eq_ignore_ascii_case("doctype") {
                    return None;
                }
                let name = parts.next().unwrap_or("html").to_ascii_lowercase();
                document.doctype = Some(DoctypeToken {
                    name: Some(name),
                    public_id: None,
                    system_id: None,
                    force_quirks: false,
                });
                cursor = tag_end + 1;
            }
            b'/' => {
                let (tag_name, tag_end) = parse_fast_end_tag(html, tag_start)?;
                let current = stack.pop()?;
                if !current.tag.eq_ignore_ascii_case(&tag_name) {
                    return None;
                }
                push_node(&mut root, &mut stack, HtmlNode::Element(current));
                cursor = tag_end + 1;
            }
            _ => {
                let (element, self_closing, tag_end) = parse_fast_start_tag(html, tag_start)?;
                if self_closing || is_void_element(&element.tag) {
                    push_node(&mut root, &mut stack, HtmlNode::Element(element));
                } else {
                    stack.push(element);
                }
                cursor = tag_end + 1;
            }
        }
    }

    while let Some(element) = stack.pop() {
        push_node(&mut root, &mut stack, HtmlNode::Element(element));
    }

    document.children = if options.scripting_enabled {
        root
    } else {
        super::transform_noscript(root)
    };

    let preload_requests = if fast_path_has_link_tag(bytes) {
        extract_preloads(&document, options)
    } else {
        Vec::new()
    };
    Some(ParseResult {
        document,
        errors: Vec::new(),
        parse_errors: Vec::new(),
        preload_requests: preload_requests.clone(),
        stats: ParseStats {
            total_errors: 0,
            total_preloads: preload_requests.len(),
            ..ParseStats::default()
        },
    })
}

fn is_fast_path_candidate(html: &str) -> bool {
    if html.len() < FAST_PATH_MIN_BYTES {
        return false;
    }

    let lowercase = html.to_ascii_lowercase();
    !lowercase.contains('&')
        && !lowercase.contains('\0')
        && !lowercase.contains("<!--")
        && !lowercase.contains("<?")
        && !lowercase.contains("<![cdata[")
        && !lowercase.contains("<script")
        && !lowercase.contains("<style")
        && !lowercase.contains("<noscript")
}

fn fast_path_has_link_tag(bytes: &[u8]) -> bool {
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            return false;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            return false;
        }
        if ascii_starts_with(bytes.get(tag_open..).unwrap_or_default(), b"link") {
            let boundary = bytes.get(tag_open + 4).map_or(true, |next| {
                next.is_ascii_whitespace() || matches!(*next, b'>' | b'/')
            });
            if boundary {
                return true;
            }
        }
        cursor = tag_open;
    }
    false
}

fn ascii_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack
            .iter()
            .zip(needle.iter())
            .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
}

fn append_fast_text(root: &mut Vec<HtmlNode>, stack: &mut [HtmlElement], text: &str) {
    if !text.is_empty() {
        push_node(root, stack, HtmlNode::Text(text.to_string()));
    }
}

fn push_node(root: &mut Vec<HtmlNode>, stack: &mut [HtmlElement], node: HtmlNode) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        root.push(node);
    }
}

fn parse_fast_end_tag(html: &str, tag_start: usize) -> Option<(String, usize)> {
    let bytes = html.as_bytes();
    let mut char_index = tag_start + 2;
    while char_index < bytes.len() && bytes[char_index].is_ascii_whitespace() {
        char_index += 1;
    }
    let name_start = char_index;
    while char_index < bytes.len() && (bytes[char_index].is_ascii_alphanumeric() || bytes[char_index] == b'-') {
        char_index += 1;
    }
    if char_index == name_start {
        return None;
    }
    let name = html[name_start..char_index].to_ascii_lowercase();
    while char_index < bytes.len() && bytes[char_index].is_ascii_whitespace() {
        char_index += 1;
    }
    if bytes.get(char_index) != Some(&b'>') {
        return None;
    }
    Some((name, char_index))
}

fn parse_fast_start_tag(html: &str, tag_start: usize) -> Option<(HtmlElement, bool, usize)> {
    let bytes = html.as_bytes();
    let mut char_index = tag_start + 1;
    let name_start = char_index;
    while char_index < bytes.len() && (bytes[char_index].is_ascii_alphanumeric() || bytes[char_index] == b'-') {
        char_index += 1;
    }
    if char_index == name_start {
        return None;
    }

    let tag = html[name_start..char_index].to_ascii_lowercase();
    let namespace = Namespace::Html;
    let mut attributes = HashMap::new();
    let mut self_closing = false;

    loop {
        while char_index < bytes.len() && bytes[char_index].is_ascii_whitespace() {
            char_index += 1;
        }

        match bytes.get(char_index).copied() {
            Some(b'>') => {
                return Some((
                    HtmlElement {
                        tag,
                        namespace,
                        attributes,
                        children: Vec::new(),
                    },
                    self_closing,
                    char_index,
                ));
            }
            Some(b'/') if bytes.get(char_index + 1) == Some(&b'>') => {
                self_closing = true;
                char_index += 1;
            }
            Some(_) => {
                let (attr_name, value) = parse_next_attribute(html, &mut char_index)?;
                attributes.entry(attr_name).or_insert(value);
            }
            None => return None,
        }
    }
}


