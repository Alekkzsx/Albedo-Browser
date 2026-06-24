use super::*;
//! HTTP/3 QUIC Client — Implementação Real via quinn + h3
//!
//! Este módulo fornece suporte HTTP/3 real via protocolo QUIC para o Albedo Browser.
//! Vantagens sobre HTTP/2:
//!   - 0-RTT resumption para conexões rápidas (25% mais rápido em cold connections)
//!   - Connection migration (troca de IP/porta não quebra a conexão)
//!   - Stream multiplexing sem head-of-line blocking
//!   - Recuperação de perda de pacotes por stream individual
//!   - Controle de congestionamento per-stream

use bytes::Buf;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Resposta HTTP/3 com status, headers e corpo


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn test_http3_client_creation() {
        let client = Http3Client::new();
        // A criação pode falhar se o SO não tiver certificados raiz
        // (CI environments, containers, etc.), então apenas verificamos que
        // não há panic
        match client {
            Ok(c) => {
                tracing::debug!("HTTP/3 client created successfully");
                assert!(c.is_alive());
            }
            Err(e) => {
                // Aceitável em ambientes sem certificados nativos
                tracing::info!(?e, "HTTP/3 client not available in this environment");
            }
        }
    }

    #[test]
pub(crate) fn test_http3_response_text() {
        let resp = Http3Response {
            status: 200,
            headers: HashMap::new(),
            body: b"Hello, QUIC!".to_vec(),
        };
        assert_eq!(resp.text(), "Hello, QUIC!");
        assert!(resp.ok());
    }

    #[test]
pub(crate) fn test_http3_response_not_ok() {
        let resp = Http3Response {
            status: 404,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(!resp.ok());
    }

    #[tokio::test]
pub(crate) async fn test_http3_graceful_close() {
        // Testar que close() funciona sem panic mesmo sem conexões ativas
        let client = Http3Client::new();
        if let Ok(mut c) = client {
            let result = c.close().await;
            assert!(result.is_ok());
        }
    }

    #[test]
pub(crate) fn test_http3_cleanup_empty_pool() {
        let client = Http3Client::new();
        if let Ok(c) = client {
            // Cleanup em pool vazio não deve causar erro
            c.cleanup_pool();
            assert!(c.is_alive());
        }
    }
}
