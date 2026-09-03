//! # Cliente de Transporte HTTP/1.1, HTTP/2 e TLS 1.3 (`TransportClient`)
//!
//! Orquestra o pool de conexões assíncronas, ALPN para negociação h2/http1.1,
//! handshake TLS seguro via `rustls` (WebPKI roots) e suporte nativo a `data:` URIs.

use crate::compression::{decompress_payload, ContentEncoding};
use crate::contention::RetryAfter;
use crate::encoding::extract_charset_from_content_type;
use crate::error::{NetError, NetResult};
use crate::request::Request;
use crate::response::{Response, ResponseBody, ResponseTiming};
use ace_core::net::{data_url::parse_data_url, sniff_mime_type};
use bytes::Bytes;
use http::HeaderValue;
use http_body_util::{BodyExt, Full};
use hyper::header::{CONTENT_TYPE, USER_AGENT};
use hyper::Request as HyperRequest;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use smol_str::SmolStr;
use std::time::{Duration, Instant};
use url::Url;

/// Cliente de transporte HTTP de baixo nível com pool de sockets seguro.
#[derive(Clone)]
pub struct TransportClient {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    user_agent: HeaderValue,
}

impl TransportClient {
    /// Cria uma nova instância de `TransportClient` com certificados WebPKI e ALPN habilitado.
    pub fn new() -> NetResult<Self> {
        let https = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();

        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(6)
            .build(https);

        let user_agent = HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Albedo/0.1.0 (ACE Engine)");

        Ok(Self { client, user_agent })
    }

    /// Executa o transporte físico de uma requisição HTTP ou resolução de URI local.
    pub async fn execute(&self, req: &Request) -> NetResult<Response> {
        let start_time = Instant::now();

        // 0. Verificação imediata de cancelamento
        if req.cancellation_token.is_cancelled() {
            return Err(NetError::Cancelled);
        }

        // 1. Suporte nativo e instantâneo a data: URIs (WHATWG Fetch §4.5)
        if req.url.scheme() == "data" {
            return self.execute_data_url(&req.url, start_time);
        }

        // 2. Validação de esquemas de rede suportados
        if req.url.scheme() != "http" && req.url.scheme() != "https" {
            return Err(NetError::UnsupportedScheme(req.url.scheme().to_string()));
        }

        // 3. Montagem da requisição Hyper
        let uri: hyper::Uri = req
            .url
            .as_str()
            .parse()
            .map_err(|e| NetError::InvalidUrl(format!("{}", e)))?;

        let mut hyper_builder = HyperRequest::builder()
            .method(req.method.clone())
            .uri(uri);

        // Copia cabeçalhos do Request
        if let Some(headers_mut) = hyper_builder.headers_mut() {
            *headers_mut = req.headers.clone();
            // Injeta User-Agent padrão se não especificado
            if !headers_mut.contains_key(USER_AGENT) {
                headers_mut.insert(USER_AGENT, self.user_agent.clone());
            }
        }

        let body_payload = req.body.clone().unwrap_or_default();
        let hyper_req = hyper_builder
            .body(Full::new(body_payload))
            .map_err(|e| NetError::HttpProtocolError(e.to_string()))?;

        // 4. Disparo com controle de timeout e cancelamento atômico
        let timeout_duration = req.timeout.unwrap_or(Duration::from_secs(30));
        let request_future = self.client.request(hyper_req);

        let hyper_resp = tokio::select! {
            _ = req.cancellation_token.cancelled() => {
                return Err(NetError::Cancelled);
            }
            res = tokio::time::timeout(timeout_duration, request_future) => {
                res.map_err(|_| NetError::Timeout)?
                    .map_err(|e| {
                        let err_msg = e.to_string();
                        if err_msg.contains("dns") || err_msg.contains("resolve") {
                            NetError::DnsResolutionFailed(req.url.host_str().unwrap_or("").into(), err_msg)
                        } else if err_msg.contains("tls") || err_msg.contains("certificate") {
                            NetError::TlsHandshakeFailed(req.url.host_str().unwrap_or("").into(), err_msg)
                        } else {
                            NetError::ConnectionFailed(req.url.host_str().unwrap_or("").into(), err_msg)
                        }
                    })?
            }
        };

        let ttfb = start_time.elapsed();
        let status = hyper_resp.status();
        let headers = hyper_resp.headers().clone();

        // 5. Coleta de bytes do corpo com suporte a cancelamento
        let raw_body_bytes = tokio::select! {
            _ = req.cancellation_token.cancelled() => {
                return Err(NetError::Cancelled);
            }
            res = hyper_resp.into_body().collect() => {
                res.map_err(|e| NetError::HttpProtocolError(e.to_string()))?.to_bytes()
            }
        };

        // 6. Descompressão transparente de conteúdo (Content-Encoding)
        let content_encoding = ContentEncoding::from_headers(&headers);
        let body_bytes = if let Some(encoding) = content_encoding {
            decompress_payload(encoding, &raw_body_bytes)?
        } else {
            raw_body_bytes
        };

        let retry_after = RetryAfter::from_headers(&headers);
        let total_duration = start_time.elapsed();

        // 7. Resolução de MIME Type e Charset (Content Sniffing WHATWG)
        let content_type_str = headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let mime_type: SmolStr = if !content_type_str.is_empty() {
            content_type_str
                .split(';')
                .next()
                .unwrap_or("application/octet-stream")
                .trim()
                .to_ascii_lowercase()
                .into()
        } else {
            // Sniffing nos primeiros 512 bytes
            sniff_mime_type(&body_bytes).into()
        };

        let charset = extract_charset_from_content_type(content_type_str);

        Ok(Response {
            url: req.url.clone(),
            status,
            headers,
            body: ResponseBody::Full(body_bytes),
            mime_type,
            charset,
            content_encoding,
            retry_after,
            from_cache: false,
            timing: ResponseTiming {
                total_duration,
                dns_duration: None,
                tcp_duration: None,
                tls_duration: None,
                ttfb,
            },
        })
    }

