use std::collections::HashMap;

use super::lexer::{
    DoctypeToken as RawDoctypeToken, HtmlLexer, HtmlToken as RawHtmlToken,
    LexerError as RawLexerError, LexerErrorKind as RawLexerErrorKind,
    StartTagToken as RawStartTagToken,
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
}

impl TokenizerError {
    fn new(
        source: TokenizerErrorSource,
        kind: TokenizerErrorKind,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            source,
            kind,
            code,
            message: message.into(),
        }
    }
}

pub struct HtmlTokenizer<'a> {
    lexer: HtmlLexer<'a>,
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
            RawHtmlToken::StartTag(tag) => self.convert_start_tag(tag),
            RawHtmlToken::EndTag(name) => self.convert_end_tag(name),
            RawHtmlToken::Character(data) => self.convert_character(data),
            RawHtmlToken::Comment(data) => Some(HtmlToken::Comment(CommentToken { data })),
            RawHtmlToken::Doctype(dt) => self.convert_doctype(dt),
            RawHtmlToken::Eof => Some(HtmlToken::Eof),
        }
    }

    fn push_tokenizer_error(
        &mut self,
        kind: TokenizerErrorKind,
        code: &'static str,
        message: impl Into<String>,
    ) {
        self.errors.push(TokenizerError::new(
            TokenizerErrorSource::Tokenizer,
            kind,
            code,
            message,
        ));
    }

    fn convert_start_tag(&mut self, tag: RawStartTagToken) -> Option<HtmlToken> {
        if !is_valid_tag_name(&tag.name) {
            self.push_tokenizer_error(
                TokenizerErrorKind::InvalidStartTagName,
                "TOK001",
                format!("invalid start tag name: '{}'", tag.name),
            );
            return None;
        }

        let mut attributes = HashMap::new();
        for (name, value) in tag.attributes {
            if name.trim().is_empty() {
                self.push_tokenizer_error(
                    TokenizerErrorKind::EmptyAttributeName,
                    "TOK002",
                    "attribute name is empty",
                );
                continue;
            }
            attributes.insert(name, value);
        }

        Some(HtmlToken::StartTag(StartTagToken {
            name: tag.name.to_ascii_lowercase(),
            attributes,
            self_closing: tag.self_closing,
        }))
    }

    fn convert_end_tag(&mut self, name: String) -> Option<HtmlToken> {
        if !is_valid_tag_name(&name) {
            self.push_tokenizer_error(
                TokenizerErrorKind::InvalidEndTagName,
                "TOK003",
                format!("invalid end tag name: '{}'", name),
            );
            return None;
        }

        Some(HtmlToken::EndTag(EndTagToken {
            name: name.to_ascii_lowercase(),
        }))
    }

    fn convert_character(&mut self, data: String) -> Option<HtmlToken> {
        if data.is_empty() {
            return None;
        }

        let mut had_null = false;
        let mut normalized = String::with_capacity(data.len());
        for ch in data.chars() {
            if ch == '\0' {
                had_null = true;
                normalized.push('\u{FFFD}');
            } else {
                normalized.push(ch);
            }
        }

        if had_null {
            self.push_tokenizer_error(
                TokenizerErrorKind::NullCharacter,
                "TOK004",
                "null character replaced with U+FFFD",
            );
        }

        Some(HtmlToken::Character(CharacterToken { data: normalized }))
    }

    fn convert_doctype(&mut self, dt: RawDoctypeToken) -> Option<HtmlToken> {
        if dt.force_quirks || dt.name.is_none() {
            self.push_tokenizer_error(
                TokenizerErrorKind::InvalidDoctype,
                "TOK005",
                "invalid DOCTYPE token ignored",
            );
            return None;
        }

        Some(HtmlToken::Doctype(DoctypeToken {
            name: dt.name.map(|name| name.to_ascii_lowercase()),
            public_id: dt.public_id,
            system_id: dt.system_id,
            force_quirks: dt.force_quirks,
        }))
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
    )
}

fn is_valid_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-' | '.'))
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
