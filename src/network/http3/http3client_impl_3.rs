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


impl Http3Client {

    /// Atalho para requisição GET simples via HTTP/3.
    /// Retorna a resposta completa ou erro se a conexão/request falhar.
    pub async fn get(&self, url: &str) -> Result<Http3Response, Box<dyn Error + Send + Sync>> {
        self.send_request("GET", url, vec![], None).await
    }

    /// Verifica se o endpoint QUIC está ativo e funcional.
    pub fn is_alive(&self) -> bool {
        // O endpoint é válido se ainda não foi fechado
        // Verificar se há pelo menos uma conexão ativa no pool
        let pool = self.connection_pool.lock().unwrap_or_else(|e| e.into_inner());
        if pool.is_empty() {
            // Sem conexões ativas, mas endpoint pode estar pronto
            return true;
        }
        // Pelo menos uma conexão sem close_reason = vivo
        pool.values().any(|conn| conn.close_reason().is_none())
    }

    /// Fecha todas as conexões QUIC e limpa o pool.
    ///
    /// As conexões são fechadas de forma graciosa (envia CLOSE frame antes).
    pub async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        // 1. Fechar todas as conexões do pool
        let connections: Vec<quinn::Connection> = {
            let mut pool = self.connection_pool.lock().unwrap_or_else(|e| e.into_inner());
            let conns: Vec<_> = pool.values().cloned().collect();
            pool.clear();
            conns
        };

        for conn in connections {
            // Enviar CLOSE frame com código H3_NO_ERROR (0x100)
            conn.close(quinn::VarInt::from_u32(0x100), b"closed by client");
        }

        // 2. Aguardar o endpoint ficar idle (todas as conexões efetivamente fechadas)
        self.endpoint.wait_idle().await;

        tracing::info!("HTTP/3 QUIC client shut down");
        Ok(())
    }

    /// Remove conexões mortas do pool (limpeza periódica).
    pub fn cleanup_pool(&self) {
        let mut pool = self.connection_pool.lock().unwrap_or_else(|e| e.into_inner());
        pool.retain(|key, conn| {
            let alive = conn.close_reason().is_none();
            if !alive {
                tracing::debug!(key = %key, "Removing dead connection from pool");
            }
            alive
        });
    }
}
