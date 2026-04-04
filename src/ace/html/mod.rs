#![deny(warnings)]

use std::collections::HashMap;

pub mod lexer;
pub mod entities;
pub mod tokenizer;
pub mod tree_builder;
pub mod preload_scanner;
pub mod encoding;
pub mod arena;
pub mod interner;
pub mod small_attr_map;
pub mod metrics;
pub mod streaming;
pub mod simd;
pub mod integrated_parser;
pub mod tests;

pub use lexer::{
    DoctypeToken as RawDoctypeToken, HtmlLexer, HtmlToken as RawHtmlToken,
    LexerError, LexerErrorKind, StartTagToken as RawStartTagToken,
};
pub use tokenizer::{
    CharacterToken, CommentToken, DoctypeToken, EndTagToken, HtmlToken, HtmlTokenKind, HtmlTokenizer,
    StartTagToken, TokenizerError, TokenizerErrorKind, TokenizerErrorSource,
};
pub use tree_builder::{
    build_document, build_document_with_errors, build_document_with_errors_and_options,
    build_fragment, build_fragment_with_context_and_options, build_fragment_with_errors,
    build_fragment_with_errors_and_options, InsertionMode, TreeBuildOutput, TreeBuilderError,
    TreeBuilderErrorKind, TreeBuilderErrorSource,
};
pub use preload_scanner::{PreloadRequest, PreloadResourceType, PreloadScanner};
pub use encoding::{
    decode_bytes, decode_html_bytes, detect_encoding_from_bom, DecodedInput, Encoding,
    EncodingDetectionResult, EncodingDetector, EncodingPrescanner, EncodingSource,
    extract_charset_from_meta, parse_content_type_header,
};
pub use arena::{NodeArena, NodeId};
pub use interner::{StringInterner, StringId};
pub use small_attr_map::SmallAttributeMap;
pub use metrics::{ParserMetrics, ParserStats};
pub use streaming::{StreamingHtmlParser, StreamingState};
pub use simd::{
    fast_entity_lookup, decode_numeric_entity, simd_find_byte,
    normalize_whitespace_simd, has_simd_support, get_optimization_level,
};
pub use integrated_parser::{
    parse_html_integrated, parse_html_integrated_from_bytes,
    parse_html_integrated_from_bytes_with_options, parse_html_integrated_with_options,
    IntegratedTreeBuilder, ParseResult, ParserStats as IntegratedParserStats,
};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserOptions {
    pub scripting_enabled: bool,
    pub base_url: Option<String>,
    pub source_url: Option<String>,
    pub encoding_hint: Option<Encoding>,
    pub track_positions: bool,
    pub collect_preloads: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            scripting_enabled: true,
            base_url: None,
            source_url: None,
            encoding_hint: None,
            track_positions: true,
            collect_preloads: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentContext {
    pub tag_name: String,
    pub namespace: Namespace,
    pub scripting_enabled: bool,
}

impl FragmentContext {
    pub fn new(tag_name: impl Into<String>) -> Self {
        Self {
            tag_name: tag_name.into().to_ascii_lowercase(),
            namespace: Namespace::Html,
            scripting_enabled: true,
        }
    }

    pub fn with_namespace(mut self, namespace: Namespace) -> Self {
        self.namespace = namespace;
        self
    }

    pub fn with_scripting(mut self, scripting_enabled: bool) -> Self {
        self.scripting_enabled = scripting_enabled;
        self
    }
}

pub fn parse_document(input: &str) -> HtmlDocument {
    parse_document_with_options(input, &ParserOptions::default())
}

pub fn parse_document_with_options(input: &str, options: &ParserOptions) -> HtmlDocument {
    build_document_with_errors_and_options(input, options).document
}

pub fn parse_document_from_bytes(bytes: &[u8]) -> Result<HtmlDocument, String> {
    parse_document_from_bytes_with_options(bytes, None, &ParserOptions::default())
}

pub fn parse_document_from_bytes_with_options(
    bytes: &[u8],
    http_header: Option<&str>,
    options: &ParserOptions,
) -> Result<HtmlDocument, String> {
    Ok(parse_document_from_bytes_with_errors_and_options(bytes, http_header, options)?.document)
}

pub fn parse_document_with_errors(input: &str) -> TreeBuildOutput {
    parse_document_with_errors_and_options(input, &ParserOptions::default())
}

pub fn parse_document_with_errors_and_options(input: &str, options: &ParserOptions) -> TreeBuildOutput {
    build_document_with_errors_and_options(input, options)
}

pub fn parse_document_from_bytes_with_errors_and_options(
    bytes: &[u8],
    http_header: Option<&str>,
    options: &ParserOptions,
) -> Result<TreeBuildOutput, String> {
    let decoded = decode_html_bytes(bytes, http_header, options.encoding_hint)?;
    Ok(build_document_with_errors_and_options(&decoded.content, options))
}

pub fn parse_fragment(input: &str, context: Option<&str>) -> Vec<HtmlNode> {
    parse_fragment_with_context(
        input,
        context.map(FragmentContext::new).as_ref(),
        &ParserOptions::default(),
    )
}

pub fn parse_fragment_with_context(
    input: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> Vec<HtmlNode> {
    build_fragment_with_context_and_options(input, context, options).document.children
}

pub fn parse_fragment_with_errors_and_context(
    input: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> TreeBuildOutput {
    build_fragment_with_context_and_options(input, context, options)
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
    UnexpectedCharacter,
    UnexpectedEof,
    NestedHead,
    FosterParenting,
    AdoptionAgency,
    TokenizerError,
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
            Self::UnexpectedCharacter => "unexpected-character",
            Self::UnexpectedEof => "unexpected-eof",
            Self::NestedHead => "nested-head",
            Self::FosterParenting => "foster-parenting",
            Self::AdoptionAgency => "adoption-agency",
            Self::TokenizerError => "tokenizer-error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub code: AceHtmlErrorCode,
    pub source: ParseErrorSource,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorSource {
    Lexer,
    Tokenizer,
    TreeBuilder,
}

impl TreeBuildOutput {
    pub fn parse_errors(&self) -> Vec<ParseError> {
        self.errors
            .iter()
            .map(|error| ParseError {
                code: match error.kind {
                    TreeBuilderErrorKind::UnexpectedToken => AceHtmlErrorCode::UnexpectedToken,
                    TreeBuilderErrorKind::UnexpectedEndTag => AceHtmlErrorCode::UnexpectedEndTag,
                    TreeBuilderErrorKind::UnexpectedDoctype => AceHtmlErrorCode::UnexpectedDoctype,
                    TreeBuilderErrorKind::UnexpectedCharacter => AceHtmlErrorCode::UnexpectedCharacter,
                    TreeBuilderErrorKind::NestedHead => AceHtmlErrorCode::NestedHead,
                    TreeBuilderErrorKind::FosterParenting => AceHtmlErrorCode::FosterParenting,
                    TreeBuilderErrorKind::AdoptionAgency => AceHtmlErrorCode::AdoptionAgency,
                    TreeBuilderErrorKind::TokenizerError => AceHtmlErrorCode::TokenizerError,
                    TreeBuilderErrorKind::UnexpectedEof => AceHtmlErrorCode::UnexpectedEof,
                },
                source: match error.source {
                    TreeBuilderErrorSource::Lexer => ParseErrorSource::Lexer,
                    TreeBuilderErrorSource::Tokenizer => ParseErrorSource::Tokenizer,
                    TreeBuilderErrorSource::TreeBuilder => ParseErrorSource::TreeBuilder,
                },
                message: error.message.clone(),
                line: error.line,
                column: error.column,
            })
            .collect()
    }
}
