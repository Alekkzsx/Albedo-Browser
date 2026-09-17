//! # Cliente de Transporte HTTP/1.1, HTTP/2 e TLS 1.3 (`TransportClient`)
//!
//! Orquestra o pool de conexões assíncronas, ALPN para negociação h2/http1.1,
//! handshake TLS seguro via `rustls` (WebPKI roots) e suporte nativo a `data:` URIs.

use crate::compression::{decompress_payload, ContentEncoding};
use crate::contention::RetryAfter;
use crate::encoding::extract_charset_from_content_type;
use crate::error::{NetError, NetResult};
use crate::range::ContentRange;
use crate::request::Request;
use crate::response::{Response, ResponseBody, ResponseTiming};
use ace_core::net::{data_url::parse_data_url, sniff_mime_type};
use bytes::Bytes;
use http::{HeaderValue, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::header::{CONTENT_TYPE, USER_AGENT};
use hyper::Request as HyperRequest;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use smol_str::SmolStr;
use std::pin::Pin;
use std::time::{Duration, Instant};
use crate::transport::dns::DohHappyEyeballsResolver;
use hyper::body::Body as HyperBody;
use url::Url;
use std::sync::Arc;
use tokio::sync::RwLock;
use rustc_hash::FxHashMap;
use hickory_resolver::proto::rr::rdata::svcb::{SvcParamKey, SvcParamValue};
use hickory_resolver::proto::rr::RecordType;
use rustls::client::{EchConfig, EchMode};

/// Stream assíncrono que encapsula o corpo bruto do Hyper para consumo com backpressure.
struct HyperIncomingStream {
    body: hyper::body::Incoming,
}

impl futures_core::Stream for HyperIncomingStream {
    type Item = NetResult<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.body).poll_frame(cx) {
                std::task::Poll::Ready(Some(Ok(frame))) => {
                    if let Ok(data) = frame.into_data() {
                        return std::task::Poll::Ready(Some(Ok(data)));
                    }
                    // Ignora trailers ou continua poll
                }
                std::task::Poll::Ready(Some(Err(e))) => {
                    return std::task::Poll::Ready(Some(Err(NetError::HttpProtocolError(e.to_string()))));
                }
                std::task::Poll::Ready(None) => return std::task::Poll::Ready(None),
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
    }
}

pub type TimingHttpsClient = Client<crate::transport::timing::TimingConnector<HttpsConnector<crate::transport::timing::TimingConnector<HttpConnector<DohHappyEyeballsResolver>>>>, Full<Bytes>>;

/// Cliente de transporte HTTP de baixo nível com pool de sockets seguro e DoH Happy Eyeballs v2.
#[derive(Clone)]
pub struct TransportClient {
    default_client: TimingHttpsClient,
    ech_clients: Arc<RwLock<FxHashMap<String, TimingHttpsClient>>>,
    resolver: DohHappyEyeballsResolver,
    root_store: rustls::RootCertStore,
    reqwest_h3_client: reqwest::Client,
    user_agent: HeaderValue,
}

impl TransportClient {
    /// Cria uma nova instância de `TransportClient` com certificados WebPKI, DoH Cloudflare e Happy Eyeballs v2.
    pub fn new() -> NetResult<Self> {
        Self::with_resolver(DohHappyEyeballsResolver::new())
    }

    /// Cria uma nova instância configurada com um resolver DoH customizado.
    pub fn with_resolver(resolver: DohHappyEyeballsResolver) -> NetResult<Self> {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let default_client = Self::build_hyper_client(&resolver, root_store.clone(), None)?;

        // Cliente reqwest para fallback HTTP/3 (QUIC)
        // Usamos reqwest experimental HTTP/3 para simplificar o contorno dos problemas do ecossistema quinn
        let reqwest_h3_client = reqwest::Client::builder()
            .http3_prior_knowledge()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Albedo/0.1.0 (ACE Engine H3)")
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback se H3 falhar na compilação do builder

        let user_agent = HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Albedo/0.1.0 (ACE Engine)");

        Ok(Self { 
            default_client, 
            ech_clients: Arc::new(RwLock::new(FxHashMap::default())),
            resolver,
            root_store,
            reqwest_h3_client, 
            user_agent 
        })
    }

