use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


pub(crate) fn decode_legacy_cdata_comment(data: &str) -> Option<String> {
    data.strip_prefix("[CDATA[")
        .and_then(|inner| inner.strip_suffix("]]"))
        .map(str::to_string)
}
