use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


/// TODO: add docs
pub fn rough_error(
    input: &str,
    code: &str,
    source: ParseErrorSource,
    kind: ParseErrorKind,
) -> ParseError {
    let (line, column) = line_column_for_offset(input, 0);
    ParseError {
        code: code.to_string(),
        source,
        kind,
        line,
        column,
        message: code.to_string(),
    }
}
