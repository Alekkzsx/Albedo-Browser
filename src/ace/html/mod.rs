//! ACE HTML Parser - A high-performance, WHATWG-compliant HTML5 parser.
//!
//! This module provides a complete HTML5 parsing implementation following the
//! [WHATWG HTML Living Standard](https://html.spec.whatwg.org/). It includes
//! a lexer, tokenizer, and tree builder that work together to parse HTML documents
//! and fragments into a DOM-like tree structure.
//!
//! # Features
//!
//! - **WHATWG Conformance**: Implements all lexer states, insertion modes, and algorithms
//! - **High Performance**: SIMD optimizations, zero-copy string handling, and speculative parsing
//! - **Streaming Support**: Incremental parsing with low latency
//! - **Error Recovery**: Robust error handling following the HTML5 specification
//! - **Preload Scanning**: Early resource discovery for improved page load performance
//! - **Encoding Detection**: Automatic character encoding detection and conversion
//!
//! # Quick Start
//!
//! Parse a complete HTML document:
//!
//! ```
//! use ace::html::parse_document;
//!
//! let html = r#"
//!     <!DOCTYPE html>
//!     <html>
//!         <head><title>Example</title></head>
//!         <body><h1>Hello, World!</h1></body>
//!     </html>
//! "#;
//!
//! let document = parse_document(html);
//! assert!(!document.children.is_empty());
//! ```
//!
//! Parse an HTML fragment:
//!
//! ```
//! use ace::html::parse_fragment;
//!
//! let html = "<div><p>Hello</p></div>";
//! let nodes = parse_fragment(html, Some("body"));
//! assert_eq!(nodes.len(), 1);
//! ```
//!
//! Parse with error reporting:
//!
//! ```
//! use ace::html::parse_document_with_errors;
//!
//! let html = "<div><p>Unclosed paragraph</div>";
//! let output = parse_document_with_errors(html);
//! assert!(!output.errors.is_empty());
//! ```
//!
//! # Architecture
//!
//! The parser consists of three main components:
//!
//! 1. **Lexer** ([`HtmlLexer`]): Tokenizes raw HTML text into tokens
//! 2. **Tokenizer** ([`HtmlTokenizer`]): Processes lexer tokens and handles character references
//! 3. **Tree Builder** ([`build_document`]): Constructs the DOM tree from tokens
//!
//! # Performance Features
//!
//! - **SIMD Acceleration**: Uses AVX2/AVX-512 for whitespace detection and entity lookup
//! - **String Interning**: Reduces memory usage by deduplicating common strings
//! - **Arena Allocation**: Fast memory allocation with minimal overhead
//! - **Speculative Parsing**: Parallel tokenization for improved throughput
//! - **Preload Scanner**: Early resource discovery without full parsing
//!
//! # Modules
//!
//! - [`lexer`]: Low-level HTML tokenization
//! - [`tokenizer`]: High-level token processing
//! - [`tree_builder`]: DOM tree construction
//! - [`preload_scanner`]: Resource preload discovery
//! - [`encoding`]: Character encoding detection and conversion
//! - [`streaming`]: Incremental parsing support
//! - [`speculative`]: Parallel parsing support
//! - [`integrated_parser`]: Simplified parsing API with statistics

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
pub mod thread_pool;
pub mod speculative;

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
pub use thread_pool::{ThreadPool, BoundedChannel, SenderWithTimeout, ReceiverWithTimeout};
pub use integrated_parser::{
    parse_html_integrated, parse_html_integrated_from_bytes,
    parse_html_integrated_from_bytes_with_options, parse_html_integrated_with_options,
    IntegratedTreeBuilder, ParseResult, ParserStats as IntegratedParserStats,
};
pub use speculative::{
    parse_speculative, parse_document_speculative, SpeculativeResult,
    SpeculativeTokenizer, SpeculativeTreeBuilder,
};

