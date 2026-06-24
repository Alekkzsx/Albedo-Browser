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
pub fn parse_fragment_with_result(
    html: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> ParseResult {
    let parse_span = trace_span!(
        "ace_html.parse_fragment",
        input_bytes = html.len(),
        has_context = context.is_some(),
        scripting_enabled = options.scripting_enabled
    );
    let _guard = parse_span.enter();
    let started = Instant::now();

    let mut result = html5ever_parser::parse_fragment_html5ever(html, context, options);
    apply_parse_telemetry(&mut result, started.elapsed(), false, html.len());
    debug!(
        parse_time_us = result.stats.parse_time_us as u64,
        total_errors = result.stats.total_errors,
        total_preloads = result.stats.total_preloads,
        "ace-html fragment parse complete"
    );
    result
}
