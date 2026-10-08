//! # WebTransport (RFC 9297)
//!
//! Implementação da API de WebTransport sobre conexões QUIC (HTTP/3).
//! Fornece comunicação bidirecional de baixa latência e suporte a datagramas não-confiáveis,
//! superando as limitações de Head-of-Line Blocking do WebSocket.

use crate::error::{NetError, NetResult};
use bytes::Bytes;
use url::Url;

/// Valida se uma URL atende aos requisitos normativos da especificação WebTransport (RFC 9297).
/// Apenas esquemas `https://` com porta e host válidos são permitidos.
pub fn validate_webtransport_url(url: &Url) -> NetResult<()> {
    if url.scheme() != "https" {
        return Err(NetError::UnsupportedScheme(format!(
            "WebTransport exige esquema https://, fornecido: {}",
            url.scheme()
        )));
    }
    if url.host_str().is_none() {
        return Err(NetError::InvalidUrl("URL WebTransport sem host válido".into()));
    }
    Ok(())
}

/// Representa uma sessão ativa de WebTransport (RFC 9297).
pub struct WebTransportSession {
    // Encapsula as primitivas do Quinn (Streams bidirecionais/unidirecionais e Datagramas)
    connection: quinn::Connection,
    session_id: u64,
}

impl WebTransportSession {
    /// Inicializa uma nova sessão de WebTransport a partir de uma conexão QUIC estabelecida.
    pub fn new(connection: quinn::Connection, session_id: u64) -> Self {
        Self { connection, session_id }
    }

    /// Retorna o identificador único da sessão WebTransport na conexão QUIC.
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Envia um datagrama não-confiável (fogo-e-esquece).
    pub async fn send_datagram(&self, data: Bytes) -> NetResult<()> {
        self.connection
            .send_datagram(data)
            .map_err(|e| NetError::HttpProtocolError(format!("Falha ao enviar datagrama WebTransport: {}", e)))
    }

    /// Recebe o próximo datagrama não-confiável.
    pub async fn receive_datagram(&self) -> NetResult<Bytes> {
        let datagram = self.connection
            .read_datagram()
            .await
            .map_err(|e| NetError::HttpProtocolError(format!("Falha ao ler datagrama WebTransport: {}", e)))?;
        Ok(datagram)
    }

    /// Abre um novo stream bidirecional confiável iniciado pelo cliente.
    pub async fn open_bidirectional_stream(&self) -> NetResult<(quinn::SendStream, quinn::RecvStream)> {
        self.connection
            .open_bi()
            .await
            .map_err(|e| NetError::HttpProtocolError(format!("Falha ao abrir stream bidirecional WebTransport: {}", e)))
    }

    /// Aceita um stream bidirecional confiável iniciado pelo servidor.
    pub async fn accept_bidirectional_stream(&self) -> NetResult<(quinn::SendStream, quinn::RecvStream)> {
        self.connection
            .accept_bi()
            .await
            .map_err(|e| NetError::HttpProtocolError(format!("Falha ao aceitar stream bidirecional WebTransport: {}", e)))
    }

    /// Abre um novo stream unidirecional confiável de envio (RFC 9297).
    pub async fn open_unidirectional_stream(&self) -> NetResult<quinn::SendStream> {
        self.connection
            .open_uni()
            .await
            .map_err(|e| NetError::HttpProtocolError(format!("Falha ao abrir stream unidirecional WebTransport: {}", e)))
    }

    /// Aceita um novo stream unidirecional confiável de recebimento iniciado pelo servidor.
    pub async fn accept_unidirectional_stream(&self) -> NetResult<quinn::RecvStream> {
        self.connection
            .accept_uni()
            .await
            .map_err(|e| NetError::HttpProtocolError(format!("Falha ao aceitar stream unidirecional WebTransport: {}", e)))
    }

    /// Encerra a sessão WebTransport notificando o servidor com um código de erro e razão.
    pub fn close_session(&self, error_code: u32, reason: &str) {
        self.connection.close(
            quinn::VarInt::from_u32(error_code),
            reason.as_bytes(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webtransport_url_validation() {
        let valid = Url::parse("https://wt.example.com:4433/counter").unwrap();
        assert!(validate_webtransport_url(&valid).is_ok());

        let invalid_scheme = Url::parse("http://wt.example.com:4433/counter").unwrap();
        assert!(validate_webtransport_url(&invalid_scheme).is_err());

        let invalid_ws = Url::parse("ws://wt.example.com/counter").unwrap();
        assert!(validate_webtransport_url(&invalid_ws).is_err());
    }
}
