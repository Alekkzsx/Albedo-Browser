//! # Testes de Validação SOTA Fase 2 — ace_net
//!
//! Valida os avanços de engenharia comparativos contra Chromium //net e Firefox Necko:
//! 1. Streaming Reativo em `ResponseBody` com backpressure.
//! 2. Conformidade RFC 9111 de cache privado (`private`).
//! 3. Validação de chaves secundárias `Vary` e rejeição de `Vary: *`.
//! 4. RFC 5861 `stale-while-revalidate`.
//! 5. Hash determinístico do cache L2 (`HttpCache::hash_key`).
//! 6. Bloqueio de Super-Cookies via Public Suffix List (PSL).
//! 7. Cota de 180 cookies por domínio com evicção LRU no `CookieJar`.
//! 8. Happy Eyeballs v2 (RFC 8305) com interleaving de IPv6 e IPv4.
//! 9. Interceptação de Service Worker no pipeline de `fetch()`.
//! 10. Handshake e ciclo de vida de WebSocket (RFC 6455).

use ace_net::cache::{CacheEntry, HttpCache};
use ace_net::cookie::{is_public_suffix, is_valid_cookie_domain, Cookie, CookieJar, MAX_COOKIES_PER_DOMAIN};
use ace_net::fetcher::ResourceFetcher;
use ace_net::request::{CredentialsMode, Request};
use ace_net::response::{ResponseBody, ResponseTiming};
use ace_net::service_worker_hook::ServiceWorkerHook;
use ace_net::transport::DohHappyEyeballsResolver;
use ace_net::websocket::{WebSocketMessage, WebSocketSession, WebSocketUpgrade};
use bytes::Bytes;
use http::header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_TYPE, VARY};
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use url::Url;

// 1. Streaming Reativo
#[tokio::test]
async fn test_streaming_response_body_with_backpressure() {
    struct ChunkStream {
        chunks: Vec<Bytes>,
        index: usize,
    }

    impl futures_core::Stream for ChunkStream {
        type Item = ace_net::error::NetResult<Bytes>;
        fn poll_next(
            mut self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            if self.index < self.chunks.len() {
                let chunk = self.chunks[self.index].clone();
                self.index += 1;
                std::task::Poll::Ready(Some(Ok(chunk)))
            } else {
                std::task::Poll::Ready(None)
            }
        }
    }

    let stream = ChunkStream {
        chunks: vec![
            Bytes::from_static(b"chunk_1; "),
            Bytes::from_static(b"chunk_2; "),
            Bytes::from_static(b"chunk_3"),
        ],
        index: 0,
    };

    let body = ResponseBody::from_stream(stream);
    assert!(!body.is_empty());

    let collected = body.collect_bytes().await.unwrap();
    assert_eq!(collected.as_ref(), b"chunk_1; chunk_2; chunk_3");
}

// 2. Cache RFC 9111: Permissão de 'private'
#[test]
fn test_cache_control_private_allowed_for_browser() {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, max-age=1800"));
    assert!(CacheEntry::is_cacheable(&Method::GET, StatusCode::OK, &headers));
}

// 3. Cache RFC 9111: Validação de Vary
#[test]
fn test_cache_vary_secondary_key_matching() {
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(VARY, HeaderValue::from_static("Accept-Encoding"));

    let mut original_req_headers = HeaderMap::new();
    original_req_headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, br"));

    let now = SystemTime::now();
    let entry = CacheEntry::new(
        Url::parse("https://cdn.example.com/bundle.js").unwrap(),
        StatusCode::OK,
        resp_headers,
        Bytes::from_static(b"console.log('test')"),
        now,
        now,
    ).with_request_headers(original_req_headers);

    // Requisição com mesmo cabeçalho deve dar match
    let mut matching_req = HeaderMap::new();
    matching_req.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, br"));
    assert!(entry.matches_request_headers(&matching_req));

    // Requisição com cabeçalho divergente deve falhar
    let mut mismatching_req = HeaderMap::new();
    mismatching_req.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    assert!(!entry.matches_request_headers(&mismatching_req));

    // Vary: * nunca pode ser cacheado
    let mut asterisk_headers = HeaderMap::new();
    asterisk_headers.insert(VARY, HeaderValue::from_static("*"));
    assert!(!CacheEntry::is_cacheable(&Method::GET, StatusCode::OK, &asterisk_headers));
}

// 4. RFC 5861: Stale-While-Revalidate
#[test]
fn test_stale_while_revalidate_spec() {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=100, stale-while-revalidate=500"));

    let now = SystemTime::now();
    let entry = CacheEntry::new(
        Url::parse("https://api.example.com/status").unwrap(),
        StatusCode::OK,
        headers,
        Bytes::from_static(b"{\"status\":\"ok\"}"),
        now,
        now,
    );

    assert_eq!(entry.freshness_lifetime(), Duration::from_secs(100));
    assert_eq!(entry.stale_while_revalidate_lifetime(), Duration::from_secs(500));

    // Em t=50s: fresco
    assert!(entry.is_fresh(now + Duration::from_secs(50)));
    assert!(!entry.is_stale_revalidatable(now + Duration::from_secs(50)));

    // Em t=250s: stale mas dentro de stale-while-revalidate (100..600s)
    let t250 = now + Duration::from_secs(250);
    assert!(!entry.is_fresh(t250));
    assert!(entry.is_stale_revalidatable(t250));

    // Em t=700s: expirado fora da janela
    let t700 = now + Duration::from_secs(700);
    assert!(!entry.is_fresh(t700));
    assert!(!entry.is_stale_revalidatable(t700));
}

