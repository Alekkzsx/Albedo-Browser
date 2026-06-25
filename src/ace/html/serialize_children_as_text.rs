use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};





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
