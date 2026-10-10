use std::fmt;
use tracing::{event, Level};

/// Tipos de eventos estruturados do ciclo de vida de rede (Chromium NetLog style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetEventType {
    RequestStart,
    ResponseStart,
    DnsStart,
    DnsEnd,
    TcpConnectStart,
    TcpConnectEnd,
    TlsHandshakeStart,
    TlsHandshakeEnd,
    SendHeaders,
    RecvHeaders,
    RecvBody,
    CacheHit,
    CacheMiss,
    Redirect,
    Queue,
    Retry,
    Cancel,
    Warning,
    Error,
}

impl fmt::Display for NetEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::RequestStart => "REQUEST_START",
            Self::ResponseStart => "RESPONSE_START",
            Self::DnsStart => "DNS_START",
            Self::DnsEnd => "DNS_END",
            Self::TcpConnectStart => "TCP_CONNECT_START",
            Self::TcpConnectEnd => "TCP_CONNECT_END",
            Self::TlsHandshakeStart => "TLS_HANDSHAKE_START",
            Self::TlsHandshakeEnd => "TLS_HANDSHAKE_END",
            Self::SendHeaders => "SEND_HEADERS",
            Self::RecvHeaders => "RECV_HEADERS",
            Self::RecvBody => "RECV_BODY",
            Self::CacheHit => "CACHE_HIT",
            Self::CacheMiss => "CACHE_MISS",
            Self::Redirect => "REDIRECT",
            Self::Queue => "QUEUE",
            Self::Retry => "RETRY",
            Self::Cancel => "CANCEL",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
        };
        write!(f, "{}", s)
    }
}

/// Helper para emitir um evento padronizado de NetLog
#[inline]
pub fn log_net_event(event_type: NetEventType, url: &str, details: &str) {
    event!(
        Level::INFO,
        net_event = %event_type,
        url = %url,
        details = %details
    );
}

/// Helper para registrar erros de rede
#[inline]
pub fn log_net_error(event_type: NetEventType, url: &str, error: &dyn std::error::Error) {
    event!(
        Level::ERROR,
        net_event = %event_type,
        url = %url,
        error = %error
    );
}

use serde::Serialize;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::SystemTime;

/// Registro estruturado individual de evento de NetLog.
#[derive(Debug, Clone, Serialize)]
pub struct NetLogEntry {
    pub time: String,
    pub event_type: String,
    pub source_id: u64,
    pub url: String,
    pub details: String,
}

/// Coletor de eventos de rede estruturados no formato Chromium NetLog.
#[derive(Debug, Clone, Default)]
pub struct NetLogCollector {
    entries: Arc<RwLock<Vec<NetLogEntry>>>,
}

impl NetLogCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um novo evento estruturado no coletor.
    pub fn record(&self, source_id: u64, event_type: NetEventType, url: &str, details: &str) {
        let now = SystemTime::now();
        let time_str = httpdate::fmt_http_date(now);
        let entry = NetLogEntry {
            time: time_str,
            event_type: event_type.to_string(),
            source_id,
            url: url.to_string(),
            details: details.to_string(),
        };
        self.entries.write().push(entry);
    }

    /// Retorna a quantidade de eventos capturados.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Verifica se o coletor está vazio.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Limpa todos os eventos do coletor.
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Exporta os eventos acumulados no formato JSON padrão Chromium NetLog.
    pub fn export_chrome_netlog_json(&self) -> String {
        let entries = self.entries.read().clone();
        let payload = serde_json::json!({
            "constants": {
                "client": "Albedo Browser",
                "version": "0.1.0",
                "netLogVersion": 1
            },
            "events": entries
        });
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    }
}

