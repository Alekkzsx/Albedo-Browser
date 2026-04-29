use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252};
use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use memchr::memchr;
use tracing::{debug, trace_span};

mod html5ever_parser;
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

pub struct HtmlTokenizer<'a> {
    tokenizer: Tokenizer<html5gum::StringReader<'a>, DefaultEmitter>,
    errors: Vec<ParseError>,
    emitted_eof: bool,
    input: &'a str,
}

impl<'a> HtmlTokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut emitter = DefaultEmitter::default();
        emitter.switch_states(true);
        Self {
            tokenizer: Tokenizer::new_with_emitter(input, emitter),
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
                Some(Ok(token)) => match token {
                    Token::Error(error) => {
                        self.errors.push(map_html5gum_error(self.input, error));
                    }
                    other => match map_html5gum_token(other) {
                        Some(kind) => return Some(HtmlToken { kind }),
                        None => continue,
                    },
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
    html5ever_parser::parse_fragment_html5ever(html, context.as_ref(), &ParserOptions::default())
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
    parse_fragment_with_result(html, context, options)
        .document
        .children
}

pub fn parse_fragment_with_result(
    html: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> ParseResult {
    let parse_span = trace_span!(
        "ace_html.parse_fragment",
        input_bytes = html.len(),
        has_context = context.is_some(),
        scripting_enabled = options.scripting_enabled
    );
    let _guard = parse_span.enter();
    let started = Instant::now();

    let mut result = html5ever_parser::parse_fragment_html5ever(html, context, options);
    apply_parse_telemetry(&mut result, started.elapsed(), false, html.len());
    debug!(
        parse_time_us = result.stats.parse_time_us as u64,
        total_errors = result.stats.total_errors,
        total_preloads = result.stats.total_preloads,
        "ace-html fragment parse complete"
    );
    result
}

pub fn parse_document_with_errors_and_options(html: &str, options: &ParserOptions) -> ParseResult {
    let parse_span = trace_span!(
        "ace_html.parse_document",
        input_bytes = html.len(),
        scripting_enabled = options.scripting_enabled
    );
    let _guard = parse_span.enter();
    let started = Instant::now();

    let mut fast_path_used = false;
    let mut result = if let Some(result) = try_fast_parse_document(html, options) {
        fast_path_used = true;
        result
    } else {
        html5ever_parser::parse_document_html5ever(html, options)
    };

    apply_parse_telemetry(&mut result, started.elapsed(), fast_path_used, html.len());
    debug!(
        parse_time_us = result.stats.parse_time_us as u64,
        total_errors = result.stats.total_errors,
        total_preloads = result.stats.total_preloads,
        fast_path_used = result.stats.fast_path_used,
        "ace-html document parse complete"
    );
    result
}

pub fn parse_html_integrated_with_options(html: &str, options: &ParserOptions) -> ParseResult {
    parse_document_with_errors_and_options(html, options)
}

pub fn decode_html_bytes(
    bytes: &[u8],
    bom: Option<&[u8]>,
    hint: Option<Encoding>,
) -> Result<DecodedHtml, ParseError> {
    let (encoding, bom_len) = sniff_document_encoding(bytes, bom, hint);
    let payload = &bytes[bom_len.min(bytes.len())..];

    let content = match encoding {
        Encoding::Utf8 => match simdutf8::basic::from_utf8(payload) {
            Ok(valid) => valid.to_string(),
            Err(_) => UTF_8.decode_without_bom_handling(payload).0.into_owned(),
        },
        Encoding::Windows1252 => WINDOWS_1252
            .decode_without_bom_handling(payload)
            .0
            .into_owned(),
        Encoding::Utf16Le => UTF_16LE
            .decode_without_bom_handling(payload)
            .0
            .into_owned(),
        Encoding::Utf16Be => UTF_16BE
            .decode_without_bom_handling(payload)
            .0
            .into_owned(),
    };

    Ok(DecodedHtml { content, encoding })
}

pub fn parse_document_from_bytes_with_errors_and_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    let decoded = decode_html_bytes(bytes, bom, options.encoding_hint.clone())?;
    Ok(parse_document_with_errors_and_options(
        &decoded.content,
        options,
    ))
}

pub fn parse_html_integrated_from_bytes_with_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    parse_document_from_bytes_with_errors_and_options(bytes, bom, options)
}

fn apply_parse_telemetry(
    result: &mut ParseResult,
    elapsed: Duration,
    fast_path_used: bool,
    input_bytes: usize,
) {
    result.stats.total_errors = result.parse_errors.len();
    result.stats.total_preloads = result.preload_requests.len();
    result.stats.parse_time_us = elapsed.as_micros();
    result.stats.fast_path_used = fast_path_used;
    result.stats.input_bytes = input_bytes;
}

