//! Wrapper integrado para parsing HTML com estatísticas resumidas.

use super::tree_builder::build_document_with_errors;
use super::{HtmlDocument, HtmlNode, PreloadRequest};

pub struct ParseResult {
    pub document: HtmlDocument,
    pub errors: Vec<String>,
    pub preload_requests: Vec<PreloadRequest>,
    pub stats: ParserStats,
}

pub struct ParserStats {
    pub arena_chunk_count: usize,
    pub arena_capacity_kb: usize,
    pub arena_utilization: f32,
    pub interner_unique_strings: usize,
    pub interner_hit_rate: f32,
    pub total_errors: usize,
    pub total_preloads: usize,
}

pub struct IntegratedTreeBuilder<'a> {
    input: &'a str,
}

impl<'a> IntegratedTreeBuilder<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub fn parse(self) -> ParseResult {
        let output = build_document_with_errors(self.input);
        let node_count = count_document_nodes(&output.document);

        ParseResult {
            stats: ParserStats {
                arena_chunk_count: node_count.max(1),
                arena_capacity_kb: 0,
                arena_utilization: 0.0,
                interner_unique_strings: count_unique_strings(&output.document),
                interner_hit_rate: 0.0,
                total_errors: output.errors.len(),
                total_preloads: output.preload_requests.len(),
            },
            errors: output.errors.into_iter().map(|e| e.message).collect(),
            preload_requests: output.preload_requests,
            document: output.document,
        }
    }
}

pub fn parse_html_integrated(input: &str) -> ParseResult {
    IntegratedTreeBuilder::new(input).parse()
}

fn count_document_nodes(document: &HtmlDocument) -> usize {
    document.children.iter().map(count_node).sum()
}

fn count_node(node: &HtmlNode) -> usize {
    match node {
        HtmlNode::Element(el) => 1 + el.children.iter().map(count_node).sum::<usize>(),
        HtmlNode::Text(_) | HtmlNode::Comment(_) => 1,
    }
}

fn count_unique_strings(document: &HtmlDocument) -> usize {
    use std::collections::HashSet;

    fn walk(node: &HtmlNode, seen: &mut HashSet<String>) {
        match node {
            HtmlNode::Element(el) => {
                seen.insert(el.tag.clone());
                for (key, value) in &el.attributes {
                    seen.insert(key.clone());
                    seen.insert(value.clone());
                }
                if let Some(slot_name) = &el.slot_name {
                    seen.insert(slot_name.clone());
                }
                if let Some(is_value) = &el.is_value {
                    seen.insert(is_value.clone());
                }
                for child in &el.children {
                    walk(child, seen);
                }
            }
            HtmlNode::Text(text) | HtmlNode::Comment(text) => {
                seen.insert(text.clone());
            }
        }
    }

    let mut seen = HashSet::new();
    for child in &document.children {
        walk(child, &mut seen);
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let r = parse_html_integrated("<!DOCTYPE html><html><body>Hi</body></html>");
        assert!(!r.document.children.is_empty());
    }

    #[test]
    fn test_preloads_are_exposed() {
        let r = parse_html_integrated("<link rel=\"stylesheet\" href=\"app.css\">");
        assert_eq!(r.stats.total_preloads, 1);
    }
}
