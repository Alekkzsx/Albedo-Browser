//! # WebSocket e Streaming Reativo (RFC 6455)
//!
//! Gerenciamento de conexões bidirecionais de baixa latência (WebSockets)
//! com suporte a enquadramento de frames, handshake HTTP Upgrade e canais assíncronos.

use crate::error::{NetError, NetResult};
use crate::request::Request;
use bytes::Bytes;
use http::header::{CONNECTION, UPGRADE};
use http::{HeaderMap, HeaderValue, Method};
use std::sync::Arc;
use tokio::sync::mpsc;
use url::Url;

/// Tipo de mensagem WebSocket segundo a RFC 6455.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebSocketMessage {
    /// Mensagem de texto UTF-8.
    Text(String),
    /// Mensagem binária arbitrária.
    Binary(Bytes),
    /// Frame de controle Ping.
    Ping(Bytes),
    /// Frame de controle Pong.
    Pong(Bytes),
    /// Frame de encerramento de conexão (código de status opcional e motivo).
    Close(Option<u16>, String),
}

/// Sessão WebSocket ativa conectada à camada JS / DOM.
pub struct WebSocketSession {
    pub url: Url,
    tx_sender: mpsc::Sender<WebSocketMessage>,
    rx_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<WebSocketMessage>>>,
}

impl WebSocketSession {
    /// Cria uma nova sessão com canais bidirecionais de mensageria.
    pub fn new(
        url: Url,
        tx_sender: mpsc::Sender<WebSocketMessage>,
        rx_receiver: mpsc::Receiver<WebSocketMessage>,
    ) -> Self {
        Self {
            url,
            tx_sender,
            rx_receiver: Arc::new(tokio::sync::Mutex::new(rx_receiver)),
        }
    }

    /// Envia uma mensagem pelo socket.
    pub async fn send(&self, msg: WebSocketMessage) -> NetResult<()> {
        self.tx_sender
            .send(msg)
            .await
            .map_err(|_| NetError::ConnectionFailed(self.url.host_str().unwrap_or("").into(), "WebSocket fechado".into()))
    }

    /// Recebe a próxima mensagem assíncrona do socket.
    pub async fn recv(&self) -> Option<WebSocketMessage> {
        let mut rx = self.rx_receiver.lock().await;
        rx.recv().await
    }
}

/// Construtor e orquestrador de handshake WebSocket RFC 6455.
pub struct WebSocketUpgrade {
    pub url: Url,
    pub headers: HeaderMap,
}

impl WebSocketUpgrade {
    /// Cria um iniciador de handshake a partir de uma URL (esquemas `ws://` ou `wss://`).
    pub fn new(url: Url) -> NetResult<Self> {
        if url.scheme() != "ws" && url.scheme() != "wss" {
            return Err(NetError::UnsupportedScheme(url.scheme().to_string()));
        }

        let mut headers = HeaderMap::new();
        headers.insert(UPGRADE, HeaderValue::from_static("websocket"));
        headers.insert(CONNECTION, HeaderValue::from_static("Upgrade"));
        headers.insert(
            http::header::HeaderName::from_static("sec-websocket-version"),
            HeaderValue::from_static("13"),
        );

        Ok(Self { url, headers })
    }

    /// Gera uma chave Sec-WebSocket-Key codificada (RFC 6455 §4.1).
    pub fn generate_nonce() -> String {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let bytes = nanos.to_le_bytes();
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::with_capacity(24);
        for &b in &bytes {
            s.push(CHARS[(b as usize) % CHARS.len()] as char);
            s.push(CHARS[((b >> 2) as usize) % CHARS.len()] as char);
        }
        while s.len() < 22 {
            s.push('A');
        }
        s.push_str("==");
        s
    }

    /// Prepara os cabeçalhos de requisição de Upgrade HTTP.
    pub fn build_handshake_request(&self) -> Request {
        let mut http_url = self.url.clone();
        if http_url.scheme() == "ws" {
            let _ = http_url.set_scheme("http");
        } else if http_url.scheme() == "wss" {
            let _ = http_url.set_scheme("https");
        }

        let mut req_builder = Request::builder(http_url, Method::GET).unwrap();
        for (k, v) in &self.headers {
            req_builder = req_builder.header(k.clone(), v.clone());
        }

        let nonce = Self::generate_nonce();
        if let Ok(val) = HeaderValue::from_str(&nonce) {
            req_builder = req_builder.header(http::header::HeaderName::from_static("sec-websocket-key"), val);
        }

        req_builder.build()
    }

    /// Valida se a resposta HTTP do servidor confirma o Upgrade (Status 101 Switching Protocols).
    pub fn validate_handshake_response(status: http::StatusCode, headers: &HeaderMap) -> bool {
        if status != http::StatusCode::SWITCHING_PROTOCOLS {
            return false;
        }

        let upgrade_match = headers
            .get(UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);

        let conn_match = headers
            .get(CONNECTION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_ascii_lowercase().contains("upgrade"))
            .unwrap_or(false);

        upgrade_match && conn_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_upgrade_handshake_creation() {
        let url = Url::parse("wss://stream.example.com/feed").unwrap();
        let upgrade = WebSocketUpgrade::new(url).unwrap();
        let req = upgrade.build_handshake_request();

        assert_eq!(req.headers.get(UPGRADE).unwrap(), "websocket");
        assert_eq!(req.headers.get(CONNECTION).unwrap(), "Upgrade");
        assert_eq!(req.headers.get("sec-websocket-version").unwrap(), "13");
        assert!(req.headers.contains_key("sec-websocket-key"));
    }

    #[test]
    fn test_websocket_scheme_validation() {
        let http_url = Url::parse("http://example.com").unwrap();
        assert!(WebSocketUpgrade::new(http_url).is_err());

        let ws_url = Url::parse("ws://example.com/socket").unwrap();
        assert!(WebSocketUpgrade::new(ws_url).is_ok());
    }

    #[test]
    fn test_websocket_handshake_validation() {
        let mut valid_headers = HeaderMap::new();
        valid_headers.insert(UPGRADE, HeaderValue::from_static("websocket"));
        valid_headers.insert(CONNECTION, HeaderValue::from_static("Upgrade"));

        assert!(WebSocketUpgrade::validate_handshake_response(
            http::StatusCode::SWITCHING_PROTOCOLS,
            &valid_headers
        ));

        // Status 200 não é upgrade válido
        assert!(!WebSocketUpgrade::validate_handshake_response(
            http::StatusCode::OK,
            &valid_headers
        ));
    }

    #[tokio::test]
    async fn test_websocket_session_messaging() {
        let (tx, _rx) = mpsc::channel(10);
        let (server_tx, client_rx) = mpsc::channel(10);
        let url = Url::parse("ws://test.local").unwrap();

        let session = WebSocketSession::new(url, tx, client_rx);

        // Teste de envio
        session.send(WebSocketMessage::Text("ping".into())).await.unwrap();

        // Teste de recebimento
        server_tx.send(WebSocketMessage::Text("pong".into())).await.unwrap();
        let received = session.recv().await.unwrap();
        assert_eq!(received, WebSocketMessage::Text("pong".into()));
    }
}