fn sniff_document_encoding(
    bytes: &[u8],
    bom: Option<&[u8]>,
    hint: Option<Encoding>,
) -> (Encoding, usize) {
    if let Some(bom_bytes) = bom {
        if let Some((encoding, len)) = detect_bom(bom_bytes) {
            return (encoding, len);
        }
    }

    if let Some((encoding, len)) = detect_bom(bytes) {
        return (encoding, len);
    }

    if let Some(encoding) = hint {
        return (encoding, 0);
    }

    if let Some(meta_encoding) = sniff_meta_charset(bytes) {
        return (meta_encoding, 0);
    }

    if simdutf8::basic::from_utf8(bytes).is_ok() {
        (Encoding::Utf8, 0)
    } else {
        (Encoding::Windows1252, 0)
    }
}

fn detect_bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Some((Encoding::Utf8, 3));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Some((Encoding::Utf16Le, 2));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Some((Encoding::Utf16Be, 2));
    }
    None
}

fn sniff_meta_charset(bytes: &[u8]) -> Option<Encoding> {
    let head = &bytes[..bytes.len().min(4096)];
    let head_str = String::from_utf8_lossy(head);
    let lower = head_str.to_ascii_lowercase();
    let lower_bytes = lower.as_bytes();
    let mut cursor = 0usize;

    while cursor < lower_bytes.len() {
        let Some(open_rel) = memchr(b'<', &lower_bytes[cursor..]) else {
            break;
        };
        let tag_start = cursor + open_rel;
        if !lower_bytes[tag_start..].starts_with(b"<meta") {
            cursor = tag_start + 1;
            continue;
        }

        let tag_tail = &lower_bytes[tag_start..];
        let tag_end_rel = memchr(b'>', tag_tail).unwrap_or(tag_tail.len().saturating_sub(1));
        let end = (tag_start + tag_end_rel + 1).min(lower.len());
        if end <= tag_start {
            break;
        }
        let tag = &lower[tag_start..end];

        if let Some(charset) = extract_meta_charset(tag) {
            if let Some(enc) = encoding_from_label(&charset) {
                return Some(enc);
            }
        }

        cursor = end;
    }
    None
}

fn extract_meta_charset(tag: &str) -> Option<String> {
    let attrs = parse_meta_attributes(tag);
    if let Some(charset) = attrs.get("charset").filter(|value| !value.is_empty()) {
        return Some(charset.clone());
    }

    let content = attrs.get("content")?;
    let lower = content.to_ascii_lowercase();
    let charset_idx = lower.find("charset=")?;
    let raw = &content[charset_idx + "charset=".len()..];
    let trimmed = raw.trim_start();
    let charset = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or("").trim()
    } else if let Some(rest) = trimmed.strip_prefix('\'') {
        rest.split('\'').next().unwrap_or("").trim()
    } else {
        trimmed
            .split(|ch: char| ch == ';' || ch.is_whitespace())
            .next()
            .unwrap_or("")
            .trim()
    };

    if charset.is_empty() {
        None
    } else {
        Some(charset.to_string())
    }
}

fn parse_meta_attributes(tag: &str) -> HashMap<String, String> {
    let bytes = tag.as_bytes();
    let mut idx = 0usize;
    let mut attrs = HashMap::new();

    while idx < bytes.len() && bytes[idx] != b' ' && bytes[idx] != b'>' {
        idx += 1;
    }

    while idx < bytes.len() {
        while idx < bytes.len()
            && (bytes[idx].is_ascii_whitespace() || bytes[idx] == b'/' || bytes[idx] == b'>')
        {
            idx += 1;
        }
        if idx >= bytes.len() {
            break;
        }

        let name_start = idx;
        while idx < bytes.len()
            && !bytes[idx].is_ascii_whitespace()
            && bytes[idx] != b'='
            && bytes[idx] != b'>'
            && bytes[idx] != b'/'
        {
            idx += 1;
        }

        if idx == name_start {
            break;
        }

        let name = tag[name_start..idx].trim().to_ascii_lowercase();
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }

        let value = if idx < bytes.len() && bytes[idx] == b'=' {
            idx += 1;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            if idx >= bytes.len() {
                String::new()
            } else if bytes[idx] == b'"' || bytes[idx] == b'\'' {
                let quote = bytes[idx];
                idx += 1;
                let start = idx;
                while idx < bytes.len() && bytes[idx] != quote {
                    idx += 1;
                }
                let value = tag[start..idx].to_string();
                if idx < bytes.len() {
                    idx += 1;
                }
                value
            } else {
                let start = idx;
                while idx < bytes.len()
                    && !bytes[idx].is_ascii_whitespace()
                    && bytes[idx] != b'>'
                    && bytes[idx] != b'/'
                {
                    idx += 1;
                }
                tag[start..idx].to_string()
            }
        } else {
            String::new()
        };

        attrs.entry(name).or_insert(value);
    }

    attrs
}

