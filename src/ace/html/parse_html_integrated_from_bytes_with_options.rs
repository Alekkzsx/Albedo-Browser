use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


/// TODO: add docs
pub fn parse_html_integrated_from_bytes_with_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    parse_document_from_bytes_with_errors_and_options(bytes, bom, options)
}
