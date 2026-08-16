use thiserror::Error;

/// Tipo de erro central do Albedo Core Engine.
/// 
/// O `AceError` unifica falhas de todas as camadas do navegador,
/// permitindo que sejam propagadas e formatadas via `anyhow`.
#[derive(Error, Debug)]
pub enum AceError {
    #[error("Erro de I/O do sistema: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de parse: {message}")]
    ParseError { message: String },

    #[error("Erro de rede: {message}")]
    NetworkError { message: String },

    #[error("Erro de segurança: {message}")]
    SecurityError { message: String },

    #[error("Erro no motor JavaScript: {message}")]
    JsError { message: String },
    
    #[error("Recurso não encontrado: {0}")]
    NotFound(String),

    #[error("Operação inválida: {0}")]
    InvalidOperation(String),
}
