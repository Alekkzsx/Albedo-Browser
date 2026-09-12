//! # WebSocket e Server-Sent Events (SSE)
//!
//! Handlers para upgrades de conexão de streaming reativo. O `ace_net`
//! fornece a infraestrutura de rede, enquanto a camada de DOM/JS vai
//! se registrar para enviar e receber frames de WebSocket.

use crate::error::NetResult;
use crate::request::Request;

/// Representa a tentativa de upgrade para WebSocket.
pub struct WebSocketUpgrade {
    pub request: Request,
}

impl WebSocketUpgrade {
    /// Inicia o handshake WebSocket (RFC 6455).
    /// O retorno disso posteriormente será um Stream reativo integrado ao `hyper`.
    pub async fn connect(self) -> NetResult<()> {
        // TODO: Integração com tungstenite sobre a conexão tcp upgreada do hyper.
        Ok(())
    }
}
