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
pub fn parse_document_with_errors_and_options(html: &str, options: &ParserOptions) -> ParseResult {
    let parse_span = trace_span!(
        "ace_html.parse_document",
        input_bytes = html.len(),
        scripting_enabled = options.scripting_enabled
    );
    let _guard = parse_span.enter();
    let started = Instant::now();

    let mut fast_path_used = false;
    let mut result = if let Some(result) = fast_parse::try_fast_parse_document(html, options) {
        fast_path_used = true;
        result
    } else {
        html5ever_parser::parse_document_html5ever(html, options)
    };

    apply_parse_telemetry(&mut result, started.elapsed(), fast_path_used, html.len());
    debug!(
        parse_time_us = result.stats.parse_time_us as u64,
        total_errors = result.stats.total_errors,
        total_preloads = result.stats.total_preloads,
        fast_path_used = result.stats.fast_path_used,
        "ace-html document parse complete"
    );
    result
}
