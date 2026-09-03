//! # Testes de Cancelamento Granular de Requisições (RequestId & CancellationToken)

use ace_net::error::NetError;
use ace_net::request::Request;
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_immediate_request_cancellation() {
    let fetcher = ResourceFetcher::new().unwrap();
    let req = Request::get("https://example.com/slow-endpoint").unwrap().build();

    // Sinaliza cancelamento antes de executar
    req.cancellation_token.cancel();

    let result = fetcher.fetch(req).await;
    assert!(matches!(result, Err(NetError::Cancelled)));
}

#[tokio::test]
async fn test_in_flight_cancellation_by_request_id() {
    // 1. Inicia um servidor HTTP mock com atraso deliberado de 2 segundos
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let service = service_fn(|_req: HyperRequest<Incoming>| async move {
                        // Simula servidor muito lento
                        tokio::time::sleep(Duration::from_millis(1500)).await;
                        let resp = HyperResponse::builder()
                            .status(200)
                            .body(Full::new(Bytes::from_static(b"Too slow")))
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
    let url = format!("http://{}/slow", local_addr);
    let req = Request::get(&url).unwrap().build();
    let request_id = req.id;

    let fetcher_clone = fetcher.clone();

    // Spawna o cancelamento para ocorrer após 100ms (enquanto a requisição está em voo)
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        fetcher_clone.cancel(request_id);
    });

    // O fetch deve ser abortado rapidamente com NetError::Cancelled
    let start = std::time::Instant::now();
    let result = fetcher.fetch(req).await;
    let elapsed = start.elapsed();

    assert!(matches!(result, Err(NetError::Cancelled)));
    // O cancelamento deve abortar em bem menos tempo que os 1500ms do servidor
    assert!(elapsed < Duration::from_millis(1000));
}
