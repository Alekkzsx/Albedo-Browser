//! # Módulo de Parseamento HTML5
//!
//! Este módulo implementa o parseamento de documentos e fragmentos HTML, seguindo a especificação HTML5.
//! Ele é responsável por transformar texto bruto (ou bytes) em uma árvore estruturada de nós (`HtmlNode`)
//! que será usada pelo motor de DOM e renderização.
//!
//! ## Componentes Principais
//!
//! - **tokenizer_v2**: Tokenizador personalizado que converte o fluxo de caracteres em tokens léxicos
//!   (tags de abertura/fechamento, texto, comentários, doctype). Baseado na crate `html5gum`.
//!
//! - **tree_builder**: Implementa o algoritmo de construção da árvore HTML, gerenciando os modos de inserção
//!   (InsertionMode) e as regras específicas para cada contexto (ex: dentro de table, select, etc.).
//!
//! - **html5ever_parser**: Wrapper e adaptações para usar o parser `html5ever` como backend principal,
//!   garantindo conformidade com os casos de teste do W3C.
//!
//! - **fast_parse**: Otimizações ("fast path") para documentos HTML simples e bem-formados, evitando
//!   overhead do parser completo quando possível.
//!
//! - **streaming**: Suporte a parseamento incremental, permitindo que o documento seja processado
//!   à medida que os dados chegam da rede, melhorando o tempo de primeira pintura (FCP).
//!
//! - **serializer**: Funções para serializar a árvore DOM de volta para string HTML, úteis para
//!   debugging, innerHTML, outerHTML e persistência.
//!
//! - **encoding**: Detecção e decodificação de charset (UTF-8, ISO-8859-1, etc.), incluindo detecção
//!   automática via BOM e heurísticas de sniffing.
//!
//! - **sink**: Implementação do trait `TreeSink` requerido pelo html5ever, atuando como ponte entre
//!   o parser e nossa estrutura de dados interna (`HtmlNode`).
//!
//! - **preloads**: Análise estática do HTML para extrair URLs de recursos críticos (CSS, JS, imagens)
//!   e iniciar o pré-carregamento paralelo.
//!
//! ## Fluxo de Parseamento
//!
//! 1. **Decodificação**: Bytes brutos são convertidos para UTF-8 (`encoding`).
//! 2. **Tokenização**: O texto é dividido em tokens léxicos (`tokenizer_v2` / `html5gum`).
//! 3. **Construção da Árvore**: Os tokens são processados pelo `tree_builder` que cria e insere nós na DOM.
//! 4. **Pré-carregamento**: Durante o parse, URLs de recursos são extraídas (`preloads`).
//! 5. **Resultado**: Uma `HtmlDocument` completa com árvore de nós, lista de erros e requests de preload.
//!
//! ## API Pública
//!
//! - `parse_document()`: Parse um documento HTML completo.
//! - `parse_fragment()`: Parse um fragmento HTML (ex: conteúdo de innerHTML).
//! - `StreamingHtmlParser`: Parser assíncrono para streams de dados.
//! - `serialize_document()`: Converte a DOM de volta para HTML string.
//! - `detect_bom()`, `sniff_document_encoding()`: Utilitários de encoding.

use std::time::{Duration, Instant};

use html5gum::{DefaultEmitter, Error as Html5Error, Token, Tokenizer};
use tracing::{debug, trace_span};

mod html5ever_parser;
pub mod tokenizer_v2;
pub mod tree_builder;

pub mod types;
pub mod encoding;
pub mod streaming;
pub mod preloads;
pub mod sink;
pub mod fast_parse;
pub mod serializer;

pub use types::*;
pub use tokenizer_v2::AceTokenizer;
pub use tree_builder::{HtmlTreeBuilder, InsertionMode};
pub use streaming::StreamingHtmlParser;
pub use encoding::{decode_html_bytes, sniff_document_encoding, detect_bom};
pub use serializer::{serialize_document, serialize_node, SerializeOptions};

/// Tokenizador HTML baseado em html5gum.
/// Converte uma string de entrada em uma sequência de tokens HTML estruturados.
pub struct HtmlTokenizer<'a> {
    tokenizer: Tokenizer<html5gum::StringReader<'a>, DefaultEmitter>,
    errors: Vec<ParseError>,
    emitted_eof: bool,
    input: &'a str,
}

impl<'a> HtmlTokenizer<'a> {
    /// Cria um novo tokenizador para a string de entrada.
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

    /// Obtém o próximo token da sequência.
    /// Retorna `None` quando todos os tokens (incluindo EOF) foram emitidos.
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

