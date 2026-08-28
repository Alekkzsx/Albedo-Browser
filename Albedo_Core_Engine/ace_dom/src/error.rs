//! # Erros do Motor DOM e Parser HTML5 (ace_dom)
//!
//! Tipos de erro específicos para parsing, mutação e navegação na árvore DOM.

use ace_core::error::SourceLocation;
use thiserror::Error;

/// Erros estruturados do motor DOM e Parser HTML5.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomError {
    #[error("Erro de Parsing HTML em {location:?}: {message}")]
    ParseError {
        message: String,
        location: Option<SourceLocation>,
    },

    #[error("Nó inválido ou inexistente na Arena: {0:?}")]
    InvalidNodeId(ace_core::id::NodeId),

    #[error("Operação de hierarquia inválida: {0}")]
    HierarchyRequestError(String),

    #[error("Erro de seletor CSS inválido: '{0}'")]
    SyntaxError(String),

    #[error("Nó não encontrado como filho do elemento especificado")]
    NotFoundError,

    #[error("O índice ou deslocamento fornecido está fora dos limites válidos (IndexSizeError)")]
    IndexSizeError,

    #[error("Operação não suportada ou não implementada no DOM: {0}")]
    NotSupportedError(String),
}

impl DomError {
    /// Cria um erro de parsing associado a uma mensagem e localização opcional.
    pub fn parse(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::ParseError {
            message: message.into(),
            location,
        }
    }
}
