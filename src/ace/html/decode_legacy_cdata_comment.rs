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



pub(crate) fn decode_legacy_cdata_comment(data: &str) -> Option<String> {
    data.strip_prefix("[CDATA[")
        .and_then(|inner| inner.strip_suffix("]]"))
        .map(str::to_string)
}
