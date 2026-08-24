//! # Dados de Texto, Comentários, Doctype e Documento
//!
//! Estruturas de nós não-elementos da árvore DOM.

use smol_str::SmolStr;

/// Modo de compatibilidade do documento (Quirks Mode) segundo a especificação WHATWG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentMode {
    #[default]
    NoQuirks,
    Quirks,
    LimitedQuirks,
}

/// Dados de um nó de Documento (`NodeKind::Document`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentData {
    pub mode: DocumentMode,
    pub title: Option<SmolStr>,
    pub url: Option<String>,
}

/// Dados de um nó Doctype (`NodeKind::DocumentType`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DoctypeData {
    pub name: SmolStr,
    pub public_id: Option<SmolStr>,
    pub system_id: Option<SmolStr>,
    pub force_quirks: bool,
}

/// Dados de um nó de Texto (`NodeKind::Text`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextData {
    pub data: SmolStr,
}

impl TextData {
    #[inline]
    pub fn new(data: impl Into<SmolStr>) -> Self {
        Self { data: data.into() }
    }
}

/// Dados de um nó de Comentário (`NodeKind::Comment`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommentData {
    pub data: SmolStr,
}

impl CommentData {
    #[inline]
    pub fn new(data: impl Into<SmolStr>) -> Self {
        Self { data: data.into() }
    }
}
