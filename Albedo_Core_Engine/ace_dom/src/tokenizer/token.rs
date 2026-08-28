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

/// Token compacto e serializável otimizado para streaming assíncrono entre threads (Blink CompactHTMLToken pattern).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactHTMLToken {
    Doctype {
        name: Option<SmolStr>,
        public_id: Option<SmolStr>,
        system_id: Option<SmolStr>,
        force_quirks: bool,
    },
    StartTag {
        name: Atom,
        self_closing: bool,
        attributes: InlineVec<Attribute, 4>,
    },
    EndTag {
        name: Atom,
    },
    Comment(SmolStr),
    Character(SmolStr),
    Eof,
}

impl CompactHTMLToken {
    /// Converte um `Token` tradicional em `CompactHTMLToken` (ignorando `Token::Null`).
    pub fn from_token(tok: Token) -> Option<Self> {
        match tok {
            Token::Doctype(d) => Some(CompactHTMLToken::Doctype {
                name: d.name,
                public_id: d.public_identifier,
                system_id: d.system_identifier,
                force_quirks: d.force_quirks,
            }),
            Token::StartTag(s) => Some(CompactHTMLToken::StartTag {
                name: s.name,
                self_closing: s.self_closing,
                attributes: s.attributes,
            }),
            Token::EndTag(e) => Some(CompactHTMLToken::EndTag { name: e.name }),
            Token::Comment(c) => Some(CompactHTMLToken::Comment(c)),
            Token::Character(c) => Some(CompactHTMLToken::Character(c)),
            Token::Eof => Some(CompactHTMLToken::Eof),
            Token::Null => None,
        }
    }

    /// Converte um `CompactHTMLToken` de volta para o `Token` do WHATWG TreeBuilder.
    #[inline]
    pub fn to_token(self) -> Token {
        match self {
            CompactHTMLToken::Doctype {
                name,
                public_id,
                system_id,
                force_quirks,
            } => Token::Doctype(DoctypeToken {
                name,
                public_identifier: public_id,
                system_identifier: system_id,
                force_quirks,
            }),
            CompactHTMLToken::StartTag {
                name,
                self_closing,
                attributes,
            } => Token::StartTag(StartTagToken {
                name,
                self_closing,
                attributes,
            }),
            CompactHTMLToken::EndTag { name } => Token::EndTag(EndTagToken { name }),
            CompactHTMLToken::Comment(c) => Token::Comment(c),
            CompactHTMLToken::Character(c) => Token::Character(c),
            CompactHTMLToken::Eof => Token::Eof,
        }
    }

    /// Retorna o nome da tag se for `StartTag` ou `EndTag`.
    #[inline]
    pub fn tag_name(&self) -> Option<&Atom> {
        match self {
            CompactHTMLToken::StartTag { name, .. } => Some(name),
            CompactHTMLToken::EndTag { name } => Some(name),
            _ => None,
        }
    }

    /// Retorna `true` se for uma tag de abertura.
    #[inline]
    pub fn is_start_tag(&self) -> bool {
        matches!(self, CompactHTMLToken::StartTag { .. })
    }

    /// Retorna `true` se for EOF.
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self, CompactHTMLToken::Eof)
    }
}
