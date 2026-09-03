//! # Testes de Contenção (Retry-After) e Serviços Alternativos (Alt-Svc)

use ace_net::contention::RetryAfter;
use ace_net::request::Request;
use ace_net::response::StatusCode;
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http::header::{RETRY_AFTER};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use std::time::{Duration, SystemTime};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_server_429_retry_after_and_alt_svc_capture() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let service = service_fn(|_req: HyperRequest<Incoming>| async move {
                        let resp = HyperResponse::builder()
                            .status(429)
                            .header(RETRY_AFTER, "120")
                            .header("alt-svc", "h3=\":443\"; ma=86400; persist=1")
                            .body(Full::new(Bytes::from_static(b"Too Many Requests")))
                            .unwrap();
                        Ok::<_, hyper::Error>(resp)
                    });

                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, service)
                        .await;
                });
            }
        }
    });

    let fetcher = ResourceFetcher::new().unwrap();
    let url = format!("http://{}/api/rate-limited", local_addr);

    let resp = fetcher.fetch(Request::get(&url).unwrap().build()).await.unwrap();

    assert_eq!(resp.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(resp.retry_after, Some(RetryAfter::Seconds(Duration::from_secs(120))));
    assert_eq!(resp.retry_delay(SystemTime::now()), Some(Duration::from_secs(120)));

    // Valida que o Alt-Svc foi capturado e indexado no registro
    let host = local_addr.ip().to_string();
    let alternatives = fetcher.alt_svc_registry().get_alternatives(None, &host, SystemTime::now());
    assert!(!alternatives.is_empty());
    assert_eq!(alternatives[0].protocol_id.as_str(), "h3");
    assert_eq!(alternatives[0].port, 443);
    assert!(alternatives[0].persist);
}
