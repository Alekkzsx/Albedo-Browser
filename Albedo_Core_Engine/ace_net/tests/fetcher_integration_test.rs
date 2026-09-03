//! # Testes de Integração do ResourceFetcher
//!
//! Executa testes ponta a ponta contra data URLs e um servidor mock HTTP local in-process.

use ace_net::priority::PriorityLevel;
use ace_net::request::{Request, RequestDestination};
use ace_net::response::StatusCode;
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, LOCATION};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_fetch_data_url_svg() {
    let fetcher = ResourceFetcher::new().unwrap();
    let data_url = "data:image/svg+xml;utf8,<svg><circle r='10'/></svg>";
    let resp = fetcher.fetch_url(data_url, PriorityLevel::Medium).await.unwrap();

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.mime_type.as_str(), "image/svg+xml");
    assert_eq!(resp.text().unwrap(), "<svg><circle r='10'/></svg>");
}

#[tokio::test]
async fn test_fetch_data_url_base64() {
    let fetcher = ResourceFetcher::new().unwrap();
    // "Albedo Browser" em base64 é "QWxiZWRvIEJyb3dzZXI="
    let data_url = "data:text/plain;base64,QWxiZWRvIEJyb3dzZXI=";
    let resp = fetcher.fetch_url(data_url, PriorityLevel::Low).await.unwrap();

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.text().unwrap(), "Albedo Browser");
}

#[tokio::test]
async fn test_fetcher_local_mock_server_caching_and_redirects() {
    // 1. Inicializa servidor HTTP mock local
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
                        let path = req.uri().path().to_string();
                        counter.fetch_add(1, Ordering::SeqCst);

                        async move {
                            if path == "/doc" {
                                let resp = HyperResponse::builder()
                                    .status(200)
                                    .header(CONTENT_TYPE, "text/html; charset=utf-8")
                                    .header(CACHE_CONTROL, "public, max-age=3600")
                                    .header(ETAG, "\"v1.0\"")
                                    .body(Full::new(Bytes::from_static(b"<!doctype html><h1>Albedo Online</h1>")))
                                    .unwrap();
                                Ok::<_, hyper::Error>(resp)
                            } else if path == "/redirect" {
                                let resp = HyperResponse::builder()
                                    .status(302)
                                    .header(LOCATION, "/doc")
                                    .body(Full::new(Bytes::new()))
                                    .unwrap();
                                Ok::<_, hyper::Error>(resp)
                            } else {
                                let resp = HyperResponse::builder()
                                    .status(404)
                                    .body(Full::new(Bytes::from_static(b"Not Found")))
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
    let base_url = format!("http://{}", local_addr);

    // 2. Primeira requisição para /doc (Cache Miss -> Busca na rede)
    let doc_url = format!("{}/doc", base_url);
    let resp1 = fetcher
        .fetch(Request::get(&doc_url).unwrap().destination(RequestDestination::Document).build())
        .await
        .unwrap();

    assert_eq!(resp1.status, StatusCode::OK);
    assert!(!resp1.from_cache);
    assert_eq!(resp1.text().unwrap(), "<!doctype html><h1>Albedo Online</h1>");
    assert_eq!(request_count.load(Ordering::SeqCst), 1);

    // 3. Segunda requisição para a mesma URL (Cache Hit RFC 9111 -> 0 latência, 0 requests na rede)
    let resp2 = fetcher
        .fetch(Request::get(&doc_url).unwrap().destination(RequestDestination::Document).build())
        .await
        .unwrap();

    assert_eq!(resp2.status, StatusCode::OK);
    assert!(resp2.from_cache);
    assert_eq!(resp2.text().unwrap(), "<!doctype html><h1>Albedo Online</h1>");
    // O contador de requisições no servidor ainda DEVE ser 1!
    assert_eq!(request_count.load(Ordering::SeqCst), 1);

    // 4. Requisição com redirecionamento /redirect -> /doc
    let redir_url = format!("{}/redirect", base_url);
    let resp3 = fetcher
        .fetch(Request::get(&redir_url).unwrap().build())
        .await
        .unwrap();

    assert_eq!(resp3.status, StatusCode::OK);
    // Como redirecionou para /doc e /doc já estava em cache, o destino final veio do cache!
    assert_eq!(resp3.text().unwrap(), "<!doctype html><h1>Albedo Online</h1>");
}
