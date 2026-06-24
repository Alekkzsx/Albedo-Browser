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


pub mod htmltokenizer; pub use htmltokenizer::*;
pub mod parse_document; pub use parse_document::*;
pub mod parse_document_with_options; pub use parse_document_with_options::*;
pub mod parse_document_with_errors; pub use parse_document_with_errors::*;
pub mod build_document_with_errors; pub use build_document_with_errors::*;
pub mod build_fragment_with_errors; pub use build_fragment_with_errors::*;
pub mod parse_fragment; pub use parse_fragment::*;
pub mod parse_fragment_with_context; pub use parse_fragment_with_context::*;
pub mod parse_fragment_with_result; pub use parse_fragment_with_result::*;
pub mod parse_document_with_errors_and_options; pub use parse_document_with_errors_and_options::*;
pub mod parse_html_integrated_with_options; pub use parse_html_integrated_with_options::*;
pub mod parse_document_from_bytes_with_errors_and_options; pub use parse_document_from_bytes_with_errors_and_options::*;
pub mod parse_html_integrated_from_bytes_with_options; pub use parse_html_integrated_from_bytes_with_options::*;
pub mod apply_parse_telemetry; pub use apply_parse_telemetry::*;
pub mod map_html5gum_token; pub use map_html5gum_token::*;
pub mod decode_text_token; pub use decode_text_token::*;
pub mod decode_legacy_cdata_comment; pub use decode_legacy_cdata_comment::*;
pub mod map_html5gum_error; pub use map_html5gum_error::*;
pub mod detect_initial_errors; pub use detect_initial_errors::*;
pub mod rough_error; pub use rough_error::*;
pub mod line_column_for_offset; pub use line_column_for_offset::*;
pub mod transform_noscript; pub use transform_noscript::*;
pub mod serialize_children_as_text; pub use serialize_children_as_text::*;
