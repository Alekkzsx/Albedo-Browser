use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


/// TODO: add docs
pub fn build_fragment_with_errors(html: &str, context: Option<&str>) -> ParseResult {
    let context = context.map(FragmentContext::new);
    html5ever_parser::parse_fragment_html5ever(html, context.as_ref(), &ParserOptions::default())
}
