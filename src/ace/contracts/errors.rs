//! # ACE-Contracts: Unified Errors
//!
//! Define o sistema de erros padrão do Albedo. Todos os módulos externos
//! (JIT, Runtime, Bridge) devem emitir e consumir AlbedoError.
//!
//! > [!IMPORTANT]
//! > **ATUALIZAÇÃO QUASE SEMPRE**: À medida que novos subsistemas (ex: Graphics, WebGL)
//! > forem integrados, o AlbedoError evoluirá.

use crate::ace::json::JsonError;

/// Erro central unificado do Albedo Browser.
#[derive(Debug, Clone, PartialEq)]
pub enum AlbedoError {
    Json(JsonError),
    Url(String),
    Network(String),
    Jit(String),
    Internal(String),
}

/// Alias unificado para resultados na engine.
pub type AlbedoResult<T> = Result<T, AlbedoError>;

impl std::fmt::Display for AlbedoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlbedoError::Json(e) => write!(f, "JSON Error: {}", e),
            AlbedoError::Url(e) => write!(f, "URL Error: {}", e),
            AlbedoError::Network(e) => write!(f, "Network Error: {}", e),
            AlbedoError::Jit(e) => write!(f, "JIT Error: {}", e),
            AlbedoError::Internal(e) => write!(f, "Internal Engine Error: {}", e),
        }
    }
}

impl std::error::Error for AlbedoError {}

// Conversão automática de erros JSON para AlbedoError
impl From<JsonError> for AlbedoError {
    fn from(err: JsonError) -> Self {
        AlbedoError::Json(err)
    }
}