/// Represents a complete HTML document.
///
/// An HTML document consists of an optional DOCTYPE declaration and a list of root-level nodes.
/// Typically, a well-formed document will have a single `<html>` element as its child.
///
/// # Examples
///
/// ```
/// use ace::html::parse_document;
///
/// let html = "<!DOCTYPE html><html><body>Content</body></html>";
/// let doc = parse_document(html);
///
/// assert!(doc.doctype.is_some());
/// assert!(!doc.children.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlDocument {
    /// The DOCTYPE declaration, if present.
    pub doctype: Option<DoctypeToken>,
    /// The root-level nodes of the document.
    pub children: Vec<HtmlNode>,
}

/// A node in the HTML document tree.
///
/// HTML nodes can be elements, text nodes, or comments. This enum provides
/// a unified representation for all node types in the document tree.
///
/// # Examples
///
/// ```
/// use ace::html::{HtmlNode, HtmlElement};
///
/// let element = HtmlNode::Element(HtmlElement::new("div"));
/// let text = HtmlNode::Text("Hello".to_string());
/// let comment = HtmlNode::Comment("TODO: fix this".to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlNode {
    /// An HTML element with tag name, attributes, and children.
    Element(HtmlElement),
    /// A text node containing character data.
    Text(String),
    /// A comment node.
    Comment(String),
}

/// XML namespace for HTML elements.
///
/// HTML5 supports three namespaces: HTML, SVG, and MathML. The parser automatically
/// switches namespaces when encountering foreign content elements.
///
/// # Examples
///
/// ```
/// use ace::html::Namespace;
///
/// let html_ns = Namespace::Html;
/// let svg_ns = Namespace::Svg;
/// let mathml_ns = Namespace::MathMl;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Namespace {
    /// HTML namespace (default).
    Html,
    /// SVG namespace for scalable vector graphics.
    Svg,
    /// MathML namespace for mathematical markup.
    MathMl,
}

impl Default for Namespace {
    fn default() -> Self {
        Self::Html
    }
}

/// An HTML element with tag name, attributes, and children.
///
/// Elements are the primary building blocks of HTML documents. Each element has a tag name,
/// optional attributes, and may contain child nodes. Elements can also have special properties
/// for Web Components (shadow DOM, slots) and custom elements.
///
/// # Examples
///
/// ```
/// use ace::html::{HtmlElement, Namespace};
///
/// // Create a simple div element
/// let div = HtmlElement::new("div");
///
/// // Create an SVG element
/// let svg = HtmlElement::with_namespace("svg", Namespace::Svg);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlElement {
    /// The tag name (e.g., "div", "p", "span").
    pub tag: String,
    /// The XML namespace of this element.
    pub namespace: Namespace,
    /// The element's attributes as key-value pairs.
    pub attributes: HashMap<String, String>,
    /// The element's child nodes.
    pub children: Vec<HtmlNode>,
    /// Slot assignment for Shadow DOM (slot="..." attribute).
    pub slot_name: Option<String>,
    /// Is attribute for custom elements (is="x-button").
    pub is_value: Option<String>,
    /// Indicates if this element is a shadow root host.
    pub shadow_root_mode: Option<ShadowRootMode>,
    /// Shadow root content (for declarative shadow DOM).
    pub shadow_root: Option<Box<HtmlDocument>>,
}

/// Shadow DOM attachment mode.
///
/// Determines whether the shadow root is accessible from JavaScript.
///
/// # Examples
///
/// ```
/// use ace::html::ShadowRootMode;
///
/// let open_mode = ShadowRootMode::Open;
/// let closed_mode = ShadowRootMode::Closed;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowRootMode {
    /// Shadow root is accessible via `element.shadowRoot`.
    Open,
    /// Shadow root is not accessible from JavaScript.
    Closed,
}

impl Default for ShadowRootMode {
    fn default() -> Self {
        Self::Open
    }
}

impl HtmlElement {
    /// Creates a new HTML element with the given tag name.
    ///
    /// The element is created in the HTML namespace with no attributes or children.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::HtmlElement;
    ///
    /// let div = HtmlElement::new("div");
    /// assert_eq!(div.tag, "div");
    /// assert!(div.attributes.is_empty());
    /// assert!(div.children.is_empty());
    /// ```
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

