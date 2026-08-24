//! # Albedo Core Engine — ace_dom
//!
//! O motor de parsing HTML5 normativo (WHATWG HTML §12) e árvore DOM em arena geracional
//! de alta performance do **Albedo Browser**.
//!
//! ## Principais Recursos
//! - **HTML5 Tokenizer:** Máquina de Estados Finita (FSM) de streaming sobre `SegmentedString` com Fast-Path SIMD (`memchr3`).
//! - **Tree Builder:** Construção da árvore com modos de inserção normativos, tags `<template>`, Foreign Content SVG/MathML e AAA.
//! - **Árvore DOM em Arena:** Nós compactos de 40 bytes por link (`Arena<NodeData>`) sem vazamentos de ciclo.
//! - **Shadow DOM & Flat Tree:** Distribuição de slots, `ShadowRoot` e árvore achatada composta.
//! - **Fragment Parser & Serializer:** Parsing contextual (`innerHTML` setter) e serialização normativa (`outerHTML`).
//! - **MutationObserver:** Observação assíncrona de mutações em lote.
//! - **DOM Range & Traversal:** `Range`, `compareBoundaryPoints`, `TreeWalker` e `NodeIterator`.
//! - **Resolução $O(1)$ de Entidades:** Tabela estática perfeita PHF para todas as 2.231 entidades nomeadas.
//! - **Consultas & Índices:** `ElementIndex`, `getElementById`, `querySelector` com combinadores CSS.
//! - **Eventos DOM 3:** Pipeline de Captura, Target e Borbulhamento.

pub mod entities;
pub mod error;
pub mod events;
pub mod fragment;
pub mod node;
pub mod observer;
pub mod query;
pub mod range;
pub mod serializer;
pub mod tokenizer;
pub mod traversal;
pub mod tree;
pub mod tree_builder;

pub use entities::{decode_character_reference, resolve_named_entity, resolve_numeric_entity};
pub use error::DomError;
pub use events::{dispatch_event, Event, EventListener, EventPhase, EventRegistry};
pub use fragment::parse_fragment;
pub use node::{
    Attribute, CommentData, DoctypeData, DocumentData, DocumentMode, DOMTokenList, ElementData,
    FlatTreeResolver, Namespace, NodeData, NodeKind, ShadowMode, ShadowRootData, TextData,
};
pub use observer::{MutationObserver, MutationObserverInit, MutationRecord, MutationType};
pub use query::{
    Combinator, ComplexSelector, CompoundSelector, ElementIndex, PseudoClass, SimpleSelector,
};
pub use range::{BoundaryPoint, Range, RangeComparison};
pub use serializer::{serialize_inner_html, serialize_node};
pub use tokenizer::{HTMLTokenizer, Token, TokenSink, TokenizerAction, TokenizerState};
pub use traversal::{FilterResult, NodeFilter, NodeIterator, TreeWalker};
pub use tree::Document;
pub use tree_builder::{
    adjust_svg_attribute_name, adjust_svg_tag_name, ActiveFormattingElements, HTMLTreeBuilder,
    InsertionMode, StackOfOpenElements,
};

use ace_core::text::SegmentedString;

/// Faz o parsing de uma string HTML completa e retorna a árvore `Document` construída.
///
/// # Exemplo
/// ```
/// use ace_dom::parse_html;
///
/// let html = r#"<!DOCTYPE html>
/// <html>
///   <head><title>Albedo Engine</title></head>
///   <body>
///     <div id="app" class="container">
///       <h1>Olá, Albedo!</h1>
///     </div>
///   </body>
/// </html>"#;
///
/// let doc = parse_html(html);
/// assert!(doc.document_element.is_some());
/// assert!(doc.body.is_some());
///
/// let app_div = doc.get_element_by_id("app");
/// assert!(app_div.is_some());
/// ```
pub fn parse_html(html: &str) -> Document {
    let mut builder = HTMLTreeBuilder::new(None);
    let mut tokenizer = HTMLTokenizer::new();
    let mut input = SegmentedString::from(html);

    tokenizer.tokenize(&mut input, &mut builder);
    builder.finish()
}