fn encoding_from_label(label: &str) -> Option<Encoding> {
    let normalized = label.trim().trim_matches('"').trim_matches('\'');
    let canonical = encoding_rs::Encoding::for_label(normalized.as_bytes())?;
    match canonical.name().to_ascii_lowercase().as_str() {
        "utf-8" => Some(Encoding::Utf8),
        "windows-1252" => Some(Encoding::Windows1252),
        "utf-16le" | "utf-16" => Some(Encoding::Utf16Le),
        "utf-16be" => Some(Encoding::Utf16Be),
        _ => None,
    }
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
    raw_bytes: Vec<u8>,
    buffer: String,
    decided_encoding: Option<Encoding>,
}

pub struct StreamingHtmlParser {
    raw_bytes: Vec<u8>,
    buffer: String,
    decided_encoding: Option<Encoding>,
    options: ParserOptions,
    chunk_latencies: Vec<Duration>,
    last_parse_result: Option<ParseResult>,
}

impl StreamingHtmlParser {
    pub fn new() -> Self {
        Self::with_options(ParserOptions::default())
    }

    pub fn with_options(options: ParserOptions) -> Self {
        Self {
            raw_bytes: Vec::new(),
            buffer: String::new(),
            decided_encoding: options.encoding_hint,
            options,
            chunk_latencies: Vec::new(),
            last_parse_result: None,
        }
    }

    pub fn feed(&mut self, chunk: &str) -> streaming::ChunkResult {
        self.feed_bytes(chunk.as_bytes())
    }

    pub fn feed_bytes(&mut self, chunk: &[u8]) -> streaming::ChunkResult {
        let start = Instant::now();
        self.raw_bytes.extend_from_slice(chunk);

        if self.decided_encoding.is_none() {
            if let Some((encoding, _)) = detect_bom(&self.raw_bytes) {
                self.decided_encoding = Some(encoding);
            } else if let Some(meta_encoding) = sniff_meta_charset(&self.raw_bytes) {
                self.decided_encoding = Some(meta_encoding);
            } else if let Some(hint) = self.options.encoding_hint {
                self.decided_encoding = Some(hint);
            }
        }

        let decode_hint = self.decided_encoding.or(Some(Encoding::Utf8));
        match decode_html_bytes(&self.raw_bytes, None, decode_hint) {
            Ok(decoded) => {
                self.buffer = decoded.content;
                self.last_parse_result =
                    Some(parse_document_with_errors_and_options(&self.buffer, &self.options));
            }
            Err(err) => {
                return streaming::ChunkResult::Error(format!(
                    "failed to decode stream chunk: {}",
                    err.message
                ));
            }
        }

        self.chunk_latencies.push(start.elapsed());
        streaming::ChunkResult::Ok
    }

    pub fn end(&mut self) -> HtmlDocument {
        self.end_with_parse_result().document
    }

    pub fn end_with_parse_result(&mut self) -> ParseResult {
        if !self.raw_bytes.is_empty() {
            if let Ok(decoded) = decode_html_bytes(
                &self.raw_bytes,
                None,
                self.decided_encoding.or(self.options.encoding_hint),
            ) {
                self.decided_encoding = Some(decoded.encoding);
                self.buffer = decoded.content;
                self.last_parse_result =
                    Some(parse_document_with_errors_and_options(&self.buffer, &self.options));
            }
        }

        self.last_parse_result
            .take()
            .unwrap_or_else(|| parse_document_with_errors_and_options(&self.buffer, &self.options))
    }

    pub fn snapshot(&self) -> StreamingSnapshot {
        StreamingSnapshot {
            raw_bytes: self.raw_bytes.clone(),
            buffer: self.buffer.clone(),
            decided_encoding: self.decided_encoding,
        }
    }

    pub fn restore(&mut self, snapshot: StreamingSnapshot) {
        self.raw_bytes = snapshot.raw_bytes;
        self.buffer = snapshot.buffer;
        self.decided_encoding = snapshot.decided_encoding;
        self.last_parse_result = Some(parse_document_with_errors_and_options(
            &self.buffer,
            &self.options,
        ));
    }

