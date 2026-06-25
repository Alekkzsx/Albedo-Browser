use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


/// TODO: add docs
pub fn parse_fragment(html: &str, context: Option<&str>) -> Vec<HtmlNode> {
    let context = context.map(FragmentContext::new);
    parse_fragment_with_context(html, context.as_ref(), &ParserOptions::default())
}
