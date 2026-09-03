//! # Tratamento de Contenção de Rede e Rate Limiting (RFC 9110 §10.2.3)
//!
//! Fornece parsing normativo e cálculo de espera para o cabeçalho `Retry-After`,
//! comumente emitido por servidores em respostas de contenção (`429 Too Many Requests`
//! e `503 Service Unavailable`).

use http::header::RETRY_AFTER;
use http::{HeaderMap, HeaderValue};
use std::time::{Duration, SystemTime};

/// Representação estruturada do cabeçalho `Retry-After`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAfter {
    /// Tempo de espera relativo em segundos (ex: `Retry-After: 120`).
    Seconds(Duration),
    /// Data/hora absoluta a partir da qual novas requisições podem ser enviadas (ex: `Retry-After: Wed, 21 Oct 2026 07:28:00 GMT`).
    DateTime(SystemTime),
}

impl RetryAfter {
    /// Extrai e analisa o cabeçalho `Retry-After` a partir de um `HeaderMap`.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let val = headers.get(RETRY_AFTER)?;
        Self::from_header_value(val)
    }

    /// Analisa o valor de um cabeçalho `Retry-After`.
    pub fn from_header_value(val: &HeaderValue) -> Option<Self> {
        let s = val.to_str().ok()?.trim();

        // 1. Tenta interpretar como número de segundos relativos (delta-seconds)
        if let Ok(secs) = s.parse::<u64>() {
            return Some(Self::Seconds(Duration::from_secs(secs)));
        }

        // 2. Tenta interpretar como HTTP-date absoluto (RFC 9110 §5.6.7)
        if let Ok(sys_time) = httpdate::parse_http_date(s) {
            return Some(Self::DateTime(sys_time));
        }

        None
    }

    /// Calcula a duração necessária de espera com base no horário atual.
    pub fn retry_delay(&self, now: SystemTime) -> Duration {
        match self {
            Self::Seconds(dur) => *dur,
            Self::DateTime(target_time) => {
                target_time.duration_since(now).unwrap_or(Duration::ZERO)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_after_delta_seconds() {
        let val = HeaderValue::from_static("60");
        let retry = RetryAfter::from_header_value(&val).unwrap();

        assert_eq!(retry, RetryAfter::Seconds(Duration::from_secs(60)));
        assert_eq!(retry.retry_delay(SystemTime::now()), Duration::from_secs(60));
    }

    #[test]
    fn test_retry_after_http_date() {
        let val = HeaderValue::from_static("Sun, 06 Nov 2039 08:49:37 GMT");
        let retry = RetryAfter::from_header_value(&val).unwrap();

        let now = SystemTime::now();
        assert!(matches!(retry, RetryAfter::DateTime(_)));
        assert!(retry.retry_delay(now) > Duration::from_secs(3600));
    }
}
