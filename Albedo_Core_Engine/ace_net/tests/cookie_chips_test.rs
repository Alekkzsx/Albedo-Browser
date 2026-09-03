use ace_net::cookie::{Cookie, SameSite};
use ace_net::request::{CredentialsMode, Request};
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http::header::{COOKIE, SET_COOKIE};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::TcpListener;
use url::Url;

#[test]
fn test_cookie_chips_partition_isolation() {
    let now = SystemTime::now();
    let target_url = Url::parse("https://tracker.net/pixel.gif").unwrap();

    let cookie = Cookie::parse(
        "client_id=12345; Secure; SameSite=None; Partitioned",
        &target_url,
        Some("publisher-a.com"),
        now,
    )
    .unwrap();

    assert!(cookie.partition_key.is_some());
    assert_eq!(cookie.partition_key.as_deref(), Some("publisher-a.com"));
    assert_eq!(cookie.same_site, SameSite::None);

    // Validação com partition key coincidente
    assert!(cookie.is_valid_for_request(
        &target_url,
        Some("publisher-a.com"),
        CredentialsMode::Include,
        false,
        false,
        now,
    ));

    // Validação com partition key diferente (deve ser rejeitada)
    assert!(!cookie.is_valid_for_request(
        &target_url,
        Some("publisher-b.com"),
        CredentialsMode::Include,
        false,
        false,
        now,
    ));
}

#[tokio::test]
async fn test_fetcher_cookie_lifecycle_live() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let request_count = Arc::new(AtomicUsize::new(0));
    let req_counter = request_count.clone();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                let counter = req_counter.clone();

                tokio::spawn(async move {
                    let service = service_fn(move |req: HyperRequest<Incoming>| {
                        let count = counter.fetch_add(1, Ordering::SeqCst);
                        async move {
                            if count == 0 {
                                // Primeira requisição: envia Set-Cookie
                                let resp = HyperResponse::builder()
                                    .status(200)
                                    .header("cache-control", "no-store")
                                    .header(SET_COOKIE, "session_auth=secret_token_999; Path=/; Max-Age=3600")
                                    .body(Full::new(Bytes::from_static(b"Logged In")))
                                    .unwrap();
                                Ok::<_, hyper::Error>(resp)
                            } else {
                                // Segunda requisição: lê o cabeçalho Cookie enviado pelo cliente
                                let cookie_hdr = req.headers().get(COOKIE).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
                                let resp = HyperResponse::builder()
                                    .status(200)
                                    .body(Full::new(Bytes::from(cookie_hdr)))
                                    .unwrap();
                                Ok::<_, hyper::Error>(resp)
                            }
                        }
                    });

                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, service)
                        .await;
                });
            }
        }
    });

    let fetcher = ResourceFetcher::new().unwrap();
    let url = format!("http://{}/api/auth", local_addr);

    // 1. Primeira requisição recebe e armazena o cookie
    let req1 = Request::get(&url).unwrap().build();
    let resp1 = fetcher.fetch(req1).await.unwrap();
    assert_eq!(resp1.status, 200);
    assert_eq!(fetcher.cookie_jar().len(), 1);

    // 2. Segunda requisição deve enviar automaticamente o cookie armazenado
    let req2 = Request::get(&url).unwrap().build();
    let resp2 = fetcher.fetch(req2).await.unwrap();
    assert_eq!(resp2.text().unwrap(), "session_auth=secret_token_999");
}
