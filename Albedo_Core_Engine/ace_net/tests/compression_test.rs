//! # Testes de Descompressão Transparente de Conteúdo HTTP (RFC 9110, RFC 7932)

use ace_net::compression::{compress_brotli, compress_gzip, decompress_payload, ContentEncoding};
use ace_net::request::Request;
use ace_net::response::StatusCode;
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

#[test]
fn test_roundtrip_gzip_payload() {
    let payload = b"<!DOCTYPE html><html><body><h1>Albedo Gzip Test</h1><p>Super fast rendering.</p></body></html>";
    let compressed = compress_gzip(payload);
    let raw = Bytes::from(compressed);

    let decompressed = decompress_payload(ContentEncoding::Gzip, &raw).unwrap();
    assert_eq!(decompressed.as_ref(), payload);
}

#[test]
fn test_roundtrip_brotli_payload() {
    let payload = b"{\"browser\":\"Albedo\",\"version\":\"0.1.0\",\"status\":\"extreme_performance\"}";
    let compressed = compress_brotli(payload);
    let raw = Bytes::from(compressed);

    let decompressed = decompress_payload(ContentEncoding::Brotli, &raw).unwrap();
    assert_eq!(decompressed.as_ref(), payload);
}

#[test]
fn test_identity_encoding_pass_through() {
    let payload = Bytes::from_static(b"raw-uncompressed-bytes");
    let result = decompress_payload(ContentEncoding::Identity, &payload).unwrap();
    assert_eq!(result, payload);
}

#[tokio::test]
async fn test_fetcher_transparent_gzip_decompression_live() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let raw_html = b"<!doctype html><h1>Transparent Gzip Decompression Succeeded</h1>";
    let compressed_html = compress_gzip(raw_html);

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                let body = compressed_html.clone();

                tokio::spawn(async move {
                    let service = service_fn(move |_req: HyperRequest<Incoming>| {
                        let payload = body.clone();
                        async move {
                            let resp = HyperResponse::builder()
                                .status(200)
                                .header(CONTENT_TYPE, "text/html; charset=utf-8")
                                .header(CONTENT_ENCODING, "gzip")
                                .body(Full::new(Bytes::from(payload)))
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
    let url = format!("http://{}/index.html", local_addr);

    let resp = fetcher.fetch(Request::get(&url).unwrap().build()).await.unwrap();

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.content_encoding, Some(ContentEncoding::Gzip));
    assert_eq!(resp.text().unwrap(), "<!doctype html><h1>Transparent Gzip Decompression Succeeded</h1>");
}
