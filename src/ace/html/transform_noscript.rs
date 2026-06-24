use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};

mod html5ever_parser;
pub mod tokenizer_v2;
pub mod tree_builder;

pub mod types;
pub mod encoding;
pub mod streaming;
pub mod preloads;
pub mod sink;
pub mod fast_parse;
pub mod serializer;



/// TODO: add docs
pub fn transform_noscript(nodes: Vec<HtmlNode>) -> Vec<HtmlNode> {
    nodes
        .into_iter()
        .filter_map(|node| match node {
            HtmlNode::Element(element) if element.tag == "noscript" => {
                // With scripting enabled, html5ever keeps the noscript element
                // and converts its children to text in the sink.
                // We keep the element as-is (already processed by sink.rs).
                Some(HtmlNode::Element(element))
            }
            HtmlNode::Element(mut element) => {
                element.children = transform_noscript(element.children);
                Some(HtmlNode::Element(element))
            }
            other => Some(other),
        })
        .collect()
}
