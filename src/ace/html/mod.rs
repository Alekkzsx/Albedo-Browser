use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use html5gum::{Tokenizer, Token};

pub mod tokenizer_v2;
pub mod tree_builder;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

pub use tokenizer_v2::AceTokenizer;
pub use tree_builder::{HtmlTreeBuilder, InsertionMode};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Windows1252,
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

pub struct HtmlTokenizer<'a> {
    tokenizer: Tokenizer<html5gum::StringReader<'a>>,
    errors: Vec<ParseError>,
    emitted_eof: bool,
    input: &'a str,
}

impl<'a> HtmlTokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            tokenizer: Tokenizer::new(input),
            errors: detect_initial_errors(input),
            emitted_eof: false,
            input,
        }
    }

    pub fn next_token(&mut self) -> Option<HtmlToken> {
        if self.emitted_eof {
            return None;
        }

        loop {
            match self.tokenizer.next() {
                Some(Ok(token)) => match map_html5gum_token(token) {
                    Some(kind) => return Some(HtmlToken { kind }),
                    None => continue,
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

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

pub fn parse_document(html: &str) -> HtmlDocument {
    parse_document_with_options(html, &ParserOptions::default())
}

pub fn parse_document_with_options(html: &str, options: &ParserOptions) -> HtmlDocument {
    parse_document_with_errors_and_options(html, options).document
}

pub fn parse_document_with_errors(html: &str) -> ParseResult {
    parse_document_with_errors_and_options(html, &ParserOptions::default())
}

pub fn build_document_with_errors(html: &str) -> ParseResult {
    parse_document_with_errors(html)
}

pub fn build_fragment_with_errors(html: &str, context: Option<&str>) -> ParseResult {
    let context = context.map(FragmentContext::new);
    let options = ParserOptions::default();
    let nodes = parse_fragment_with_context(html, context.as_ref(), &options);
    ParseResult {
        document: HtmlDocument {
            doctype: None,
            children: nodes,
        },
        errors: Vec::new(),
        parse_errors: Vec::new(),
        preload_requests: Vec::new(),
        stats: ParseStats::default(),
    }
}

pub fn parse_fragment(html: &str, context: Option<&str>) -> Vec<HtmlNode> {
    let context = context.map(FragmentContext::new);
    parse_fragment_with_context(html, context.as_ref(), &ParserOptions::default())
}

pub fn parse_fragment_with_context(
    html: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> Vec<HtmlNode> {
    let wrapped = if let Some(context) = context {
        format!(
            "<{tag}>{html}</{tag}>",
            tag = context.tag_name,
            html = html
        )
    } else {
        html.to_string()
    };

    let parsed = parse_document_with_errors_and_options(&wrapped, options);
    if context.is_none() {
        return parsed.document.children;
    }

    parsed
        .document
        .children
        .into_iter()
        .find_map(|node| match node {
            HtmlNode::Element(element) => Some(element.children),
            _ => None,
        })
        .unwrap_or_default()
}

pub fn parse_document_with_errors_and_options(
    html: &str,
    options: &ParserOptions,
) -> ParseResult {
    let mut tokenizer = HtmlTokenizer::new(html);
    let mut tokens = Vec::new();

    while let Some(token) = tokenizer.next_token() {
        let eof = matches!(token.kind, HtmlTokenKind::Eof);
        tokens.push(token);
        if eof {
            break;
        }
    }

    let mut errors = tokenizer.errors().to_vec();
    let mut document = HtmlDocument {
        doctype: None,
        children: Vec::new(),
    };
    build_document_from_tokens(&tokens, &mut document, &mut errors, options);
    let preload_requests = extract_preloads(&document, options);
    let stats = ParseStats {
        total_errors: errors.len(),
        total_preloads: preload_requests.len(),
    };

    ParseResult {
        document,
        errors: errors.clone(),
        parse_errors: errors,
        preload_requests,
        stats,
    }
}

pub fn parse_html_integrated_with_options(
    html: &str,
    options: &ParserOptions,
) -> ParseResult {
    parse_document_with_errors_and_options(html, options)
}

pub fn decode_html_bytes(
    bytes: &[u8],
    _bom: Option<&[u8]>,
    hint: Option<Encoding>,
) -> Result<DecodedHtml, ParseError> {
    let encoding = hint.unwrap_or(Encoding::Utf8);
    let content = match encoding {
        Encoding::Utf8 => String::from_utf8_lossy(bytes).to_string(),
        Encoding::Windows1252 => decode_windows_1252(bytes),
    };

    Ok(DecodedHtml { content, encoding })
}

pub fn parse_document_from_bytes_with_errors_and_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    let decoded = decode_html_bytes(bytes, bom, options.encoding_hint.clone())?;
    Ok(parse_document_with_errors_and_options(&decoded.content, options))
}

pub fn parse_html_integrated_from_bytes_with_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    parse_document_from_bytes_with_errors_and_options(bytes, bom, options)
}

pub mod streaming {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ChunkResult {
        Ok,
        Error(String),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingSnapshot {
    buffer: String,
}

pub struct StreamingHtmlParser {
    buffer: String,
    options: ParserOptions,
    chunk_latencies: Vec<Duration>,
}

impl StreamingHtmlParser {
    pub fn new() -> Self {
        Self::with_options(ParserOptions::default())
    }

    pub fn with_options(options: ParserOptions) -> Self {
        Self {
            buffer: String::new(),
            options,
            chunk_latencies: Vec::new(),
        }
    }

    pub fn feed(&mut self, chunk: &str) -> streaming::ChunkResult {
        let start = Instant::now();
        self.buffer.push_str(chunk);
        self.chunk_latencies.push(start.elapsed());
        streaming::ChunkResult::Ok
    }

    pub fn end(&mut self) -> HtmlDocument {
        self.end_with_parse_result().document
    }

    pub fn end_with_parse_result(&mut self) -> ParseResult {
        parse_document_with_errors_and_options(&self.buffer, &self.options)
    }

    pub fn snapshot(&self) -> StreamingSnapshot {
        StreamingSnapshot {
            buffer: self.buffer.clone(),
        }
    }

    pub fn restore(&mut self, snapshot: StreamingSnapshot) {
        self.buffer = snapshot.buffer;
    }

    pub fn p50_latency(&self) -> Duration {
        percentile_duration(&self.chunk_latencies, 50)
    }

    pub fn p99_latency(&self) -> Duration {
        percentile_duration(&self.chunk_latencies, 99)
    }
}

fn percentile_duration(samples: &[Duration], percentile: usize) -> Duration {
    if samples.is_empty() {
        return Duration::ZERO;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile) / 100;
    sorted[index]
}

fn map_html5gum_token(token: Token) -> Option<HtmlTokenKind> {
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
            data: String::from_utf8_lossy(&text).to_string(),
        })),
        Token::Comment(comment) => Some(HtmlTokenKind::Comment(CommentToken {
            data: String::from_utf8_lossy(&comment).to_string(),
        })),
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

fn build_document_from_tokens(
    tokens: &[HtmlToken],
    document: &mut HtmlDocument,
    errors: &mut Vec<ParseError>,
    options: &ParserOptions,
) {
    let mut root = Vec::<HtmlNode>::new();
    let mut stack = Vec::<HtmlElement>::new();

    for token in tokens {
        match &token.kind {
            HtmlTokenKind::Doctype(dt) => {
                if document.doctype.is_none() {
                    document.doctype = Some(dt.clone());
                }
            }
            HtmlTokenKind::StartTag(tag) => {
                let namespace = infer_namespace(&stack, &tag.name);
                let element = HtmlElement {
                    tag: tag.name.clone(),
                    namespace,
                    attributes: tag.attributes.clone().into_iter().collect(),
                    children: Vec::new(),
                };

                if is_void_element(&tag.name) || tag.self_closing {
                    push_node(&mut root, &mut stack, HtmlNode::Element(element));
                } else {
                    stack.push(element);
                }
            }
            HtmlTokenKind::EndTag(tag) => {
                if let Some(position) = stack
                    .iter()
                    .rposition(|element| element.tag.eq_ignore_ascii_case(&tag.name))
                {
                    while stack.len() > position + 1 {
                        let element = stack.pop().unwrap();
                        push_node(&mut root, &mut stack, HtmlNode::Element(element));
                    }
                    let element = stack.pop().unwrap();
                    push_node(&mut root, &mut stack, HtmlNode::Element(element));
                } else if tag.name == "html" || tag.name == "body" || tag.name == "head" {
                    continue;
                } else {
                    errors.push(ParseError {
                        code: "unexpected-end-tag".to_string(),
                        source: ParseErrorSource::TreeBuilder,
                        kind: ParseErrorKind::HtmlSyntax,
                        line: 1,
                        column: 1,
                        message: format!("unexpected closing tag </{}>", tag.name),
                    });
                }
            }
            HtmlTokenKind::Character(text) => {
                if !text.data.is_empty() {
                    if let Some(current) = stack.last_mut() {
                        current.children.push(HtmlNode::Text(text.data.clone()));
                    } else {
                        root.push(HtmlNode::Text(text.data.clone()));
                    }
                }
            }
            HtmlTokenKind::Comment(comment) => {
                push_node(&mut root, &mut stack, HtmlNode::Comment(comment.data.clone()));
            }
            HtmlTokenKind::Eof => break,
        }
    }

    while let Some(element) = stack.pop() {
        push_node(&mut root, &mut stack, HtmlNode::Element(element));
    }

    if options.scripting_enabled {
        document.children = root;
    } else {
        document.children = transform_noscript(root);
    }
}

fn push_node(root: &mut Vec<HtmlNode>, stack: &mut [HtmlElement], node: HtmlNode) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        root.push(node);
    }
}

fn infer_namespace(stack: &[HtmlElement], tag: &str) -> Namespace {
    if tag.eq_ignore_ascii_case("svg") {
        Namespace::Svg
    } else if tag.eq_ignore_ascii_case("math")
        || stack.last().is_some_and(|element| element.namespace == Namespace::MathMl)
    {
        Namespace::MathMl
    } else if stack.last().is_some_and(|element| element.namespace == Namespace::Svg)
        && !tag.eq_ignore_ascii_case("foreignobject")
    {
        Namespace::Svg
    } else {
        Namespace::Html
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn transform_noscript(nodes: Vec<HtmlNode>) -> Vec<HtmlNode> {
    nodes
        .into_iter()
        .map(|node| match node {
            HtmlNode::Element(mut element) if element.tag == "noscript" => {
                let raw = serialize_children_as_text(&element.children);
                element.children = vec![HtmlNode::Text(raw)];
                HtmlNode::Element(element)
            }
            HtmlNode::Element(mut element) => {
                element.children = transform_noscript(element.children);
                HtmlNode::Element(element)
            }
            other => other,
        })
        .collect()
}

fn serialize_children_as_text(children: &[HtmlNode]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            HtmlNode::Element(element) => {
                out.push('<');
                out.push_str(&element.tag);
                out.push('>');
                out.push_str(&serialize_children_as_text(&element.children));
                out.push_str("</");
                out.push_str(&element.tag);
                out.push('>');
            }
            HtmlNode::Text(text) | HtmlNode::Comment(text) => out.push_str(text),
        }
    }
    out
}

fn extract_preloads(document: &HtmlDocument, options: &ParserOptions) -> Vec<PreloadRequest> {
    let mut out = Vec::new();
    for node in &document.children {
        collect_preloads(node, options, &mut out);
    }
    out
}

fn collect_preloads(node: &HtmlNode, options: &ParserOptions, out: &mut Vec<PreloadRequest>) {
    let HtmlNode::Element(element) = node else {
        return;
    };

    if element.tag == "link" {
        if let Some(rel) = element.attributes.get("rel") {
            if rel.contains("stylesheet") || rel.contains("preload") || rel.contains("modulepreload")
            {
                let href = element
                    .attributes
                    .get("href")
                    .cloned()
                    .unwrap_or_default();
                out.push(PreloadRequest {
                    url: absolutize_url(&href, options.base_url.as_deref()),
                    resource_type: if rel.contains("modulepreload") {
                        ResourceType::ModulePreload
                    } else {
                        ResourceType::Stylesheet
                    },
                    priority: RequestPriority::Auto,
                    crossorigin: element.attributes.get("crossorigin").cloned(),
                    rel: Some(rel.clone()),
                    as_attribute: element.attributes.get("as").cloned(),
                    fetchpriority: element.attributes.get("fetchpriority").cloned(),
                    loading: element.attributes.get("loading").cloned(),
                    is_module: rel.contains("modulepreload"),
                    is_async: false,
                    is_defer: false,
                });
            }
        }
    }

    if element.tag == "script" {
        if let Some(src) = element.attributes.get("src") {
            out.push(PreloadRequest {
                url: absolutize_url(src, options.base_url.as_deref()),
                resource_type: ResourceType::Script,
                priority: RequestPriority::Auto,
                crossorigin: element.attributes.get("crossorigin").cloned(),
                rel: None,
                as_attribute: None,
                fetchpriority: element.attributes.get("fetchpriority").cloned(),
                loading: None,
                is_module: element.attributes.get("type").is_some_and(|value| value == "module"),
                is_async: element.attributes.contains_key("async"),
                is_defer: element.attributes.contains_key("defer"),
            });
        }
    }

    for child in &element.children {
        collect_preloads(child, options, out);
    }
}

fn absolutize_url(url: &str, base_url: Option<&str>) -> String {
    if url.contains("://") || url.starts_with('/') || base_url.is_none() {
        url.to_string()
    } else {
        let base = base_url.unwrap();
        if base.ends_with('/') {
            format!("{base}{url}")
        } else {
            format!("{base}/{url}")
        }
    }
}

fn detect_initial_errors(input: &str) -> Vec<ParseError> {
    let mut errors = Vec::new();
    if let Some(index) = input.find("<!DOCTYPE>") {
        let (line, column) = line_column_for_offset(input, index);
        errors.push(ParseError {
            code: "invalid-doctype".to_string(),
            source: ParseErrorSource::Tokenizer,
            kind: ParseErrorKind::InvalidDoctype,
            line,
            column,
            message: "doctype is missing a name".to_string(),
        });
    }
    errors
}

fn rough_error(
    input: &str,
    code: &str,
    source: ParseErrorSource,
    kind: ParseErrorKind,
) -> ParseError {
    let (line, column) = line_column_for_offset(input, 0);
    ParseError {
        code: code.to_string(),
        source,
        kind,
        line,
        column,
        message: code.to_string(),
    }
}

fn line_column_for_offset(input: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for (idx, ch) in input.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x80 => '\u{20AC}',
            0x82 => '\u{201A}',
            0x83 => '\u{0192}',
            0x84 => '\u{201E}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02C6}',
            0x89 => '\u{2030}',
            0x8A => '\u{0160}',
            0x8B => '\u{2039}',
            0x8C => '\u{0152}',
            0x8E => '\u{017D}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201C}',
            0x94 => '\u{201D}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02DC}',
            0x99 => '\u{2122}',
            0x9A => '\u{0161}',
            0x9B => '\u{203A}',
            0x9C => '\u{0153}',
            0x9E => '\u{017E}',
            0x9F => '\u{0178}',
            value => char::from(*value),
        })
        .collect()
}
