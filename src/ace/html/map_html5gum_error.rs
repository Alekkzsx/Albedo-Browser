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



pub(crate) fn map_html5gum_error(input: &str, error: Html5Error) -> ParseError {
    let (code, kind) = match error {
        Html5Error::MissingSemicolonAfterCharacterReference => (
            "LEX001",
            ParseErrorKind::MissingSemicolonAfterCharacterReference,
        ),
        Html5Error::UnexpectedNullCharacter | Html5Error::NullCharacterReference => {
            ("TOK004", ParseErrorKind::NullCharacter)
        }
        Html5Error::CdataInHtmlContent => {
            ("LEX001", ParseErrorKind::CdataSectionOutsideForeignContent)
        }
        Html5Error::NestedComment => ("LEX001", ParseErrorKind::NestedComment),
        Html5Error::EofInComment => ("LEX001", ParseErrorKind::EofInComment),
        Html5Error::EofInDoctype => ("LEX001", ParseErrorKind::EofInDoctype),
        Html5Error::EofInTag | Html5Error::EofBeforeTagName => ("LEX001", ParseErrorKind::EofInTag),
        Html5Error::UnexpectedQuestionMarkInsteadOfTagName
        | Html5Error::IncorrectlyOpenedComment
        | Html5Error::InvalidFirstCharacterOfTagName => ("LEX001", ParseErrorKind::LexerParseError),
        _ => ("LEX001", ParseErrorKind::HtmlSyntax),
    };

    let (line, column) = line_column_for_offset(input, 0);
    ParseError {
        code: code.to_string(),
        source: ParseErrorSource::Tokenizer,
        kind,
        line,
        column,
        message: error.as_str().to_string(),
    }
}