    fn build_hyper_client(
        resolver: &DohHappyEyeballsResolver,
        root_store: rustls::RootCertStore,
        ech_config: Option<EchConfig>,
    ) -> NetResult<TimingHttpsClient> {
        let provider = std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let builder_versions = rustls::ClientConfig::builder_with_provider(provider.clone());

        let builder_verifier = if let Some(ech) = ech_config {
            builder_versions.with_ech(EchMode::Enable(ech)).map_err(|e| NetError::HttpProtocolError(format!("ECH config error: {}", e)))?
        } else {
            builder_versions.with_safe_default_protocol_versions().map_err(|e| NetError::HttpProtocolError(format!("TLS config error: {}", e)))?
        };

        let mut config = builder_verifier
            .with_root_certificates(root_store)
            .with_no_client_auth();
        // Milestone 5: Telemetria Preditiva & 0-RTT
        config.enable_early_data = true;

        // Conector HTTP/TCP configurado com DoH e Happy Eyeballs v2 (RFC 8305)
        let mut http_connector = HttpConnector::new_with_resolver(resolver.clone());
        http_connector.enforce_http(false);
        http_connector.set_happy_eyeballs_timeout(Some(Duration::from_millis(250)));

        let tcp_timing_connector = crate::transport::timing::TimingConnector::new(http_connector, false);

        let https = HttpsConnectorBuilder::new()
            .with_tls_config(config)
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(tcp_timing_connector);

        let tls_timing_connector = crate::transport::timing::TimingConnector::new(https, true);

        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(6)
            .build(tls_timing_connector);

        Ok(client)
    }

    /// Executa o transporte físico de uma requisição HTTP ou resolução de URI local.
    #[tracing::instrument(skip(self, req), fields(url = %req.url, method = %req.method))]
    pub async fn execute(&self, req: &Request) -> NetResult<Response> {
        let start_time = Instant::now();

        // 0. Verificação imediata de cancelamento
        if req.cancellation_token.is_cancelled() {
            return Err(NetError::Cancelled);
        }

        // 0.5. Roteamento Alt-Svc / HTTP/3 (Arquitetura M2) com Fallback Gracioso para TCP/TLS
        if req.force_h3 {
            crate::net_log::log_net_event(crate::net_log::NetEventType::Redirect, req.url.as_str(), "Tentando transporte QUIC HTTP/3 (Alt-Svc)");
            match self.execute_h3(req, start_time).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    crate::net_log::log_net_event(
                        crate::net_log::NetEventType::Warning,
                        req.url.as_str(),
                        &format!("Falha em conexao QUIC HTTP/3 ({}), realizando fallback gracioso para TCP/TLS", err),
                    );
                    tracing::warn!("Fallback de HTTP/3 para TCP/TLS em {}: {}", req.url, err);
                }
            }
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

        // NOVO: Busca do ECH
        let mut target_client = self.default_client.clone();
        
