//! # Estrutura de Tokens HTML5 (WHATWG §12.2.5)
//!
//! Tipagem estrita para todos os tipos de tokens emitidos pelo Tokenizer.

use crate::node::element::Attribute;
use ace_core::collections::InlineVec;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Token que representa uma tag de abertura (ex: `<div class="container">` ou `<img src="..." />`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartTagToken {
    pub name: Atom,
    pub self_closing: bool,
    pub attributes: InlineVec<Attribute, 4>,
}

impl StartTagToken {
    #[inline]
    pub fn new(name: impl Into<Atom>) -> Self {
        Self {
            name: name.into(),
            self_closing: false,
            attributes: InlineVec::new(),
        }
    }
}

/// Token que representa uma tag de fechamento (ex: `</div>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndTagToken {
    pub name: Atom,
}

impl EndTagToken {
    #[inline]
    pub fn new(name: impl Into<Atom>) -> Self {
        Self { name: name.into() }
    }
}

/// Token que representa uma declaração de tipo de documento (ex: `<!DOCTYPE html>`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DoctypeToken {
    pub name: Option<SmolStr>,
    pub public_identifier: Option<SmolStr>,
    pub system_identifier: Option<SmolStr>,
    pub force_quirks: bool,
}

/// O enum de tokens unificado emitido pelo `HTMLTokenizer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Doctype(DoctypeToken),
    StartTag(StartTagToken),
    EndTag(EndTagToken),
    Comment(SmolStr),
    Character(SmolStr),
    Null,
    Eof,
}