    /// Creates a new element with the given tag name and namespace.
    ///
    /// Use this constructor when creating SVG or MathML elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::{HtmlElement, Namespace};
    ///
    /// let svg = HtmlElement::with_namespace("svg", Namespace::Svg);
    /// assert_eq!(svg.namespace, Namespace::Svg);
    /// ```
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

/// Configuration options for the HTML parser.
///
/// These options control various aspects of parsing behavior, including scripting support,
/// base URL resolution, encoding hints, and feature flags.
///
/// # Examples
///
/// ```
/// use ace::html::{ParserOptions, Encoding};
///
/// let mut options = ParserOptions::default();
/// options.scripting_enabled = false;
/// options.base_url = Some("https://example.com/".to_string());
/// options.encoding_hint = Some(Encoding::Utf8);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserOptions {
    /// Whether scripting is enabled (affects `<noscript>` parsing).
    pub scripting_enabled: bool,
    /// Base URL for resolving relative URLs.
    pub base_url: Option<String>,
    /// Source URL of the document being parsed.
    pub source_url: Option<String>,
    /// Hint for character encoding detection.
    pub encoding_hint: Option<Encoding>,
    /// Whether to track line and column positions for errors.
    pub track_positions: bool,
    /// Whether to collect preload requests during parsing.
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

/// Context information for parsing HTML fragments.
///
/// When parsing a fragment (e.g., `innerHTML`), the parser needs to know the context
/// element to determine the correct parsing rules. This struct provides that context.
///
/// # Examples
///
/// ```
/// use ace::html::{FragmentContext, Namespace};
///
/// // Parse as if inside a <div> element
/// let context = FragmentContext::new("div");
///
/// // Parse as if inside an SVG element
/// let svg_context = FragmentContext::new("svg")
///     .with_namespace(Namespace::Svg);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentContext {
    /// The tag name of the context element.
    pub tag_name: String,
    /// The namespace of the context element.
    pub namespace: Namespace,
    /// Whether scripting is enabled in the context.
    pub scripting_enabled: bool,
}

impl FragmentContext {
    /// Creates a new fragment context with the given tag name.
    ///
    /// The tag name is automatically converted to lowercase, and the namespace
    /// defaults to HTML with scripting enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::FragmentContext;
    ///
    /// let context = FragmentContext::new("div");
    /// assert_eq!(context.tag_name, "div");
    /// ```
    pub fn new(tag_name: impl Into<String>) -> Self {
        Self {
            tag_name: tag_name.into().to_ascii_lowercase(),
            namespace: Namespace::Html,
            scripting_enabled: true,
        }
    }

    /// Sets the namespace for this fragment context.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::{FragmentContext, Namespace};
    ///
    /// let context = FragmentContext::new("svg")
    ///     .with_namespace(Namespace::Svg);
    /// assert_eq!(context.namespace, Namespace::Svg);
    /// ```
    pub fn with_namespace(mut self, namespace: Namespace) -> Self {
        self.namespace = namespace;
        self
    }

    /// Sets whether scripting is enabled for this fragment context.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::FragmentContext;
    ///
    /// let context = FragmentContext::new("div")
    ///     .with_scripting(false);
    /// assert!(!context.scripting_enabled);
    /// ```
    pub fn with_scripting(mut self, scripting_enabled: bool) -> Self {
        self.scripting_enabled = scripting_enabled;
        self
    }
}

/// Parses an HTML document from a string.
///
/// This is the simplest way to parse HTML. It uses default parser options and
/// returns only the document structure without error information.
///
/// # Examples
///
/// ```
/// use ace::html::parse_document;
///
/// let html = "<!DOCTYPE html><html><body>Hello</body></html>";
/// let doc = parse_document(html);
/// assert!(!doc.children.is_empty());
/// ```
///
/// # See Also
///
/// - [`parse_document_with_options`] - Parse with custom options
/// - [`parse_document_with_errors`] - Parse and collect errors
pub fn parse_document(input: &str) -> HtmlDocument {
    parse_document_with_options(input, &ParserOptions::default())
}

