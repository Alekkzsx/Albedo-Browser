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
