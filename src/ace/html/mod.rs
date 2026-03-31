#![deny(warnings)]

use std::collections::HashMap;

pub mod lexer;
pub mod entities;
pub mod tokenizer;
pub mod tree_builder;
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
}

impl HtmlElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            namespace: Namespace::Html,
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn with_namespace(tag: impl Into<String>, ns: Namespace) -> Self {
        Self {
            tag: tag.into(),
            namespace: ns,
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }
}

pub fn parse_document(input: &str) -> HtmlDocument {
    build_document(input)
}

pub fn parse_fragment(input: &str, context: Option<&str>) -> Vec<HtmlNode> {
    build_fragment(input, context)
}

// Testes migrados para o diretório tests/
