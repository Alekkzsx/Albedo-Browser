//! # Albedo Core Engine — ace_dom
//!
//! O motor de parsing HTML5 normativo (WHATWG HTML §12) e árvore DOM em arena geracional
//! de alta performance do **Albedo Browser**.
//!
//! ## Principais Recursos
//! - **HTML5 Tokenizer:** Máquina de Estados Finita (FSM) de streaming sobre `SegmentedString`.
//! - **Tree Builder:** Construção da árvore com modos de inserção normativos e Adoption Agency Algorithm (AAA).
//! - **Árvore DOM em Arena:** Nós compactos de 40 bytes por link (`Arena<NodeData>`) sem vazamentos de ciclo.
//! - **Resolução $O(1)$ de Entidades:** Tabela estática perfeita PHF para todas as 2.231 entidades nomeadas.
//! - **Consultas & Índices:** `ElementIndex`, `getElementById`, `querySelector`.
//! - **Eventos DOM 3:** Pipeline de Captura, Target e Borbulhamento.

pub mod entities;
pub mod error;
pub mod events;
pub mod node;
pub mod query;
pub mod tokenizer;
pub mod tree;
pub mod tree_builder;

pub use entities::{decode_character_reference, resolve_named_entity, resolve_numeric_entity};
pub use error::DomError;
pub use events::{dispatch_event, Event, EventListener, EventPhase, EventRegistry};
pub use node::{
    Attribute, CommentData, DoctypeData, DocumentData, DocumentMode, DOMTokenList, ElementData,
    Namespace, NodeData, NodeKind, ShadowMode, ShadowRootData, TextData,
};
pub use query::{ElementIndex, SimpleSelector};
pub use tokenizer::{HTMLTokenizer, Token, TokenSink, TokenizerAction, TokenizerState};
pub use tree::Document;
pub use tree_builder::{ActiveFormattingElements, HTMLTreeBuilder, InsertionMode, StackOfOpenElements};

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
