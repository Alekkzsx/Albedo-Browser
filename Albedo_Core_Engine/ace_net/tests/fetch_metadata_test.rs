//! # Testes de W3C Fetch Metadata Request Headers

use ace_core::security::origin::Origin;
use ace_net::fetch_metadata::SecFetchSite;
use ace_net::request::{Request, RequestDestination, RequestMode};
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use url::Url;

#[test]
fn test_fetch_metadata_site_logic() {
    let origin_a = Origin::parse("https://example.com").unwrap();
    let url_same_origin = Url::parse("https://example.com/feed").unwrap();
    let url_same_site = Url::parse("https://sub.example.com/assets/app.js").unwrap();
    let url_cross_site = Url::parse("https://another-domain.org/data").unwrap();

    assert_eq!(SecFetchSite::compute(Some(&origin_a), &url_same_origin), SecFetchSite::SameOrigin);
    assert_eq!(SecFetchSite::compute(Some(&origin_a), &url_same_site), SecFetchSite::SameSite);
    assert_eq!(SecFetchSite::compute(Some(&origin_a), &url_cross_site), SecFetchSite::CrossSite);
    assert_eq!(SecFetchSite::compute(None, &url_same_origin), SecFetchSite::None);
}

#[tokio::test]
async fn test_fetch_metadata_live_server_reception() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let service = service_fn(|req: HyperRequest<Incoming>| async move {
                        // Valida se os cabeçalhos Sec-Fetch-* foram injetados corretamente
                        let site = req.headers().get("sec-fetch-site").map(|v| v.to_str().unwrap().to_string());
                        let mode = req.headers().get("sec-fetch-mode").map(|v| v.to_str().unwrap().to_string());
                        let dest = req.headers().get("sec-fetch-dest").map(|v| v.to_str().unwrap().to_string());
                        let user = req.headers().get("sec-fetch-user").map(|v| v.to_str().unwrap().to_string());

                        let body_text = format!("{}:{}:{}:{}", site.unwrap_or_default(), mode.unwrap_or_default(), dest.unwrap_or_default(), user.unwrap_or_default());

                        let resp = HyperResponse::builder()
                            .status(200)
                            .body(Full::new(Bytes::from(body_text)))
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
    let url = format!("http://{}/page", local_addr);

    let req = Request::get(&url)
        .unwrap()
        .destination(RequestDestination::Document)
        .mode(RequestMode::Navigate)
        .user_activated(true)
        .build();

    let resp = fetcher.fetch(req).await.unwrap();
    let body = resp.text().unwrap();

    // Como não há initiator explícito, site é 'none', mode é 'navigate', dest é 'document' e user é '?1'
    assert_eq!(body, "none:navigate:document:?1");
}
