use super::*;
use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};


pub(crate) fn map_html5gum_token(token: Token) -> Option<HtmlTokenKind> {
    match token {
        Token::StartTag(tag) => Some(HtmlTokenKind::StartTag(StartTagToken {
            name: String::from_utf8_lossy(&tag.name).to_ascii_lowercase(),
            attributes: tag
                .attributes
                .into_iter()
                .map(|(name, value)| {
                    (
                        String::from_utf8_lossy(&name).to_ascii_lowercase(),
                        String::from_utf8_lossy(&value).to_string(),
                    )
                })
                .collect(),
            self_closing: tag.self_closing,
        })),
        Token::EndTag(tag) => Some(HtmlTokenKind::EndTag(EndTagToken {
            name: String::from_utf8_lossy(&tag.name).to_ascii_lowercase(),
        })),
        Token::String(text) => Some(HtmlTokenKind::Character(CharacterToken {
            data: decode_text_token(&text),
        })),
        Token::Comment(comment) => {
            let data = String::from_utf8_lossy(&comment).to_string();
            if let Some(cdata) = decode_legacy_cdata_comment(&data) {
                Some(HtmlTokenKind::Character(CharacterToken { data: cdata }))
            } else {
                Some(HtmlTokenKind::Comment(CommentToken { data }))
            }
        }
        Token::Doctype(dt) => Some(HtmlTokenKind::Doctype(DoctypeToken {
            name: Some(String::from_utf8_lossy(&dt.name).to_ascii_lowercase()),
            public_id: dt
                .public_identifier
                .map(|value| String::from_utf8_lossy(&value).to_string()),
            system_id: dt
                .system_identifier
                .map(|value| String::from_utf8_lossy(&value).to_string()),
            force_quirks: dt.force_quirks,
        })),
        Token::Error(_) => None,
    }
}
