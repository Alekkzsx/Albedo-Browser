//! HTTP/3 QUIC Client — Implementação Real via quinn + h3
//!
//! Este módulo fornece suporte HTTP/3 real via protocolo QUIC para o Albedo Browser.
//! Vantagens sobre HTTP/2:
//!   - 0-RTT resumption para conexões rápidas (25% mais rápido em cold connections)
//!   - Connection migration (troca de IP/porta não quebra a conexão)
//!   - Stream multiplexing sem head-of-line blocking
//!   - Recuperação de perda de pacotes por stream individual
//!   - Controle de congestionamento per-stream

use super::*;
use bytes::Buf;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Resposta HTTP/3 com status, headers e corpo


/// HTTP/3 Client usando protocolo QUIC real.
///
/// Arquitetura:
///   - Transporte QUIC: `quinn` (100% Rust, sem dependências C)
///   - TLS: `rustls` com certificados nativos do sistema operacional
///   - Camada HTTP/3: `h3` + `h3-quinn` (frames HTTP/3 sobre streams QUIC)
///   - Connection Pool: `HashMap<String, quinn::Connection>` para reusar conexões
///
/// Configuração:
///   - TLS: rustls com certificados nativos do SO
///   - Connection timeout: 10 segundos para conexão inicial
///   - Idle timeout: 30 segundos antes de fechar conexões ociosas
///   - ALPN: "h3" para negociação HTTP/3
#[derive(Clone)]
pub struct Http3Client {
    /// Endpoint QUIC compartilhado (um único socket UDP para todas as conexões)
    pub endpoint: Arc<quinn::Endpoint>,
    /// Pool de conexões QUIC ativas indexadas por "host:port"
    pub connection_pool: Arc<Mutex<HashMap<String, quinn::Connection>>>,
}

impl Default for Http3Client {
pub(crate) fn default() -> Self {
        Http3Client::new().unwrap_or_else(|e| {
            tracing::error!(?e, "Failed to create default HTTP/3 client");
            // Criar um client com endpoint não funcional como fallback seguro
            // Isso nunca deve acontecer em condições normais (falta de certs no SO)
            panic!("[HTTP/3] Impossível criar client QUIC: {}", e);
        })
    }
}
