use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


/// TODO: add docs
pub fn parse_document(html: &str) -> HtmlDocument {
    parse_document_with_options(html, &ParserOptions::default())
}