    pub fn p50_latency(&self) -> Duration {
        percentile_duration(&self.chunk_latencies, 50)
    }

    pub fn p99_latency(&self) -> Duration {
        percentile_duration(&self.chunk_latencies, 99)
    }

    pub fn decided_encoding(&self) -> Option<Encoding> {
        self.decided_encoding
    }

    pub fn bytes_seen(&self) -> usize {
        self.raw_bytes.len()
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

fn decode_text_token(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\0', "\u{FFFD}")
}

fn decode_legacy_cdata_comment(data: &str) -> Option<String> {
    data.strip_prefix("[CDATA[")
        .and_then(|inner| inner.strip_suffix("]]"))
        .map(str::to_string)
}

fn map_html5gum_error(input: &str, error: Html5Error) -> ParseError {
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

const FAST_PATH_MIN_BYTES: usize = 16 * 1024;

fn try_fast_parse_document(html: &str, options: &ParserOptions) -> Option<ParseResult> {
    if !is_fast_path_candidate(html) {
        return None;
    }

    let bytes = html.as_bytes();
    let mut cursor = 0usize;
    let mut document = HtmlDocument {
        doctype: None,
        children: Vec::new(),
    };
    let mut root = Vec::<HtmlNode>::new();
    let mut stack = Vec::<HtmlElement>::new();

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            append_fast_text(&mut root, &mut stack, &html[cursor..]);
            break;
        };

        let tag_start = cursor + relative;
        append_fast_text(&mut root, &mut stack, &html[cursor..tag_start]);

        if tag_start + 1 >= bytes.len() {
            return None;
        }

        match bytes[tag_start + 1] {
            b'!' => {
                let after_bang = &html[tag_start + 2..];
                if after_bang.len() < 7 {
                    return None;
                }
                if !after_bang[..7].eq_ignore_ascii_case("doctype") {
                    return None;
                }

                let Some(tag_end_rel) = memchr(b'>', &bytes[tag_start..]) else {
                    return None;
                };
                let tag_end = tag_start + tag_end_rel;
                let inner = html[tag_start + 2..tag_end].trim();
                let mut parts = inner.split_whitespace();
                let keyword = parts.next()?;
                if !keyword.eq_ignore_ascii_case("doctype") {
                    return None;
                }
                let name = parts.next().unwrap_or("html").to_ascii_lowercase();
                document.doctype = Some(DoctypeToken {
                    name: Some(name),
                    public_id: None,
                    system_id: None,
                    force_quirks: false,
                });
                cursor = tag_end + 1;
            }
            b'/' => {
                let (tag_name, tag_end) = parse_fast_end_tag(html, tag_start)?;
                let current = stack.pop()?;
                if !current.tag.eq_ignore_ascii_case(&tag_name) {
                    return None;
                }
                push_node(&mut root, &mut stack, HtmlNode::Element(current));
                cursor = tag_end + 1;
            }
            _ => {
                let (element, self_closing, tag_end) = parse_fast_start_tag(html, tag_start)?;
                if self_closing || is_void_element(&element.tag) {
                    push_node(&mut root, &mut stack, HtmlNode::Element(element));
                } else {
                    stack.push(element);
                }
                cursor = tag_end + 1;
            }
        }
    }

    while let Some(element) = stack.pop() {
        push_node(&mut root, &mut stack, HtmlNode::Element(element));
    }

    document.children = if options.scripting_enabled {
        root
    } else {
        transform_noscript(root)
    };

    let preload_requests = if fast_path_has_link_tag(bytes) {
        extract_preloads(&document, options)
    } else {
        Vec::new()
    };
    Some(ParseResult {
        document,
        errors: Vec::new(),
        parse_errors: Vec::new(),
        preload_requests: preload_requests.clone(),
        stats: ParseStats {
            total_errors: 0,
            total_preloads: preload_requests.len(),
            ..ParseStats::default()
        },
    })
}

fn is_fast_path_candidate(html: &str) -> bool {
    if html.len() < FAST_PATH_MIN_BYTES {
        return false;
    }

    let lowercase = html.to_ascii_lowercase();
    !lowercase.contains('&')
        && !lowercase.contains('\0')
        && !lowercase.contains("<!--")
        && !lowercase.contains("<?")
        && !lowercase.contains("<![cdata[")
        && !lowercase.contains("<script")
        && !lowercase.contains("<style")
        && !lowercase.contains("<noscript")
}

fn fast_path_has_link_tag(bytes: &[u8]) -> bool {
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            return false;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            return false;
        }
        if ascii_starts_with(bytes.get(tag_open..).unwrap_or_default(), b"link") {
            let boundary = bytes.get(tag_open + 4).map_or(true, |next| {
                next.is_ascii_whitespace() || matches!(*next, b'>' | b'/')
            });
            if boundary {
                return true;
            }
        }
        cursor = tag_open;
    }
    false
}

