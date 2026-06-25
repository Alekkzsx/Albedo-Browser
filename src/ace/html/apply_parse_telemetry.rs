use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


pub(crate) fn apply_parse_telemetry(
    result: &mut ParseResult,
    elapsed: Duration,
    fast_path_used: bool,
    input_bytes: usize,
) {
    result.stats.total_errors = result.parse_errors.len();
    result.stats.total_preloads = result.preload_requests.len();
    result.stats.parse_time_us = elapsed.as_micros();
    result.stats.fast_path_used = fast_path_used;
    result.stats.input_bytes = input_bytes;
}