/// Parses an HTML document with custom parser options.
///
/// This function allows you to customize parsing behavior through [`ParserOptions`].
///
/// # Examples
///
/// ```
/// use ace::html::{parse_document_with_options, ParserOptions};
///
/// let mut options = ParserOptions::default();
/// options.scripting_enabled = false;
///
/// let html = "<noscript>Visible content</noscript>";
/// let doc = parse_document_with_options(html, &options);
/// ```
pub fn parse_document_with_options(input: &str, options: &ParserOptions) -> HtmlDocument {
    build_document_with_errors_and_options(input, options).document
}

/// Parses an HTML document from bytes, detecting the character encoding.
///
/// The encoding is detected from:
/// 1. Byte Order Mark (BOM)
/// 2. HTTP Content-Type header (if provided)
/// 3. `<meta charset>` declaration
/// 4. Defaults to UTF-8
///
/// # Errors
///
/// Returns an error if the bytes cannot be decoded to valid text.
///
/// # Examples
///
/// ```
/// use ace::html::parse_document_from_bytes;
///
/// let html_bytes = b"<!DOCTYPE html><html><body>Hello</body></html>";
/// let doc = parse_document_from_bytes(html_bytes).unwrap();
/// assert!(!doc.children.is_empty());
/// ```
pub fn parse_document_from_bytes(bytes: &[u8]) -> Result<HtmlDocument, String> {
    parse_document_from_bytes_with_options(bytes, None, &ParserOptions::default())
}

/// Parses an HTML document from bytes with custom options and HTTP header.
///
/// # Arguments
///
/// * `bytes` - The raw bytes of the HTML document
/// * `http_header` - Optional HTTP Content-Type header for encoding detection
/// * `options` - Parser configuration options
///
/// # Errors
///
/// Returns an error if the bytes cannot be decoded to valid text.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_document_from_bytes_with_options, ParserOptions};
///
/// let bytes = b"<!DOCTYPE html><html><body>Hello</body></html>";
/// let header = Some("text/html; charset=utf-8");
/// let options = ParserOptions::default();
///
/// let doc = parse_document_from_bytes_with_options(bytes, header, &options).unwrap();
/// ```
pub fn parse_document_from_bytes_with_options(
    bytes: &[u8],
    http_header: Option<&str>,
    options: &ParserOptions,
) -> Result<HtmlDocument, String> {
    Ok(parse_document_from_bytes_with_errors_and_options(bytes, http_header, options)?.document)
}

/// Parses an HTML document and returns detailed error information.
///
/// This function returns a [`TreeBuildOutput`] that includes the parsed document
/// and any errors encountered during parsing.
///
/// # Examples
///
/// ```
/// use ace::html::parse_document_with_errors;
///
/// let html = "<div><p>Unclosed paragraph</div>";
/// let output = parse_document_with_errors(html);
///
/// assert!(!output.document.children.is_empty());
/// // May have errors for unclosed tags
/// ```
pub fn parse_document_with_errors(input: &str) -> TreeBuildOutput {
    parse_document_with_errors_and_options(input, &ParserOptions::default())
}

/// Parses an HTML document with custom options and returns detailed error information.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_document_with_errors_and_options, ParserOptions};
///
/// let mut options = ParserOptions::default();
/// options.track_positions = true;
///
/// let html = "<div><p>Content</div>";
/// let output = parse_document_with_errors_and_options(html, &options);
/// ```
pub fn parse_document_with_errors_and_options(input: &str, options: &ParserOptions) -> TreeBuildOutput {
    build_document_with_errors_and_options(input, options)
}

/// Parses HTML document from bytes with full error reporting.
///
/// # Arguments
///
/// * `bytes` - The raw bytes of the HTML document
/// * `http_header` - Optional HTTP Content-Type header for encoding detection
/// * `options` - Parser configuration options
///
/// # Errors
///
/// Returns an error if the bytes cannot be decoded to valid text.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_document_from_bytes_with_errors_and_options, ParserOptions};
///
/// let bytes = b"<div><p>Content</div>";
/// let output = parse_document_from_bytes_with_errors_and_options(
///     bytes, None, &ParserOptions::default()
/// ).unwrap();
/// ```
pub fn parse_document_from_bytes_with_errors_and_options(
    bytes: &[u8],
    http_header: Option<&str>,
    options: &ParserOptions,
) -> Result<TreeBuildOutput, String> {
    let decoded = decode_html_bytes(bytes, http_header, options.encoding_hint)?;
    Ok(build_document_with_errors_and_options(&decoded.content, options))
}