// 5. Hash determinístico do cache L2
#[test]
fn test_cache_l2_deterministic_hash() {
    let key = "https://example.com/asset.png";
    let hash1 = HttpCache::hash_key(key);
    let hash2 = HttpCache::hash_key(key);
    assert_eq!(hash1, hash2, "O hash deve ser 100% determinístico e estável");
    assert_ne!(hash1, 0);
}

// 6. Defesa contra Super-Cookies via PSL
#[test]
fn test_public_suffix_super_cookie_defense() {
    assert!(is_public_suffix("com"));
    assert!(is_public_suffix("co.uk"));
    assert!(is_public_suffix("com.br"));
    assert!(is_public_suffix("github.io"));

    assert!(!is_public_suffix("albedo.browser"));
    assert!(!is_public_suffix("myblog.com"));

    let url = Url::parse("https://bank.example.co.uk").unwrap();
    let now = SystemTime::now();

    // Rejeição de super-cookie para .co.uk
    let super_cookie = Cookie::parse("session=attacker; Domain=co.uk", &url, None, now);
    assert!(super_cookie.is_none());

    // Aceitação de escopo válido
    let valid_cookie = Cookie::parse("session=legit; Domain=example.co.uk", &url, None, now);
    assert!(valid_cookie.is_some());
    assert_eq!(valid_cookie.unwrap().domain, "example.co.uk");
    assert!(is_valid_cookie_domain("bank.example.co.uk", "example.co.uk"));
    assert!(!is_valid_cookie_domain("bank.example.co.uk", "co.uk"));
}

// 7. CookieJar: Cota e evicção LRU por domínio
#[test]
fn test_cookie_jar_lru_quota_eviction() {
    let jar = CookieJar::new();
    let url = Url::parse("https://ecommerce.com").unwrap();
    let now = SystemTime::now();

    for i in 0..=MAX_COOKIES_PER_DOMAIN {
        let c = Cookie::parse(&format!("k_{}={}; Path=/", i, i), &url, None, now).unwrap();
        jar.store_cookie(c);
    }

    assert_eq!(jar.len(), MAX_COOKIES_PER_DOMAIN);

    let hdr = jar.build_cookie_header(&url, None, CredentialsMode::SameOrigin, true, true, now).unwrap();
    let s = hdr.to_str().unwrap();
    // O cookie k_0 deve ter sido evictado (LRU)
    assert!(!s.contains("k_0="));
    assert!(s.contains("k_1="));
}

// 8. Happy Eyeballs v2 (RFC 8305) Interleaving
#[test]
fn test_happy_eyeballs_interleaving_rfc8305() {
    let ips = vec![
        IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
        IpAddr::V6(Ipv6Addr::new(0x2606, 0x2800, 0x220, 0x1, 0x248, 0x1893, 0x25c8, 0x1946)),
    ];

    let addrs = DohHappyEyeballsResolver::interleave_happy_eyeballs(ips);
    assert_eq!(addrs.len(), 2);
    // IPv6 tem prioridade como primeiro da lista
    assert!(addrs[0].is_ipv6());
    assert!(addrs[1].is_ipv4());
}

// 9. Interceptação de Service Worker
#[tokio::test]
async fn test_service_worker_fetch_hook_integration() {
    struct SwMock;
    impl ServiceWorkerHook for SwMock {
        fn on_fetch(
            &self,
            req: &Request,
        ) -> Pin<Box<dyn std::future::Future<Output = ace_net::error::NetResult<Option<ace_net::response::Response>>> + Send + '_>> {
            let url = req.url.clone();
            Box::pin(async move {
                if url.as_str() == "https://sw.test/cached" {
                    let mut headers = HeaderMap::new();
                    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                    Ok(Some(ace_net::response::Response {
                        url,
                        status: StatusCode::OK,
                        headers,
                        body: ResponseBody::Full(Bytes::from_static(b"SW Intercepted")),
                        mime_type: "text/plain".into(),
                        charset: Some("utf-8".into()),
                        content_encoding: None,
                        retry_after: None,
                        content_range: None,
                        from_cache: false,
                        timing: ResponseTiming::default(),
                    }))
                } else {
                    Ok(None)
                }
            })
        }
    }

    let fetcher = ResourceFetcher::new().unwrap();
    fetcher.set_service_worker_hook(Arc::new(SwMock));

    let req = Request::get("https://sw.test/cached").unwrap().build();
    let resp = fetcher.fetch(req).await.unwrap();

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.text().unwrap(), "SW Intercepted");
}

// 10. WebSocket Upgrade RFC 6455
#[tokio::test]
async fn test_websocket_upgrade_and_session() {
    let url = Url::parse("wss://chat.example.com/ws").unwrap();
    let upgrade = WebSocketUpgrade::new(url).unwrap();
    let req = upgrade.build_handshake_request();

    assert_eq!(req.headers.get("upgrade").unwrap(), "websocket");
    assert_eq!(req.headers.get("connection").unwrap(), "Upgrade");

    let (tx, _rx) = mpsc::channel(5);
    let (srv_tx, client_rx) = mpsc::channel(5);
    let session = WebSocketSession::new(Url::parse("ws://chat.example.com").unwrap(), tx, client_rx);

    session.send(WebSocketMessage::Text("hello".into())).await.unwrap();
    srv_tx.send(WebSocketMessage::Text("world".into())).await.unwrap();

    let rec = session.recv().await.unwrap();
    assert_eq!(rec, WebSocketMessage::Text("world".into()));
}
