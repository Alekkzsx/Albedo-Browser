use std::collections::HashMap;

use super::lexer::{
    HtmlLexer, HtmlToken as RawHtmlToken,
    LexerError as RawLexerError, LexerErrorKind as RawLexerErrorKind,
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
pub enum HtmlToken {
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

pub struct HtmlTokenizer<'a> {
    pub lexer: HtmlLexer<'a>,
    errors: Vec<TokenizerError>,
}

impl<'a> HtmlTokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            lexer: HtmlLexer::new(input),
            errors: Vec::new(),
        }
    }

    pub fn set_raw_text_tag(&mut self, tag: Option<String>) {
        self.lexer.set_raw_text_tag(tag);
    }

    pub fn errors(&self) -> &[TokenizerError] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<TokenizerError> {
        std::mem::take(&mut self.errors)
    }

    pub fn next_token(&mut self) -> HtmlToken {
        loop {
            let raw = self.lexer.next_token();
            self.collect_lexer_errors();

            if let Some(token) = self.convert_raw_token(raw) {
                return token;
            }
        }
    }

    fn collect_lexer_errors(&mut self) {
        let lexer_errors = self.lexer.take_errors();
        self.errors
            .extend(lexer_errors.into_iter().map(map_lexer_error));
    }

    pub fn convert_raw_token(&mut self, raw: RawHtmlToken) -> Option<HtmlToken> {
        match raw {
            RawHtmlToken::StartTag(tag) => {
                let mut attributes = HashMap::new();
                for (name, value) in tag.attributes {
                    attributes.insert(name.to_ascii_lowercase(), value);
                }
                Some(HtmlToken::StartTag(StartTagToken {
                    name: tag.name.to_ascii_lowercase(),
                    attributes,
                    self_closing: tag.self_closing,
                }))
            }
            RawHtmlToken::EndTag(name) => Some(HtmlToken::EndTag(EndTagToken {
                name: name.to_ascii_lowercase(),
            })),
            RawHtmlToken::Character(data) => {
                if data.is_empty() {
                    return None;
                }
                Some(HtmlToken::Character(CharacterToken { data }))
            }
            RawHtmlToken::Comment(data) => Some(HtmlToken::Comment(CommentToken { data })),
            RawHtmlToken::Doctype(dt) => Some(HtmlToken::Doctype(DoctypeToken {
                name: dt.name.map(|name| name.to_ascii_lowercase()),
                public_id: dt.public_id,
                system_id: dt.system_id,
                force_quirks: dt.force_quirks,
            }))
            .and_then(|token| {
                let HtmlToken::Doctype(ref doc) = token else {
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
                        1, 1, // Tokenizer generic errors use placeholder for now or we could pass pos
                    ));
                    None
                } else {
                    Some(token)
                }
            }),
            RawHtmlToken::Eof => Some(HtmlToken::Eof),
        }
    }
}

fn map_lexer_error(err: RawLexerError) -> TokenizerError {
    let (kind, code) = match err.kind {
        RawLexerErrorKind::UnexpectedNullCharacter => (TokenizerErrorKind::NullCharacter, "TOK004"),
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
    use super::{HtmlToken, HtmlTokenizer, TokenizerErrorKind, TokenizerErrorSource};

    #[test]
    fn converts_raw_tokens_into_typed_tokens() {
        let mut tokenizer = HtmlTokenizer::new(r#"<div class="hero">Hello</div>"#);

        let token = tokenizer.next_token();
        let HtmlToken::StartTag(tag) = token else {
            panic!("expected start tag");
        };
        assert_eq!(tag.name, "div");
        assert_eq!(tag.attributes.get("class"), Some(&"hero".to_string()));

        let token = tokenizer.next_token();
        let HtmlToken::Character(text) = token else {
            panic!("expected character token");
        };
        assert_eq!(text.data, "Hello");

        let token = tokenizer.next_token();
        let HtmlToken::EndTag(tag) = token else {
            panic!("expected end tag");
        };
        assert_eq!(tag.name, "div");
    }

    #[test]
    fn ignores_invalid_doctype_with_recovery() {
        let mut tokenizer = HtmlTokenizer::new("<!DOCTYPE><p>ok</p>");

        let token = tokenizer.next_token();
        let HtmlToken::StartTag(tag) = token else {
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
        let token = tokenizer.next_token();

        let HtmlToken::Character(text) = token else {
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
        let _ = tokenizer.next_token();

        assert!(tokenizer.errors().iter().any(|err| {
            err.source == TokenizerErrorSource::Lexer
                && err.kind == TokenizerErrorKind::LexerParseError
        }));
    }
}
