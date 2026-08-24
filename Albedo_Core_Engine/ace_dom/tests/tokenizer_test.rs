//! # Bateria de Testes do Tokenizer HTML5 (ace_dom)

use ace_core::text::SegmentedString;
use ace_dom::tokenizer::{HTMLTokenizer, Token, TokenSink};

#[derive(Default)]
struct TestSink {
    tokens: Vec<Token>,
}

impl TokenSink for TestSink {
    fn process_token(&mut self, token: Token) {
        self.tokens.push(token);
    }
}

#[test]
fn test_tokenize_basic_tag() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestSink::default();
    let mut input = SegmentedString::from("<div id=\"main\" class='box'>Olá</div>");

    tokenizer.tokenize(&mut input, &mut sink);

    assert!(sink.tokens.len() >= 3);
    match &sink.tokens[0] {
        Token::StartTag(tag) => {
            assert_eq!(tag.name.as_str(), "div");
            assert_eq!(tag.attributes.len(), 2);
            assert_eq!(tag.attributes[0].name.as_str(), "id");
            assert_eq!(tag.attributes[0].value.as_str(), "main");
            assert_eq!(tag.attributes[1].name.as_str(), "class");
            assert_eq!(tag.attributes[1].value.as_str(), "box");
        }
        _ => panic!("Esperado StartTag"),
    }
}

#[test]
fn test_tokenize_self_closing_tag() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestSink::default();
    let mut input = SegmentedString::from("<img src=\"avatar.png\" alt=\"Foto\" />");

    tokenizer.tokenize(&mut input, &mut sink);

    match &sink.tokens[0] {
        Token::StartTag(tag) => {
            assert_eq!(tag.name.as_str(), "img");
            assert!(tag.self_closing);
            assert_eq!(tag.attributes.len(), 2);
            assert_eq!(tag.attributes[0].name.as_str(), "src");
            assert_eq!(tag.attributes[0].value.as_str(), "avatar.png");
        }
        _ => panic!("Esperado StartTag para img"),
    }
}

#[test]
fn test_tokenize_comments_and_doctype() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestSink::default();
    let mut input = SegmentedString::from("<!DOCTYPE html><!-- Meu Comentário --><span></span>");

    tokenizer.tokenize(&mut input, &mut sink);

    match &sink.tokens[0] {
        Token::Doctype(dt) => {
            assert_eq!(dt.name.as_ref().map(|s| s.as_str()), Some("html"));
            assert!(!dt.force_quirks);
        }
        _ => panic!("Esperado Doctype"),
    }

    match &sink.tokens[1] {
        Token::Comment(c) => {
            assert_eq!(c.as_str(), " Meu Comentário ");
        }
        _ => panic!("Esperado Comment"),
    }
}

#[test]
fn test_tokenize_entities_in_text_and_attributes() {
    let mut tokenizer = HTMLTokenizer::new();
    let mut sink = TestSink::default();
    let mut input = SegmentedString::from("<p title=\"A&amp;B\">Copyright &copy; 2026</p>");

    tokenizer.tokenize(&mut input, &mut sink);

    match &sink.tokens[0] {
        Token::StartTag(tag) => {
            assert_eq!(tag.attributes[0].value.as_str(), "A&B");
        }
        _ => panic!("Esperado StartTag com entidade decodificada"),
    }

    // Coleta caracteres de texto emitidos
    let mut text = String::new();
    for t in &sink.tokens[1..] {
        if let Token::Character(c) = t {
            text.push_str(c.as_str());
        }
    }
    assert!(text.contains("Copyright © 2026"));
}
