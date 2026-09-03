//! # Modelagem de Respostas de Rede (`Response` & `ResponseBody`)
//!
//! Representa os metadados HTTP, cabeçalhos, métricas de performance (Navigation Timing)
//! e stream assíncrono do corpo de bytes recebido.

use crate::compression::ContentEncoding;
use crate::contention::RetryAfter;
use bytes::Bytes;
pub use http::header::HeaderMap;
pub use http::StatusCode;
use smol_str::SmolStr;
use std::time::Duration;
use url::Url;

/// Métricas de temporização da requisição segundo W3C Navigation Timing Level 2.
#[derive(Debug, Clone, Default)]
pub struct ResponseTiming {
    pub total_duration: Duration,
    pub dns_duration: Option<Duration>,
    pub tcp_duration: Option<Duration>,
    pub tls_duration: Option<Duration>,
    pub ttfb: Duration,
}

/// Corpo de uma resposta HTTP, suportando armazenamento em memória contígua ou streaming.
#[derive(Debug, Clone)]
pub enum ResponseBody {
    /// Corpo completo mantido em buffer de bytes contíguo.
    Full(Bytes),
    /// Corpo vazio (ex: status 204 No Content ou 304 Not Modified).
    Empty,
}

impl ResponseBody {
    /// Retorna os bytes completos do corpo como uma fatia imutável.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Full(b) => b.as_ref(),
            Self::Empty => &[],
        }
    }

    /// Consome e retorna a estrutura `Bytes` subjacente.
    pub fn into_bytes(self) -> Bytes {
        match self {
            Self::Full(b) => b,
            Self::Empty => Bytes::new(),
        }
    }

    /// Retorna o tamanho total do corpo em bytes.
    pub fn len(&self) -> usize {
        match self {
            Self::Full(b) => b.len(),
            Self::Empty => 0,
        }
    }

    /// Verifica se o corpo é vazio.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Resposta HTTP entregue pelo `ResourceFetcher`.
#[derive(Debug, Clone)]
pub struct Response {
    /// URL final após resolução de todos os redirecionamentos.
    pub url: Url,
    /// Código de status HTTP da resposta (ex: 200 OK, 304 Not Modified, 404 Not Found).
    pub status: StatusCode,
    /// Cabeçalhos HTTP retornados pelo servidor ou injetados pelo cache.
    pub headers: HeaderMap,
    /// Payload em bytes da resposta.
    pub body: ResponseBody,
    /// MIME type resolvido (via cabeçalho Content-Type ou via Content Sniffing WHATWG).
    pub mime_type: SmolStr,
    /// Codificação de caracteres detectada (ex: "utf-8", "iso-8859-1").
    pub charset: Option<SmolStr>,
    /// Codificação de compressão de conteúdo (ex: gzip, br, deflate) que foi descomprimida.
    pub content_encoding: Option<ContentEncoding>,
    /// Indicação de espera estruturada caso o servidor tenha enviado `Retry-After`.
    pub retry_after: Option<RetryAfter>,
    /// Indica se a resposta foi servida diretamente do cache HTTP RFC 9111 (0 ms de rede).
    pub from_cache: bool,
    /// Métricas de latência e tempos de resposta.
    pub timing: ResponseTiming,
}

impl Response {
    /// Verifica se o status code representa sucesso (200..=299).
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Retorna os bytes do corpo.
    pub fn bytes(&self) -> &[u8] {
        self.body.as_bytes()
    }

    /// Tenta interpretar o corpo da resposta como texto UTF-8.
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.as_bytes().to_vec())
    }

    /// Tenta interpretar o corpo da resposta como texto, usando `String::from_utf8_lossy`.
    pub fn text_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.body.as_bytes())
    }

    /// Retorna a duração de espera sugerida caso exista cabeçalho `Retry-After`.
    pub fn retry_delay(&self, now: std::time::SystemTime) -> Option<Duration> {
        self.retry_after.map(|r| r.retry_delay(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_body_access() {
        let body = ResponseBody::Full(Bytes::from_static(b"<h1>Hello Albedo</h1>"));
        assert_eq!(body.len(), 21);
        assert!(!body.is_empty());
        assert_eq!(body.as_bytes(), b"<h1>Hello Albedo</h1>");

        let empty = ResponseBody::Empty;
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }
}