    /// Retorna a lista de erros encontrados durante a tokenização.
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

/// Parse um documento HTML completo com opções padrão.
pub fn parse_document(html: &str) -> HtmlDocument {
    parse_document_with_options(html, &ParserOptions::default())
}

/// Parse um documento HTML com opções personalizadas.
pub fn parse_document_with_options(html: &str, options: &ParserOptions) -> HtmlDocument {
    parse_document_with_errors_and_options(html, options).document
}

/// Parse um documento HTML e retorna tanto o resultado quanto os erros encontrados.
pub fn parse_document_with_errors(html: &str) -> ParseResult {
    parse_document_with_errors_and_options(html, &ParserOptions::default())
}

/// Alias para `parse_document_with_errors`.
pub fn build_document_with_errors(html: &str) -> ParseResult {
    parse_document_with_errors(html)
}

/// Parse um fragmento HTML (contexto específico) e retorna erros.
pub fn build_fragment_with_errors(html: &str, context: Option<&str>) -> ParseResult {
    let context = context.map(FragmentContext::new);
    html5ever_parser::parse_fragment_html5ever(html, context.as_ref(), &ParserOptions::default())
}

/// Parse um fragmento HTML e retorna apenas os nós filhos.
pub fn parse_fragment(html: &str, context: Option<&str>) -> Vec<HtmlNode> {
    let context = context.map(FragmentContext::new);
    parse_fragment_with_context(html, context.as_ref(), &ParserOptions::default())
}

/// Parse um fragmento HTML com contexto explícito (ex: elemento pai).
pub fn parse_fragment_with_context(
    html: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> Vec<HtmlNode> {
    parse_fragment_with_result(html, context, options)
        .document
        .children
}

/// Parse um fragmento HTML e retorna o resultado completo (DOM + erros + stats).
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

/// Parse um documento HTML completo com opções e retorna resultado detalhado.
/// Tenta usar o "fast path" primeiro para performance, fallback para html5ever.
pub fn parse_document_with_errors_and_options(html: &str, options: &ParserOptions) -> ParseResult {
    let parse_span = trace_span!(
        "ace_html.parse_document",
        input_bytes = html.len(),
        scripting_enabled = options.scripting_enabled
    );
    let _guard = parse_span.enter();
    let started = Instant::now();

    let mut fast_path_used = false;
    let mut result = if let Some(result) = fast_parse::try_fast_parse_document(html, options) {
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

/// Alias para `parse_document_with_errors_and_options`.
pub fn parse_html_integrated_with_options(html: &str, options: &ParserOptions) -> ParseResult {
    parse_document_with_errors_and_options(html, options)
}

/// Parse um documento HTML a partir de bytes brutos, lidando com decodificação de encoding.
pub fn parse_document_from_bytes_with_errors_and_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    let decoded = encoding::decode_html_bytes(bytes, bom, options.encoding_hint.clone())?;
    Ok(parse_document_with_errors_and_options(
        &decoded.content,
        options,
    ))
}

/// Alias para `parse_document_from_bytes_with_errors_and_options`.
pub fn parse_html_integrated_from_bytes_with_options(
    bytes: &[u8],
    bom: Option<&[u8]>,
    options: &ParserOptions,
) -> Result<ParseResult, ParseError> {
    parse_document_from_bytes_with_errors_and_options(bytes, bom, options)
}

/// Aplica estatísticas e telemetria ao resultado do parse.
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

/// Converte tokens brutos do html5gum para nossa estrutura interna `HtmlTokenKind`.
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

/// Decodifica bytes de texto para string, substituindo nulos por REPLACEMENT CHARACTER.
fn decode_text_token(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\0', "\u{FFFD}")
}

/// Detecta e extrai conteúdo de comentários CDATA legados (ex: <!--[CDATA[...]]-->).
fn decode_legacy_cdata_comment(data: &str) -> Option<String> {
    data.strip_prefix("[CDATA[")
        .and_then(|inner| inner.strip_suffix("]]"))
        .map(str::to_string)
}

/// Mapeia erros do html5gum para nossa estrutura de erro interna.
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

/// Detecta erros iniciais comuns, como DOCTYPE inválido.
pub fn detect_initial_errors(input: &str) -> Vec<ParseError> {
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

/// Cria um erro genérico para casos onde não há informação detalhada.
pub fn rough_error(
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

/// Calcula linha e coluna para um determinado offset em uma string.
pub fn line_column_for_offset(input: &str, offset: usize) -> (usize, usize) {
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

/// Transforma recursivamente elementos <noscript> conforme estado do scripting.
/// Com scripting habilitado, o conteúdo permanece como nós filhos (já processado pelo sink).
pub fn transform_noscript(nodes: Vec<HtmlNode>) -> Vec<HtmlNode> {
    nodes
        .into_iter()
        .filter_map(|node| match node {
            HtmlNode::Element(element) if element.tag == "noscript" => {
                // With scripting enabled, html5ever keeps the noscript element
                // and converts its children to text in the sink.
                // We keep the element as-is (already processed by sink.rs).
                Some(HtmlNode::Element(element))
            }
            HtmlNode::Element(mut element) => {
                element.children = transform_noscript(element.children);
                Some(HtmlNode::Element(element))
            }
            other => Some(other),
        })
        .collect()
}

/// Serializa os nós filhos de um elemento como uma única string de texto.
/// Usado internamente para implementar `.textContent` e similar.
pub fn serialize_children_as_text(children: &[HtmlNode]) -> String {
    let options = SerializeOptions {
        indent: None,
        escape_text: false,
    };
    let mut out = String::new();
    for child in children {
        out.push_str(&serialize_node(child, &options));
    }
    out
}