    /// Processamento de data: URLs em memória.
    fn execute_data_url(&self, url: &Url, start_time: Instant) -> NetResult<Response> {
        let record = parse_data_url(url.as_str())
            .map_err(|e| NetError::InvalidUrl(format!("Falha ao decodificar data URL: {}", e)))?;

        let body_bytes = Bytes::from(record.body);
        let mime_essence = record.mime_type.essence();
        let charset = record.mime_type.get_param("charset").map(SmolStr::from);

        let mut headers = http::HeaderMap::new();
        if let Ok(val) = http::HeaderValue::from_str(&mime_essence) {
            headers.insert(CONTENT_TYPE, val);
        }

        let elapsed = start_time.elapsed();

        Ok(Response {
            url: url.clone(),
            status: http::StatusCode::OK,
            headers,
            body: ResponseBody::Full(body_bytes),
            mime_type: mime_essence.into(),
            charset,
            content_encoding: None,
            retry_after: None,
            from_cache: false,
            timing: ResponseTiming {
                total_duration: elapsed,
                dns_duration: None,
                tcp_duration: None,
                tls_duration: None,
                ttfb: elapsed,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_url_execution() {
        let client = TransportClient::new().unwrap();
        let req = Request::get("data:text/html;charset=utf-8,<h1>Albedo</h1>").unwrap().build();

        let resp = client.execute(&req).await.unwrap();
        assert_eq!(resp.status, http::StatusCode::OK);
        assert_eq!(resp.mime_type.as_str(), "text/html");
        assert_eq!(resp.text().unwrap(), "<h1>Albedo</h1>");
    }

    #[tokio::test]
    async fn test_unsupported_scheme() {
        let client = TransportClient::new().unwrap();
        let req = Request::get("ftp://files.example.com/file.txt").unwrap().build();

        let result = client.execute(&req).await;
        assert!(matches!(result, Err(NetError::UnsupportedScheme(_))));
    }
}