fn ascii_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack
            .iter()
            .zip(needle.iter())
            .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
}

fn append_fast_text(root: &mut Vec<HtmlNode>, stack: &mut [HtmlElement], text: &str) {
    if !text.is_empty() {
        push_node(root, stack, HtmlNode::Text(text.to_string()));
    }
}

fn parse_fast_end_tag(html: &str, tag_start: usize) -> Option<(String, usize)> {
    let bytes = html.as_bytes();
    let mut idx = tag_start + 2;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    let name_start = idx;
    while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'-') {
        idx += 1;
    }
    if idx == name_start {
        return None;
    }
    let name = html[name_start..idx].to_ascii_lowercase();
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if bytes.get(idx) != Some(&b'>') {
        return None;
    }
    Some((name, idx))
}

fn parse_fast_start_tag(html: &str, tag_start: usize) -> Option<(HtmlElement, bool, usize)> {
    let bytes = html.as_bytes();
    let mut idx = tag_start + 1;
    let name_start = idx;
    while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'-') {
        idx += 1;
    }
    if idx == name_start {
        return None;
    }

    let tag = html[name_start..idx].to_ascii_lowercase();
    let namespace = Namespace::Html;
    let mut attributes = HashMap::new();
    let mut self_closing = false;

    loop {
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }

        match bytes.get(idx).copied() {
            Some(b'>') => {
                return Some((
                    HtmlElement {
                        tag,
                        namespace,
                        attributes,
                        children: Vec::new(),
                    },
                    self_closing,
                    idx,
                ));
            }
            Some(b'/') if bytes.get(idx + 1) == Some(&b'>') => {
                self_closing = true;
                idx += 1;
            }
            Some(_) => {
                let attr_start = idx;
                while idx < bytes.len()
                    && !bytes[idx].is_ascii_whitespace()
                    && bytes[idx] != b'='
                    && bytes[idx] != b'>'
                    && bytes[idx] != b'/'
                {
                    idx += 1;
                }
                if idx == attr_start {
                    return None;
                }
                let attr_name = html[attr_start..idx].to_ascii_lowercase();
                while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                    idx += 1;
                }

                let value = if bytes.get(idx) == Some(&b'=') {
                    idx += 1;
                    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                        idx += 1;
                    }
                    parse_fast_attr_value(html, &mut idx)?
                } else {
                    String::new()
                };
                attributes.entry(attr_name).or_insert(value);
            }
            None => return None,
        }
    }
}

fn parse_fast_attr_value(html: &str, idx: &mut usize) -> Option<String> {
    let bytes = html.as_bytes();
    match bytes.get(*idx).copied() {
        Some(b'"') | Some(b'\'') => {
            let quote = bytes[*idx];
            *idx += 1;
            let start = *idx;
            while *idx < bytes.len() && bytes[*idx] != quote {
                *idx += 1;
            }
            let value = html[start..*idx].to_string();
            if *idx < bytes.len() {
                *idx += 1;
            }
            Some(value)
        }
        Some(_) => {
            let start = *idx;
            while *idx < bytes.len()
                && !bytes[*idx].is_ascii_whitespace()
                && bytes[*idx] != b'>'
                && bytes[*idx] != b'/'
            {
                *idx += 1;
            }
            Some(html[start..*idx].to_string())
        }
        None => Some(String::new()),
    }
}

#[allow(dead_code)]
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
                push_node(
                    &mut root,
                    &mut stack,
                    HtmlNode::Comment(comment.data.clone()),
                );
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

#[allow(dead_code)]
fn infer_namespace(stack: &[HtmlElement], tag: &str) -> Namespace {
    if tag.eq_ignore_ascii_case("svg") {
        Namespace::Svg
    } else if tag.eq_ignore_ascii_case("math")
        || stack
            .last()
            .is_some_and(|element| element.namespace == Namespace::MathMl)
    {
        Namespace::MathMl
    } else if stack
        .last()
        .is_some_and(|element| element.namespace == Namespace::Svg)
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
            if rel.contains("stylesheet")
                || rel.contains("preload")
                || rel.contains("modulepreload")
            {
                let href = element.attributes.get("href").cloned().unwrap_or_default();
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
                is_module: element
                    .attributes
                    .get("type")
                    .is_some_and(|value| value == "module"),
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
