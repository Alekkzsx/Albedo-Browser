//! # Erros de IPC In-Process (`IpcError`)

use thiserror::Error;

/// Erros originados nas operações de canal e roteamento de IPC.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum IpcError {
    /// Canal IPC desconectado / fechado.
    #[error("Canal IPC desconectado / fechado")]
    Disconnected,

    /// Canal IPC cheio (capacidade esgotada).
    #[error("Canal IPC cheio (capacidade esgotada)")]
    Full,

    /// Canal IPC vazio (nenhuma mensagem disponível).
    #[error("Canal IPC vazio (nenhuma mensagem disponível)")]
    Empty,

    /// Timeout expirado ao aguardar mensagem IPC.
    #[error("Timeout expirado ao aguardar mensagem IPC")]
    Timeout,

    /// Tópico de IPC especificado não foi encontrado.
    #[error("Tópico de IPC '{0}' não encontrado")]
    TopicNotFound(String),
}
