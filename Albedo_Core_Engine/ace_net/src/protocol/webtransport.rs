//! # WebTransport (RFC 9297)
//!
//! Implementação da API de WebTransport sobre conexões QUIC (HTTP/3).
//! Fornece comunicação bidirecional de baixa latência e suporte a datagramas não-confiáveis,
//! superando as limitações de Head-of-Line Blocking do WebSocket.

use crate::error::{NetError, NetResult};
use bytes::Bytes;

/// Representa uma sessão ativa de WebTransport.
pub struct WebTransportSession {
    // Encapsula as primitivas do Quinn (Streams e Datagrams)
    connection: quinn::Connection,
    #[allow(dead_code)]
    session_id: u64,
}

impl WebTransportSession {
    /// Inicializa uma nova sessão de WebTransport a partir de uma conexão QUIC estabelecida.
    pub fn new(connection: quinn::Connection, session_id: u64) -> Self {
        Self { connection, session_id }
    }

    /// Envia um datagrama não-confiável (fogo-e-esquece).
    pub async fn send_datagram(&self, data: Bytes) -> NetResult<()> {
        self.connection.send_datagram(data).map_err(|e| NetError::HttpProtocolError(format!("Failed to send datagram: {}", e)))
    }

    /// Recebe o próximo datagrama não-confiável.
    pub async fn receive_datagram(&self) -> NetResult<Bytes> {
        let datagram = self.connection.read_datagram().await.map_err(|e| NetError::HttpProtocolError(format!("Failed to read datagram: {}", e)))?;
        Ok(datagram)
    }

    /// Abre um novo stream bidirecional confiável iniciado pelo cliente.
    pub async fn open_bidirectional_stream(&self) -> NetResult<(quinn::SendStream, quinn::RecvStream)> {
        self.connection.open_bi().await.map_err(|e| NetError::HttpProtocolError(format!("Failed to open bidirectional stream: {}", e)))
    }

    /// Aceita um stream bidirecional confiável iniciado pelo servidor.
    pub async fn accept_bidirectional_stream(&self) -> NetResult<(quinn::SendStream, quinn::RecvStream)> {
        self.connection.accept_bi().await.map_err(|e| NetError::HttpProtocolError(format!("Failed to accept bidirectional stream: {}", e)))
    }
}
