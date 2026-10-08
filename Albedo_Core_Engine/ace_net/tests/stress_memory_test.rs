use ace_net::transport::client::TransportClient;
use ace_net::http::request::Request;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::StatusCode;
use std::convert::Infallible;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_streaming_memory_stress() {
    // Servidor mock local loopback simulando resposta em stream
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let io = hyper_util::rt::TokioIo::new(stream);
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|_req| async {
                        let payload = Bytes::from(vec![b'A'; 64 * 1024]);
                        let mut resp = hyper::Response::new(Full::new(payload));
                        *resp.status_mut() = StatusCode::OK;
                        Ok::<_, Infallible>(resp)
                    }),
                )
                .await;
        }
    });

    let client = TransportClient::new().unwrap();
    let req = Request::get(format!("http://127.0.0.1:{}", addr.port())).unwrap().build();

    let resp_res = client.execute(&req).await;
    assert!(resp_res.is_ok(), "Requisição deve ser atendida com sucesso pelo mock local");
    let resp = resp_res.unwrap();
    assert!(resp.status.is_success());
    assert!(!resp.body.is_empty());
}
