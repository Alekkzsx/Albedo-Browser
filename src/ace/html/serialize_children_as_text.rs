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
pub fn serialize_children_as_text(children: &[HtmlNode]) -> String {
    let options = SerializeOptions {
        indent: None,
        escape_text: false,
    };
    let mut out = String::new();
    for child in children {
        out.push_str(&serialize_node(child, &options));
    }
    out
}
