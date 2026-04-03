#![deny(warnings)]

use std::collections::HashMap;

pub mod lexer;
pub mod entities;
pub mod tokenizer;
pub mod tree_builder;
pub mod preload_scanner;
pub mod tests;

pub use lexer::{
    DoctypeToken as RawDoctypeToken, HtmlLexer, HtmlToken as RawHtmlToken,
    LexerError, LexerErrorKind, StartTagToken as RawStartTagToken,
};
pub use tokenizer::{
    CharacterToken, CommentToken, DoctypeToken, EndTagToken, HtmlToken, HtmlTokenizer,
    StartTagToken, TokenizerError, TokenizerErrorKind, TokenizerErrorSource,
};
pub use tree_builder::{
    build_document, build_document_with_errors, build_fragment, build_fragment_with_errors,
    InsertionMode, TreeBuildOutput, TreeBuilderError, TreeBuilderErrorKind,
    TreeBuilderErrorSource,
};
pub use preload_scanner::{PreloadRequest, PreloadResourceType, PreloadScanner};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlDocument {
    pub doctype: Option<DoctypeToken>,
    pub children: Vec<HtmlNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlNode {
    Element(HtmlElement),
    Text(String),
    Comment(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

impl Default for Namespace {
    fn default() -> Self {
        Self::Html
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlElement {
    pub tag: String,
    pub namespace: Namespace,
    pub attributes: HashMap<String, String>,
    pub children: Vec<HtmlNode>,
    /// Slot assignment for Shadow DOM (slot="..." attribute)
    pub slot_name: Option<String>,
    /// Is attribute for custom elements (is="x-button")
    pub is_value: Option<String>,
    /// Indicates if this element is a shadow root host
    pub shadow_root_mode: Option<ShadowRootMode>,
    /// Shadow root content (for declarative shadow DOM)
    pub shadow_root: Option<Box<HtmlDocument>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowRootMode {
    Open,
    Closed,
}

impl Default for ShadowRootMode {
    fn default() -> Self {
        Self::Open
    }
}

impl HtmlElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            namespace: Namespace::Html,
            attributes: HashMap::new(),
            children: Vec::new(),
            slot_name: None,
            is_value: None,
            shadow_root_mode: None,
            shadow_root: None,
        }
    }

    pub fn with_namespace(tag: impl Into<String>, ns: Namespace) -> Self {
        Self {
            tag: tag.into(),
            namespace: ns,
            attributes: HashMap::new(),
            children: Vec::new(),
            slot_name: None,
            is_value: None,
            shadow_root_mode: None,
            shadow_root: None,
        }
    }
}

pub fn parse_document(input: &str) -> HtmlDocument {
    build_document(input)
}

pub fn parse_document_with_errors(input: &str) -> TreeBuildOutput {
    build_fragment_with_errors(input, None)
}

pub fn parse_fragment(input: &str, context: Option<&str>) -> Vec<HtmlNode> {
    build_fragment(input, context)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AceHtmlErrorCode {
    // Lexer Errors
    AbruptClosingOfEmptyComment,
    AbsenceOfDigitsInNumericCharacterReference,
    AmbiguousAmpersand,
    CharacterReferenceOutsideUnicodeRange,
    CdataSectionOutsideForeignContent,
    DuplicateAttribute,
    EndTagWithAttributes,
    EndTagWithTrailingSolidus,
    EofBeforeTagName,
    EofInCdata,
    EofInComment,
    EofInDoctype,
    EofInScriptHtmlCommentLikeText,
    EofInTag,
    IncorrectlyOpenedComment,
    InvalidCharacterSequenceAfterDoctypeName,
    InvalidFirstCharacterOfTagName,
    MissingAttributeValue,
    MissingDoctypeName,
    MissingDoctypePublicIdentifier,
    MissingDoctypeSystemIdentifier,
    MissingEndTagName,
    MissingQuoteBeforeDoctypePublicIdentifier,
    MissingQuoteBeforeDoctypeSystemIdentifier,
    MissingSemicolonAfterCharacterReference,
    MissingWhitespaceAfterDoctypePublicKeyword,
    MissingWhitespaceAfterDoctypeSystemKeyword,
    MissingWhitespaceBeforeDoctypeName,
    MissingWhitespaceBetweenAttributes,
    MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
    NestedComment,
    NoncharacterCharacterReference,
    NoncharacterInInputStream,
    NullCharacterReference,
    SurrogateCharacterReference,
    SurrogateInInputStream,
    UnexpectedCharacterInAttributeName,
    UnexpectedCharacterInUnquotedAttributeValue,
    UnexpectedEqualsSignBeforeAttributeName,
    UnexpectedNullCharacter,
    UnexpectedQuestionMarkInsteadOfTagName,
    UnexpectedSolidusInTag,
    UnknownNamedCharacterReference,

    // Tree Builder Errors
    UnexpectedDoctype,
    UnexpectedToken,
    UnexpectedEndTag,
    FosterParenting,
    AdoptionAgency,
}

impl AceHtmlErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AbruptClosingOfEmptyComment => "abrupt-closing-of-empty-comment",
            Self::AbsenceOfDigitsInNumericCharacterReference => "absence-of-digits-in-numeric-character-reference",
            Self::AmbiguousAmpersand => "ambiguous-ampersand",
            Self::CharacterReferenceOutsideUnicodeRange => "character-reference-outside-unicode-range",
            Self::CdataSectionOutsideForeignContent => "cdata-section-outside-foreign-content",
            Self::DuplicateAttribute => "duplicate-attribute",
            Self::EndTagWithAttributes => "end-tag-with-attributes",
            Self::EndTagWithTrailingSolidus => "end-tag-with-trailing-solidus",
            Self::EofBeforeTagName => "eof-before-tag-name",
            Self::EofInCdata => "eof-in-cdata",
            Self::EofInComment => "eof-in-comment",
            Self::EofInDoctype => "eof-in-doctype",
            Self::EofInScriptHtmlCommentLikeText => "eof-in-script-html-comment-like-text",
            Self::EofInTag => "eof-in-tag",
            Self::IncorrectlyOpenedComment => "incorrectly-opened-comment",
            Self::InvalidCharacterSequenceAfterDoctypeName => "invalid-character-sequence-after-doctype-name",
            Self::InvalidFirstCharacterOfTagName => "invalid-first-character-of-tag-name",
            Self::MissingAttributeValue => "missing-attribute-value",
            Self::MissingDoctypeName => "missing-doctype-name",
            Self::MissingDoctypePublicIdentifier => "missing-doctype-public-identifier",
            Self::MissingDoctypeSystemIdentifier => "missing-doctype-system-identifier",
            Self::MissingEndTagName => "missing-end-tag-name",
            Self::MissingQuoteBeforeDoctypePublicIdentifier => "missing-quote-before-doctype-public-identifier",
            Self::MissingQuoteBeforeDoctypeSystemIdentifier => "missing-quote-before-doctype-system-identifier",
            Self::MissingSemicolonAfterCharacterReference => "missing-semicolon-after-character-reference",
            Self::MissingWhitespaceAfterDoctypePublicKeyword => "missing-whitespace-after-doctype-public-keyword",
            Self::MissingWhitespaceAfterDoctypeSystemKeyword => "missing-whitespace-after-doctype-system-keyword",
            Self::MissingWhitespaceBeforeDoctypeName => "missing-whitespace-before-doctype-name",
            Self::MissingWhitespaceBetweenAttributes => "missing-whitespace-between-attributes",
            Self::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers => "missing-whitespace-between-doctype-public-and-system-identifiers",
            Self::NestedComment => "nested-comment",
            Self::NoncharacterCharacterReference => "noncharacter-character-reference",
            Self::NoncharacterInInputStream => "noncharacter-in-input-stream",
            Self::NullCharacterReference => "null-character-reference",
            Self::SurrogateCharacterReference => "surrogate-character-reference",
            Self::SurrogateInInputStream => "surrogate-in-input-stream",
            Self::UnexpectedCharacterInAttributeName => "unexpected-character-in-attribute-name",
            Self::UnexpectedCharacterInUnquotedAttributeValue => "unexpected-character-in-unquoted-attribute-value",
            Self::UnexpectedEqualsSignBeforeAttributeName => "unexpected-equals-sign-before-attribute-name",
            Self::UnexpectedNullCharacter => "unexpected-null-character",
            Self::UnexpectedQuestionMarkInsteadOfTagName => "unexpected-question-mark-instead-of-tag-name",
            Self::UnexpectedSolidusInTag => "unexpected-solidus-in-tag",
            Self::UnknownNamedCharacterReference => "unknown-named-character-reference",
            Self::UnexpectedDoctype => "unexpected-doctype",
            Self::UnexpectedToken => "unexpected-token",
            Self::UnexpectedEndTag => "unexpected-end-tag",
            Self::FosterParenting => "foster-parenting",
            Self::AdoptionAgency => "adoption-agency",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub code: AceHtmlErrorCode,
    pub message: String,
    pub line: usize,
    pub column: usize,
}
