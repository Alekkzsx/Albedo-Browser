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


pub use types::*;
pub use tokenizer_v2::AceTokenizer;
pub use tree_builder::{HtmlTreeBuilder, InsertionMode};
pub use streaming::StreamingHtmlParser;
pub use encoding::{decode_html_bytes, sniff_document_encoding, detect_bom};
pub use serializer::{serialize_document, serialize_node, SerializeOptions};

pub struct HtmlTokenizer<'a> {
    tokenizer: Tokenizer<html5gum::StringReader<'a>, DefaultEmitter>,
    errors: Vec<ParseError>,
    emitted_eof: bool,
    input: &'a str,
}

impl<'a> HtmlTokenizer<'a> {
    /// TODO: add docs
    pub fn new(input: &'a str) -> Self {
        let mut emitter = DefaultEmitter::default();
        emitter.switch_states(true);
        Self {
            tokenizer: Tokenizer::new_with_emitter(input, emitter),
            errors: detect_initial_errors(input),
            emitted_eof: false,
            input,
        }
    }

    /// TODO: add docs
    pub fn next_token(&mut self) -> Option<HtmlToken> {
        if self.emitted_eof {
            return None;
        }

        loop {
            match self.tokenizer.next() {
                Some(Ok(token)) => match token {
                    Token::Error(error) => {
                        self.errors.push(map_html5gum_error(self.input, error));
                    }
                    other => match map_html5gum_token(other) {
                        Some(kind) => return Some(HtmlToken { kind }),
                        None => continue,
                    },
                },
                Some(Err(_)) => {
                    self.errors.push(rough_error(
                        self.input,
                        "tokenizer-error",
                        ParseErrorSource::Tokenizer,
                        ParseErrorKind::HtmlSyntax,
                    ));
                }
                None => {
                    self.emitted_eof = true;
                    return Some(HtmlToken {
                        kind: HtmlTokenKind::Eof,
                    });
                }
            }
        }
    }

    /// TODO: add docs
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}
