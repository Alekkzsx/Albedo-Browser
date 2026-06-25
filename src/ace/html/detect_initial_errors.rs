use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


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
