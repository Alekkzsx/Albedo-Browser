use std::collections::HashMap;

use super::lexer::{
    HtmlLexer, HtmlToken as RawHtmlToken, HtmlTokenKind as RawHtmlTokenKind,
    LexerError as RawLexerError, LexerErrorKind as RawLexerErrorKind,
    LexerState,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartTagToken {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub self_closing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndTagToken {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterToken {
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentToken {
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctypeToken {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub force_quirks: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlToken {
    pub kind: HtmlTokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlTokenKind {
    StartTag(StartTagToken),
    EndTag(EndTagToken),
    Character(CharacterToken),
    Comment(CommentToken),
    Doctype(DoctypeToken),
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerErrorSource {
    Lexer,
    Tokenizer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerErrorKind {
    InvalidStartTagName,
    InvalidEndTagName,
    InvalidDoctype,
    NullCharacter,
    EmptyAttributeName,
    EofInTag,
    EofInComment,
    EofInDoctype,
    NestedComment,
    CdataSectionOutsideForeignContent,
    MissingSemicolonAfterCharacterReference,
    LexerParseError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenizerError {
    pub source: TokenizerErrorSource,
    pub kind: TokenizerErrorKind,
    pub code: &'static str,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl TokenizerError {
    fn new(
        source: TokenizerErrorSource,
        kind: TokenizerErrorKind,
        code: &'static str,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            source,
            kind,
            code,
            message: message.into(),
            line,
            column,
        }
    }
}

pub struct HtmlTokenizer {
    pub lexer: HtmlLexer,
    errors: Vec<TokenizerError>,
}

impl HtmlTokenizer {
    pub fn new(input: &str) -> Self {
        // Non-streaming constructor: the full input is already available.
        // Signal end-of-input so the lexer can flush trailing text and emit EOF.
        let mut lexer = HtmlLexer::new(input);
        lexer.end();
        Self {
            lexer,
            errors: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            lexer: HtmlLexer::empty(),
            errors: Vec::new(),
        }
    }

    pub fn feed(&mut self, input: &str) {
        self.lexer.feed(input);
    }

    pub fn end(&mut self) {
        self.lexer.end();
    }

    pub fn set_raw_text_tag(&mut self, tag: Option<String>) {
        self.lexer.set_raw_text_tag(tag);
    }

    pub fn set_state(&mut self, state: LexerState) {
        self.lexer.set_state(state);
    }

    pub fn set_cdata_allowed(&mut self, allowed: bool) {
        self.lexer.set_cdata_allowed(allowed);
    }

    pub fn errors(&self) -> &[TokenizerError] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<TokenizerError> {
        std::mem::take(&mut self.errors)
    }

    pub fn next_token(&mut self) -> Option<HtmlToken> {
        // Guard against degenerate inputs where the lexer produces an unbounded
        // stream of skipped tokens (e.g., repeated empty characters on a stuck state).
        // After 65535 iterations without a real token, emit EOF to terminate parsing.
        let mut guard = 0u32;
        loop {
            guard += 1;
            if guard > 65535 {
                // SAFETY: defensive escape hatch — should never trigger on valid paths.
                return Some(HtmlToken {
                    kind: HtmlTokenKind::Eof,
                    line: 0,
                    column: 0,
                });
            }
            let raw = self.lexer.next_token()?;
            self.collect_lexer_errors();

            if let Some(token) = self.convert_raw_token(raw) {
                return Some(token);
            }
        }
    }

    fn collect_lexer_errors(&mut self) {
        let lexer_errors = self.lexer.take_errors();
        self.errors
            .extend(lexer_errors.into_iter().map(map_lexer_error));
    }

    pub fn convert_raw_token(&mut self, raw: RawHtmlToken) -> Option<HtmlToken> {
        let (line, column) = (raw.line, raw.column);
        match raw.kind {
            RawHtmlTokenKind::StartTag(tag) => {
                let mut attributes = HashMap::new();
                for (name, value) in tag.attributes {
                    attributes.insert(name.to_ascii_lowercase(), value);
                }
                Some(HtmlToken {
                    kind: HtmlTokenKind::StartTag(StartTagToken {
                        name: tag.name.to_ascii_lowercase(),
                        attributes,
                        self_closing: tag.self_closing,
                    }),
                    line,
                    column,
                })
            }
            RawHtmlTokenKind::EndTag(name) => Some(HtmlToken {
                kind: HtmlTokenKind::EndTag(EndTagToken {
                    name: name.to_ascii_lowercase(),
                }),
                line,
                column,
            }),
            RawHtmlTokenKind::Character(data) => {
                if data.is_empty() {
                    return None;
                }
                Some(HtmlToken {
                    kind: HtmlTokenKind::Character(CharacterToken { data }),
                    line,
                    column,
                })
            }
            RawHtmlTokenKind::Comment(data) => Some(HtmlToken {
                kind: HtmlTokenKind::Comment(CommentToken { data }),
                line,
                column,
            }),
            RawHtmlTokenKind::Doctype(dt) => Some(HtmlToken {
                kind: HtmlTokenKind::Doctype(DoctypeToken {
                    name: dt.name.map(|name| name.to_ascii_lowercase()),
                    public_id: dt.public_id,
                    system_id: dt.system_id,
                    force_quirks: dt.force_quirks,
                }),
                line,
                column,
            })
            .and_then(|token| {
                let HtmlTokenKind::Doctype(ref doc) = token.kind else {
                    return Some(token);
                };

                // Recovery: malformed `<!DOCTYPE>` without name should raise parse
                // error and be ignored in the output stream.
                if doc.name.is_none() && doc.public_id.is_none() && doc.system_id.is_none() {
                    self.errors.push(TokenizerError::new(
                        TokenizerErrorSource::Tokenizer,
                        TokenizerErrorKind::InvalidDoctype,
                        "TOK003",
                        "invalid doctype without name",
                        line, column, // Correct position!
                    ));
                    None
                } else {
                    Some(token)
                }
            }),
            RawHtmlTokenKind::Eof => Some(HtmlToken {
                kind: HtmlTokenKind::Eof,
                line,
                column,
            }),
        }
    }
}

fn map_lexer_error(err: RawLexerError) -> TokenizerError {
    let (kind, code) = match err.kind {
        RawLexerErrorKind::UnexpectedNullCharacter => (TokenizerErrorKind::NullCharacter, "TOK004"),
        RawLexerErrorKind::EofInTag => (TokenizerErrorKind::EofInTag, "LEX001"),
        RawLexerErrorKind::EofInComment => (TokenizerErrorKind::EofInComment, "LEX001"),
        RawLexerErrorKind::EofInDoctype => (TokenizerErrorKind::EofInDoctype, "LEX001"),
        RawLexerErrorKind::NestedComment => (TokenizerErrorKind::NestedComment, "LEX001"),
        RawLexerErrorKind::CdataSectionOutsideForeignContent => {
            (TokenizerErrorKind::CdataSectionOutsideForeignContent, "LEX001")
        }
        RawLexerErrorKind::MissingSemicolonAfterCharacterReference => {
            (TokenizerErrorKind::MissingSemicolonAfterCharacterReference, "LEX001")
        }
        _ => (TokenizerErrorKind::LexerParseError, "LEX001"),
    };

    TokenizerError::new(
        TokenizerErrorSource::Lexer,
        kind,
        code,
        err.message,
        err.line,
        err.column,
    )
}


#[cfg(test)]
mod tests {
    use super::{HtmlTokenKind, HtmlTokenizer, TokenizerErrorKind, TokenizerErrorSource};

    #[test]
    fn converts_raw_tokens_into_typed_tokens() {
        let mut tokenizer = HtmlTokenizer::new(r#"<div class="hero">Hello</div>"#);

        let token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::StartTag(tag) = token.kind else {
            panic!("expected start tag");
        };
        assert_eq!(tag.name, "div");
        assert_eq!(tag.attributes.get("class"), Some(&"hero".to_string()));

        let token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::Character(text) = token.kind else {
            panic!("expected character token");
        };
        assert_eq!(text.data, "Hello");

        let token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::EndTag(tag) = token.kind else {
            panic!("expected end tag");
        };
        assert_eq!(tag.name, "div");
    }

    #[test]
    fn ignores_invalid_doctype_with_recovery() {
        let mut tokenizer = HtmlTokenizer::new("<!DOCTYPE><p>ok</p>");

        let token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::StartTag(tag) = token.kind else {
            panic!("expected start tag after invalid doctype");
        };
        assert_eq!(tag.name, "p");

        assert!(tokenizer
            .errors()
            .iter()
            .any(|err| err.kind == TokenizerErrorKind::InvalidDoctype));
    }

    #[test]
    fn replaces_null_characters_and_keeps_stream_valid() {
        let mut tokenizer = HtmlTokenizer::new("a\0b");
        let token = tokenizer.next_token().unwrap();

        let HtmlTokenKind::Character(text) = token.kind else {
            panic!("expected character token");
        };
        assert_eq!(text.data, "a\u{FFFD}b");
        assert!(tokenizer
            .errors()
            .iter()
            .any(|err| err.kind == TokenizerErrorKind::NullCharacter));
    }

    #[test]
    fn maps_lexer_errors_with_source() {
        let mut tokenizer = HtmlTokenizer::new("</>");
        let _ = tokenizer.next_token().unwrap();

        assert!(tokenizer.errors().iter().any(|err| {
            err.source == TokenizerErrorSource::Lexer
                && err.kind == TokenizerErrorKind::LexerParseError
        }));
    }

    #[test]
    fn treats_noscript_contents_as_raw_text() {
        let mut tokenizer = HtmlTokenizer::new("<noscript><style>.x{}</style></noscript>");

        let start_token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::StartTag(start) = start_token.kind else {
            panic!("expected noscript start tag");
        };
        assert_eq!(start.name, "noscript");

        let text_token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::Character(text) = text_token.kind else {
            panic!("expected noscript raw text");
        };
        assert_eq!(text.data, "<style>.x{}</style>");

        let end_token = tokenizer.next_token().unwrap();
        let HtmlTokenKind::EndTag(end) = end_token.kind else {
            panic!("expected noscript end tag");
        };
        assert_eq!(end.name, "noscript");
    }
}