/// Parses an HTML fragment.
///
/// Fragment parsing is used when parsing HTML that will be inserted into an existing
/// document (e.g., `innerHTML`). The context element determines how the HTML is parsed.
///
/// # Arguments
///
/// * `input` - The HTML fragment to parse
/// * `context` - Optional context element tag name (defaults to "body")
///
/// # Examples
///
/// ```
/// use ace::html::parse_fragment;
///
/// let html = "<div><p>Hello</p></div>";
/// let nodes = parse_fragment(html, Some("body"));
/// assert_eq!(nodes.len(), 1);
/// ```
pub fn parse_fragment(input: &str, context: Option<&str>) -> Vec<HtmlNode> {
    parse_fragment_with_context(
        input,
        context.map(FragmentContext::new).as_ref(),
        &ParserOptions::default(),
    )
}

/// Parses an HTML fragment with custom context and options.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_fragment_with_context, FragmentContext, ParserOptions};
///
/// let html = "<li>Item</li>";
/// let context = FragmentContext::new("ul");
/// let options = ParserOptions::default();
///
/// let nodes = parse_fragment_with_context(html, Some(&context), &options);
/// ```
pub fn parse_fragment_with_context(
    input: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> Vec<HtmlNode> {
    build_fragment_with_context_and_options(input, context, options).document.children
}

/// Parses an HTML fragment and returns detailed error information.
///
/// # Examples
///
/// ```
/// use ace::html::{parse_fragment_with_errors_and_context, FragmentContext, ParserOptions};
///
/// let html = "<div><p>Unclosed</div>";
/// let context = FragmentContext::new("body");
/// let options = ParserOptions::default();
///
/// let output = parse_fragment_with_errors_and_context(html, Some(&context), &options);
/// assert!(!output.errors.is_empty());
/// ```
pub fn parse_fragment_with_errors_and_context(
    input: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> TreeBuildOutput {
    build_fragment_with_context_and_options(input, context, options)
}

/// HTML5 parse error codes as defined by the WHATWG specification.
///
/// These error codes correspond to the parse errors defined in the
/// [WHATWG HTML Living Standard](https://html.spec.whatwg.org/#parse-errors).
///
/// # Examples
///
/// ```
/// use ace::html::AceHtmlErrorCode;
///
/// let error = AceHtmlErrorCode::UnexpectedNullCharacter;
/// assert_eq!(error.as_str(), "unexpected-null-character");
/// ```
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
    /// Returns the string representation of this error code.
    ///
    /// The string format matches the error codes defined in the WHATWG specification.
    ///
    /// # Examples
    ///
    /// ```
    /// use ace::html::AceHtmlErrorCode;
    ///
    /// let error = AceHtmlErrorCode::EofInComment;
    /// assert_eq!(error.as_str(), "eof-in-comment");
    /// ```
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

/// A parse error encountered during HTML parsing.
///
/// Parse errors include information about the error type, source component,
/// error message, and position in the input.
///
/// # Examples
///
/// ```
/// use ace::html::parse_document_with_errors;
///
/// let html = "<div><p>Unclosed</div>";
/// let output = parse_document_with_errors(html);
///
/// for error in output.parse_errors() {
///     println!("Error at {}:{}: {}", error.line, error.column, error.message);
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The error code as defined by the WHATWG specification.
    pub code: AceHtmlErrorCode,
    /// The parser component that generated this error.
    pub source: ParseErrorSource,
    /// A human-readable error message.
    pub message: String,
    /// The line number where the error occurred (1-indexed).
    pub line: usize,
    /// The column number where the error occurred (1-indexed).
    pub column: usize,
}

/// The source component that generated a parse error.
///
/// Parse errors can originate from the lexer, tokenizer, or tree builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorSource {
    /// Error from the lexer (low-level tokenization).
    Lexer,
    /// Error from the tokenizer (token processing).
    Tokenizer,
    /// Error from the tree builder (DOM construction).
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