        if let Some(host) = req.url.host_str() {
            if req.url.scheme() == "https" {
                let cache_key = host.to_string();
                
                // Tenta ler do cache primeiro
                let cached = {
                    let map = self.ech_clients.read().await;
                    map.get(&cache_key).cloned()
                };

                if let Some(client) = cached {
                    target_client = client;
                } else {
                    // Consulta HTTPS para descobrir EchConfig
                    if let Ok(lookup) = self.resolver.resolver().lookup(host, RecordType::HTTPS).await {
                        let mut ech_config_bytes: Option<Vec<u8>> = None;
                        for record in lookup.iter() {
                            if let hickory_resolver::proto::rr::RData::HTTPS(svcb) = record {
                                for (key, val) in svcb.svc_params().iter() {
                                    if *key == SvcParamKey::EchConfig {
                                        // Acessar unknown octets
                                        match val {
                                            hickory_resolver::proto::rr::rdata::svcb::SvcParamValue::Unknown(data) => {
                                                ech_config_bytes = Some(data.0.clone());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        
                        if let Some(bytes) = ech_config_bytes {
                            // Construir novo client configurado para ECH
                            if let Ok(ech_config) = EchConfig::new(bytes.into(), rustls::crypto::aws_lc_rs::hpke::ALL_SUPPORTED_SUITES) {
                                if let Ok(new_client) = Self::build_hyper_client(&self.resolver, self.root_store.clone(), Some(ech_config)) {
                                    target_client = new_client.clone();
                                    let mut map = self.ech_clients.write().await;
                                    map.insert(cache_key, new_client);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Disparo com controle de timeout, cancelamento atômico e escopo de métricas
        let timeout_duration = req.timeout.unwrap_or(Duration::from_secs(30));
        
        let timing_state = std::sync::Arc::new(tokio::sync::Mutex::new(crate::transport::timing::ConnectionTiming::default()));
        let request_future = crate::transport::timing::CONNECTION_TIMING.scope(
            timing_state.clone(),
            target_client.request(hyper_req)
        );

        let hyper_resp = tokio::select! {
            _ = req.cancellation_token.cancelled() => {
                crate::net_log::log_net_event(crate::net_log::NetEventType::Cancel, req.url.as_str(), "Cancelled before connecting");
                return Err(NetError::Cancelled);
            }
            res = tokio::time::timeout(timeout_duration, request_future) => {
                res.map_err(|_| {
                    crate::net_log::log_net_event(crate::net_log::NetEventType::Error, req.url.as_str(), "Connection timeout");
                    NetError::Timeout
                })?
                .map_err(|e| {
                    let err_msg = e.to_string();
                    if err_msg.contains("dns") || err_msg.contains("resolve") {
                        crate::net_log::log_net_error(crate::net_log::NetEventType::Error, req.url.as_str(), &e);
                        NetError::DnsResolutionFailed(req.url.host_str().unwrap_or("").into(), err_msg)
                    } else if err_msg.contains("tls") || err_msg.contains("certificate") {
                        crate::net_log::log_net_error(crate::net_log::NetEventType::Error, req.url.as_str(), &e);
                        NetError::TlsHandshakeFailed(req.url.host_str().unwrap_or("").into(), err_msg)
                    } else {
                        crate::net_log::log_net_error(crate::net_log::NetEventType::Error, req.url.as_str(), &e);
                        NetError::ConnectionFailed(req.url.host_str().unwrap_or("").into(), err_msg)
                    }
                })?
            }
        };

        let ttfb = start_time.elapsed();
        let status = hyper_resp.status();
        let headers = hyper_resp.headers().clone();

        let content_encoding = ContentEncoding::from_headers(&headers);
        const MAX_BUFFERED_BODY_BYTES: usize = 64 * 1024 * 1024; // Teto de 64 MB

        let content_length = headers.get(hyper::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let use_stream = req.streaming || content_length.map_or(true, |len| len > 64 * 1024);

        // 5. Coleta ou streaming reativo de bytes do corpo com suporte a cancelamento
        let (body_bytes, response_body) = if use_stream {
            let stream = HyperIncomingStream { body: hyper_resp.into_body() };
            let raw_stream = ResponseBody::from_stream(stream);
            
            // 6. Descompressão transparente de conteúdo (Content-Encoding) via pipeline de stream
            let final_body = if let Some(encoding) = content_encoding {
                if let Some(stream_box) = raw_stream.take_stream().await {
                    let decompressed_stream = crate::compression::decompress_stream(encoding, stream_box);
                    ResponseBody::Stream(std::sync::Arc::new(tokio::sync::Mutex::new(Some(decompressed_stream))))
                } else {
                    raw_stream
                }
            } else {
                raw_stream
            };
            
            (Bytes::new(), final_body)
        } else {
            let incoming = hyper_resp.into_body();
            let limited = http_body_util::Limited::new(incoming, MAX_BUFFERED_BODY_BYTES);
            let raw_body_bytes = tokio::select! {
                _ = req.cancellation_token.cancelled() => {
                    return Err(NetError::Cancelled);
                }
                res = limited.collect() => {
                    res.map_err(|e| {
                        let err_msg = e.to_string();
                        if err_msg.contains("length limit") || err_msg.contains("limit exceeded") {
                            NetError::HttpProtocolError("Payload excedeu o teto de 64MB em memória; utilize o modo streaming".into())
                        } else {
                            NetError::HttpProtocolError(err_msg)
                        }
                    })?.to_bytes()
                }
            };

            // 6. Descompressão transparente de conteúdo (Content-Encoding)
            let decompressed = if let Some(encoding) = content_encoding {
                decompress_payload(encoding, &raw_body_bytes)?
            } else {
                raw_body_bytes
            };

            let full_bytes = decompressed.clone();
            (decompressed, ResponseBody::Full(full_bytes))
        };

        let retry_after = RetryAfter::from_headers(&headers);
        let content_range = if status == StatusCode::PARTIAL_CONTENT {
            headers.get(http::header::CONTENT_RANGE).and_then(ContentRange::parse)
        } else {
            None
        };
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
        } else if !body_bytes.is_empty() {
            // Sniffing nos primeiros 512 bytes
            sniff_mime_type(&body_bytes).into()
        } else {
            "application/octet-stream".into()
        };

        let charset = extract_charset_from_content_type(content_type_str);

        let (dns_duration, tcp_duration, tls_duration) = {
            let guard = timing_state.lock().await;
            (guard.dns_duration, guard.tcp_duration, guard.tls_duration)
        };

        Ok(Response {
            url: req.url.clone(),
            status,
            headers,
            body: response_body,
            mime_type,
            charset,
            content_encoding,
            retry_after,
            content_range,
            from_cache: false,
            timing: ResponseTiming {
                total_duration,
                dns_duration,
                tcp_duration,
                tls_duration,
                ttfb,
            },
        })
    }

    /// Processamento paralelo dedicado a HTTP/3 QUIC (Fase 6)
    async fn execute_h3(&self, req: &Request, start_time: Instant) -> NetResult<Response> {
        let reqwest_method = match req.method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "HEAD" => reqwest::Method::HEAD,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            _ => reqwest::Method::GET,
        };

        let mut h3_req = self.reqwest_h3_client.request(reqwest_method, req.url.clone());
        for (k, v) in &req.headers {
            h3_req = h3_req.header(k.as_str(), v.as_bytes());
        }
        if let Some(body) = &req.body {
            h3_req = h3_req.body(body.clone());
        }

        let timeout = req.timeout.unwrap_or(Duration::from_secs(30));
        let request_future = h3_req.send();

        let reqwest_resp = tokio::select! {
            _ = req.cancellation_token.cancelled() => {
                crate::net_log::log_net_event(crate::net_log::NetEventType::Cancel, req.url.as_str(), "H3 Cancelled");
                return Err(NetError::Cancelled);
            }
            res = tokio::time::timeout(timeout, request_future) => {
                res.map_err(|_| NetError::Timeout)?
                   .map_err(|e| NetError::ConnectionFailed(req.url.host_str().unwrap_or("").into(), e.to_string()))?
            }
        };

        let ttfb = start_time.elapsed();
        let status = StatusCode::from_u16(reqwest_resp.status().as_u16()).unwrap_or(StatusCode::OK);
        
        let mut headers = http::HeaderMap::new();
        for (k, v) in reqwest_resp.headers() {
            if let (Ok(name), Ok(value)) = (http::HeaderName::from_bytes(k.as_str().as_bytes()), http::HeaderValue::from_bytes(v.as_bytes())) {
                headers.insert(name, value);
            }
        }

        let raw_body_bytes = tokio::select! {
            _ = req.cancellation_token.cancelled() => return Err(NetError::Cancelled),
            res = reqwest_resp.bytes() => res.map_err(|e| NetError::HttpProtocolError(e.to_string()))?
        };

        let content_encoding = ContentEncoding::from_headers(&headers);
        let body_bytes = if let Some(encoding) = content_encoding {
            decompress_payload(encoding, &raw_body_bytes)?
        } else {
            raw_body_bytes
        };

        let retry_after = RetryAfter::from_headers(&headers);
        let content_range = if status == StatusCode::PARTIAL_CONTENT {
            headers.get(http::header::CONTENT_RANGE).and_then(ContentRange::parse)
        } else {
            None
        };
        let total_duration = start_time.elapsed();

        let content_type_str = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
        let mime_type: SmolStr = if !content_type_str.is_empty() {
            content_type_str.split(';').next().unwrap_or("application/octet-stream").trim().to_ascii_lowercase().into()
        } else {
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
            content_range,
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
            content_range: None,
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
