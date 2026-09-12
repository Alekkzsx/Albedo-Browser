use std::fmt;
use tracing::{event, Level};

/// Tipos de eventos estruturados do ciclo de vida de rede (Chromium NetLog style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetEventType {
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
    Cancel,
    Error,
}

impl fmt::Display for NetEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Formatter<'_> {
        let s = match self {
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
            Self::Cancel => "CANCEL",
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
