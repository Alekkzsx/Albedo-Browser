//! # Albedo Core Engine — ace_dom
//!
//! O motor de parsing HTML5 normativo (WHATWG HTML §12) e árvore DOM em arena geracional
//! de alta performance do **Albedo Browser**.
//!
//! ## Principais Recursos
//! - **HTML5 Tokenizer:** Máquina de Estados Finita (FSM) de streaming sobre `SegmentedString` com Fast-Path SIMD (`memchr3`).
//! - **Preload Scanner:** Lookahead não-bloqueante de sub-recursos (`<link>`, `<script>`, `<img>`, etc.).
//! - **Tree Builder:** Modos de inserção normativos, tags `<template>`, Declarative Shadow DOM (DSD), Foreign Content SVG/MathML e AAA.
//! - **Árvore DOM em Arena:** Nós compactos de 40 bytes por link (`Arena<NodeData>`) sem vazamentos de ciclo.
//! - **Shadow DOM & Flat Tree:** Distribuição de slots, `ShadowRoot` e árvore achatada composta.
//! - **Fragment Parser & Serializer:** Parsing contextual (`innerHTML` setter) e serialização normativa (`outerHTML`).
//! - **MutationObserver:** Observação assíncrona de mutações em lote.
//! - **DOM Range & Traversal:** `Range`, `compareDocumentPosition`, `TreeWalker` e `NodeIterator`.
//! - **Counting Bloom Filter:** Rejeição instantânea em $O(1)$ de seletores CSS descendentes (`AncestorFilter`).
//! - **Forms & ValidityState:** Validação de restrições de formulário e serialização de `FormData`.
//! - **Custom Elements:** Registro de Web Components e fila de reações de ciclo de vida.
//! - **HTML Sanitizer:** Defesas nativas contra injeção de código malicioso e ataques XSS.
//! - **DOMParser:** API universal com suporte a tipos MIME (`text/html`, `image/svg+xml`, `application/xml`).
//! - **Consultas & Índices:** `ElementIndex`, `getElementById`, `querySelector` com combinadores CSS e `dataset`.
//! - **Eventos DOM 3/4:** Pipeline de Captura, Target e Borbulhamento com `AddEventListenerOptions` e `AbortSignal`.

pub mod custom_elements;
pub mod dom_parser;
pub mod entities;
pub mod error;
pub mod events;
pub mod form;
pub mod fragment;
pub mod node;
pub mod observer;
pub mod preload_scanner;
pub mod query;
pub mod range;
pub mod sanitizer;
pub mod serializer;
pub mod tokenizer;
pub mod traversal;
pub mod tree;
pub mod tree_builder;

pub use custom_elements::{
    is_valid_custom_element_name, CustomElementDefinition, CustomElementRegistry, LifecycleQueue,
    LifecycleReaction,
};
pub use dom_parser::{DOMParser, SupportedType};
pub use entities::{decode_character_reference, resolve_named_entity, resolve_numeric_entity};
pub use error::DomError;
pub use events::{
    dispatch_event, AddEventListenerOptions, Event, EventListener, EventPhase, EventRegistry,
};
pub use form::{
    check_control_validity, FormAssociation, FormData, FormDataEntry, FormDataValue, ValidityState,
};
pub use fragment::parse_fragment;
pub use node::{
    Attribute, CommentData, DoctypeData, DocumentData, DocumentMode, DocumentPosition,
    DOMStringMap, DOMTokenList, ElementData, ElementRareData, FlatTreeResolver, Namespace,
    NodeData, NodeKind, ShadowMode, ShadowRootData, TextData,
};
pub use observer::{MutationObserver, MutationObserverInit, MutationRecord, MutationType};
pub use preload_scanner::{PreloadKind, PreloadRequest, PreloadScanner};
pub use query::{
    AncestorFilter, AttributeOp, Combinator, ComplexSelector, CompoundSelector, ElementIndex,
    PseudoClass, RuleBucketIndex, SimpleSelector,
};
pub use range::{BoundaryPoint, LiveRangeHandle, LiveRangeRegistry, Range, RangeComparison};
pub use sanitizer::{HTMLSanitizer, SanitizerConfig};
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
