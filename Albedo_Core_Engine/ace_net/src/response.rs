//! # Modelagem de Respostas de Rede (`Response` & `ResponseBody`)
//!
//! Representa os metadados HTTP, cabeçalhos, métricas de performance (Navigation Timing)
//! e stream assíncrono do corpo de bytes recebido.

use crate::compression::ContentEncoding;
use crate::contention::RetryAfter;
use crate::range::ContentRange;
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

use crate::error::{NetError, NetResult};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Stream assíncrono de chunks de bytes para consumo em fluxo contínuo com contrapressão (backpressure).
pub type BoxByteStream = Pin<Box<dyn futures_core::Stream<Item = NetResult<Bytes>> + Send>>;

/// Stream de chunk único para conversão transparente de buffers contíguos em fluxo reativo.
struct SingleChunkStream(Option<Bytes>);

impl futures_core::Stream for SingleChunkStream {
    type Item = NetResult<Bytes>;
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        std::task::Poll::Ready(self.0.take().map(Ok))
    }
}

/// Corpo de uma resposta HTTP, suportando buffer contíguo em RAM, vazio ou streaming reativo (WHATWG Fetch ReadableStream).
#[derive(Clone)]
pub enum ResponseBody {
    /// Corpo completo mantido em buffer de bytes contíguo.
    Full(Bytes),
    /// Corpo vazio (ex: status 204 No Content ou 304 Not Modified).
    Empty,
    /// Stream reativo desacoplado com contrapressão e consumo sob demanda.
    Stream(Arc<Mutex<Option<BoxByteStream>>>),
}

impl std::fmt::Debug for ResponseBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full(b) => write!(f, "ResponseBody::Full({} bytes)", b.len()),
            Self::Empty => write!(f, "ResponseBody::Empty"),
            Self::Stream(_) => write!(f, "ResponseBody::Stream(<reactive>)"),
        }
    }
}

impl ResponseBody {
    /// Cria um novo corpo a partir de um stream assíncrono.
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: futures_core::Stream<Item = NetResult<Bytes>> + Send + 'static,
    {
        Self::Stream(Arc::new(Mutex::new(Some(Box::pin(stream)))))
    }

    /// Retorna os bytes completos do corpo como uma fatia imutável (apenas se estiver em memória).
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Full(b) => b.as_ref(),
            Self::Empty | Self::Stream(_) => &[],
        }
    }

    /// Consome e retorna a estrutura `Bytes` síncrona (se em memória) ou vazia.
    pub fn into_bytes(self) -> Bytes {
        match self {
            Self::Full(b) => b,
            Self::Empty | Self::Stream(_) => Bytes::new(),
        }
    }

    /// Coleta todos os bytes do corpo de forma assíncrona, drenando o stream se necessário.
    pub async fn collect_bytes(self) -> NetResult<Bytes> {
        match self {
            Self::Full(b) => Ok(b),
            Self::Empty => Ok(Bytes::new()),
            Self::Stream(stream_mutex) => {
                let mut guard = stream_mutex.lock().await;
                if let Some(mut stream) = guard.take() {
                    let mut buf = bytes::BytesMut::new();
                    while let Some(chunk_res) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
                        let chunk = chunk_res?;
                        buf.extend_from_slice(&chunk);
                    }
                    Ok(buf.freeze())
                } else {
                    Err(NetError::HttpProtocolError("Corpo de streaming já foi consumido (disturbed body)".into()))
                }
            }
        }
    }

    /// Extrai o stream reativo para consumo progressivo em pipeline.
    pub async fn take_stream(&self) -> Option<BoxByteStream> {
        match self {
            Self::Full(b) => Some(Box::pin(SingleChunkStream(Some(b.clone())))),
            Self::Empty => None,
            Self::Stream(m) => {
                let mut guard = m.lock().await;
                guard.take()
            }
        }
    }

    /// Retorna o tamanho total do corpo em bytes se conhecido antecipadamente.
    pub fn len(&self) -> usize {
        match self {
            Self::Full(b) => b.len(),
            Self::Empty | Self::Stream(_) => 0,
        }
    }

    /// Verifica se o corpo é vazio.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Full(b) => b.is_empty(),
            Self::Empty => true,
            Self::Stream(_) => false,
        }
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
    /// Faixa de bytes recebida caso seja resposta parcial (206 Partial Content).
    pub content_range: Option<ContentRange>,
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

    /// Verifica se o status code é 206 Partial Content.
    pub fn is_partial(&self) -> bool {
        self.status == StatusCode::PARTIAL_CONTENT
    }

    /// Retorna os bytes do corpo.
    pub fn bytes(&self) -> &[u8] {
        self.body.as_bytes()
    }

    /// Coleta todos os bytes da resposta de forma assíncrona, consumindo o stream se aplicável.
    pub async fn collect_bytes(self) -> NetResult<Bytes> {
        self.body.collect_bytes().await
    }

    /// Interpreta o corpo da resposta como texto UTF-8 de forma assíncrona.
    pub async fn text_async(self) -> NetResult<String> {
        let b = self.collect_bytes().await?;
        String::from_utf8(b.to_vec()).map_err(|e| NetError::HttpProtocolError(e.to_string()))
    }

    /// Tenta interpretar o corpo da resposta como texto UTF-8 se já carregado em memória.
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

    #[tokio::test]
    async fn test_response_body_streaming_consumption() {
        // Simula um stream de chunks de rede: chunk1 + chunk2
        struct TwoChunkStream {
            chunk1: Option<Bytes>,
            chunk2: Option<Bytes>,
        }

        impl futures_core::Stream for TwoChunkStream {
            type Item = NetResult<Bytes>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Self::Item>> {
                if let Some(c1) = self.chunk1.take() {
                    std::task::Poll::Ready(Some(Ok(c1)))
                } else if let Some(c2) = self.chunk2.take() {
                    std::task::Poll::Ready(Some(Ok(c2)))
                } else {
                    std::task::Poll::Ready(None)
                }
            }
        }

        let stream = TwoChunkStream {
            chunk1: Some(Bytes::from_static(b"Hello ")),
            chunk2: Some(Bytes::from_static(b"Streaming World!")),
        };

        let body = ResponseBody::from_stream(stream);
        assert!(!body.is_empty());

        let collected = body.collect_bytes().await.unwrap();
        assert_eq!(collected.as_ref(), b"Hello Streaming World!");
    }
}

