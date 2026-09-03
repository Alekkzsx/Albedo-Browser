//! # Testes de Range Requests e HTTP 206 Partial Content (RFC 9110 §14)

use ace_net::range::ByteRangeSpec;
use ace_net::request::Request;
use ace_net::response::StatusCode;
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http::header::{CONTENT_RANGE, RANGE};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

#[test]
fn test_range_spec_variants() {
    assert_eq!(ByteRangeSpec::From(2048).to_header_value().to_str().unwrap(), "bytes=2048-");
    assert_eq!(ByteRangeSpec::Range(100, 200).to_header_value().to_str().unwrap(), "bytes=100-200");
    assert_eq!(ByteRangeSpec::Suffix(1024).to_header_value().to_str().unwrap(), "bytes=-1024");
}

#[tokio::test]
async fn test_range_request_live_server_206() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let full_content = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 36 bytes

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let service = service_fn(|req: HyperRequest<Incoming>| async move {
                        let range_hdr = req.headers().get(RANGE).map(|v| v.to_str().unwrap());

                        if let Some("bytes=0-9") = range_hdr {
                            let slice = &full_content[0..10];
                            let resp = HyperResponse::builder()
                                .status(206)
                                .header(CONTENT_RANGE, "bytes 0-9/36")
                                .body(Full::new(Bytes::from_static(slice)))
                                .unwrap();
                            Ok::<_, hyper::Error>(resp)
                        } else {
                            let resp = HyperResponse::builder()
                                .status(200)
                                .body(Full::new(Bytes::from_static(full_content)))
                                .unwrap();
                            Ok::<_, hyper::Error>(resp)
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
    let url = format!("http://{}/video.mp4", local_addr);

    let req = Request::get(&url)
        .unwrap()
        .range(ByteRangeSpec::Range(0, 9))
        .build();

    let resp = fetcher.fetch(req).await.unwrap();

    assert_eq!(resp.status, StatusCode::PARTIAL_CONTENT);
    assert!(resp.is_partial());

    let cr = resp.content_range.expect("Deve conter ContentRange parseado");
    assert_eq!(cr.start, 0);
    assert_eq!(cr.end, 9);
    assert_eq!(cr.total, Some(36));
    assert_eq!(cr.range_len(), 10);
    assert_eq!(resp.bytes(), b"0123456789");
}
