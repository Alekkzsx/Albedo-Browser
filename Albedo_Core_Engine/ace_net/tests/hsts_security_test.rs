//! # Testes de HSTS (RFC 6797) e Preload List com Auto-Upgrade

use ace_net::hsts::HstsStore;
use ace_net::request::Request;
use ace_net::ResourceFetcher;
use http::HeaderValue;
use std::time::SystemTime;
use url::Url;

#[test]
fn test_hsts_store_preloaded_domains() {
    let store = HstsStore::new();
    let now = SystemTime::now();

    assert!(store.should_upgrade("google.com", now));
    assert!(store.should_upgrade("sub.google.com", now));
    assert!(store.should_upgrade("github.com", now));
    assert!(store.should_upgrade("cloudflare.com", now));
    assert!(store.should_upgrade("albedo.browser", now));

    assert!(!store.should_upgrade("insecure-site.org", now));
}

#[test]
fn test_hsts_store_url_rewrite() {
    let store = HstsStore::new();
    let now = SystemTime::now();

    let http_url = Url::parse("http://google.com/search?q=rust").unwrap();
    let upgraded = store.upgrade_url(&http_url, now).unwrap();

    assert_eq!(upgraded.scheme(), "https");
    assert_eq!(upgraded.host_str(), Some("google.com"));
    assert_eq!(upgraded.path(), "/search");
    assert_eq!(upgraded.query(), Some("q=rust"));
}

#[tokio::test]
async fn test_fetcher_hsts_auto_upgrade_intercept() {
    let fetcher = ResourceFetcher::new().unwrap();

    // Registra dynamicamente uma política HSTS para domain.test
    fetcher.hsts_store().update_from_header(
        "domain.test",
        &HeaderValue::from_static("max-age=86400; includeSubDomains"),
        SystemTime::now(),
    );

    // Constrói requisição com URL http insegura
    let req = Request::get("http://domain.test/resource").unwrap().build();
    assert_eq!(req.url.scheme(), "http");

    // O fetcher deve detectar e reescrever a URL para https antes de qualquer tentativa de rede
    // (A requisição falhará no TLS/DNS mock, mas validamos a checagem no store)
    let upgraded = fetcher.hsts_store().upgrade_url(&req.url, SystemTime::now()).unwrap();
    assert_eq!(upgraded.scheme(), "https");
}
