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
pub fn detect_initial_errors(input: &str) -> Vec<ParseError> {
    let mut errors = Vec::new();
    if let Some(index) = input.find("<!DOCTYPE>") {
        let (line, column) = line_column_for_offset(input, index);
        errors.push(ParseError {
            code: "invalid-doctype".to_string(),
            source: ParseErrorSource::Tokenizer,
            kind: ParseErrorKind::InvalidDoctype,
            line,
            column,
            message: "doctype is missing a name".to_string(),
        });
    }
    errors
}
