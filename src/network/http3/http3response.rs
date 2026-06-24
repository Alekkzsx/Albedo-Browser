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

#[derive(Debug, Clone)]
pub struct Http3Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Http3Response {
    /// Retorna o corpo como texto UTF-8
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// Retorna se a resposta foi bem-sucedida (200-299)
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}
