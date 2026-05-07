use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctypeToken {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub force_quirks: bool,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlElement {
    pub tag: String,
    pub namespace: Namespace,
    pub attributes: HashMap<String, String>,
    pub children: Vec<HtmlNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlToken {
    pub kind: HtmlTokenKind,
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
pub struct StartTagToken {
    pub name: String,
    pub attributes: BTreeMap<String, String>,
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
pub enum ParseErrorSource {
    Tokenizer,
    TreeBuilder,
    Decoder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    HtmlSyntax,
    InvalidDoctype,
    DecodeError,
    FosterParenting,
    UnexpectedEof,
    MissingSemicolonAfterCharacterReference,
    LexerParseError,
    CdataSectionOutsideForeignContent,
    NullCharacter,
    NestedComment,
    EofInComment,
    EofInDoctype,
    EofInTag,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub code: String,
    pub source: ParseErrorSource,
    pub kind: ParseErrorKind,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingSnapshot {
    pub raw_bytes: Vec<u8>,
    pub buffer: String,
    pub decided_encoding: Option<Encoding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Windows1252,
    Utf16Le,
    Utf16Be,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedHtml {
    pub content: String,
    pub encoding: Encoding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentContext {
    pub tag_name: String,
    pub namespace: Namespace,
    pub scripting_enabled: bool,
}

impl FragmentContext {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_string(),
            namespace: Namespace::Html,
            scripting_enabled: true,
        }
    }

    pub fn with_scripting(mut self, enabled: bool) -> Self {
        self.scripting_enabled = enabled;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserOptions {
    pub base_url: Option<String>,
    pub encoding_hint: Option<Encoding>,
    pub scripting_enabled: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            encoding_hint: None,
            scripting_enabled: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceType {
    Stylesheet,
    Script,
    ModulePreload,
    Image,
    Fetch,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestPriority {
    High,
    Auto,
    Low,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreloadRequest {
    pub url: String,
    pub resource_type: ResourceType,
    pub priority: RequestPriority,
    pub crossorigin: Option<String>,
    pub rel: Option<String>,
    pub as_attribute: Option<String>,
    pub fetchpriority: Option<String>,
    pub loading: Option<String>,
    pub is_module: bool,
    pub is_async: bool,
    pub is_defer: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParseStats {
    pub total_errors: usize,
    pub total_preloads: usize,
    pub parse_time_us: u128,
    pub input_bytes: usize,
    pub fast_path_used: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseResult {
    pub document: HtmlDocument,
    pub errors: Vec<ParseError>,
    pub parse_errors: Vec<ParseError>,
    pub preload_requests: Vec<PreloadRequest>,
    pub stats: ParseStats,
}

impl ParseResult {
    pub fn parse_errors(&self) -> &[ParseError] {
        &self.parse_errors
    }
}
