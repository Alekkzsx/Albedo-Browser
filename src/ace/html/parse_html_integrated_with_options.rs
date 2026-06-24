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
pub fn parse_html_integrated_with_options(html: &str, options: &ParserOptions) -> ParseResult {
    parse_document_with_errors_and_options(html, options)
}
