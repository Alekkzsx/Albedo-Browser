//! # Testes de W3C Clear-Site-Data (RFC 8879)

use ace_net::clear_site_data::ClearSiteDataAction;
use ace_net::cookie::Cookie;
use ace_net::ResourceFetcher;
use http::header::HeaderValue;
use std::time::SystemTime;
use url::Url;

#[test]
fn test_clear_site_data_parsing_combinations() {
    let val_all = HeaderValue::from_static("\"*\"");
    let action_all = ClearSiteDataAction::parse(&val_all);
    assert!(action_all.clear_cache);
    assert!(action_all.clear_cookies);
    assert!(action_all.clear_storage);
    assert!(action_all.clear_execution_contexts);

    let val_cookies = HeaderValue::from_static("\"cookies\"");
    let action_cookies = ClearSiteDataAction::parse(&val_cookies);
    assert!(!action_cookies.clear_cache);
    assert!(action_cookies.clear_cookies);
    assert!(!action_cookies.clear_storage);
}

#[tokio::test]
async fn test_clear_site_data_action_execution() {
    let fetcher = ResourceFetcher::new().unwrap();
    let url = Url::parse("https://bank.example/account").unwrap();
    let now = SystemTime::now();

    // 1. Armazena um cookie prévio para bank.example
    let cookie = Cookie::parse("auth=secret_token; Secure", &url, None, now).unwrap();
    fetcher.cookie_jar().store_cookie(cookie);
    assert_eq!(fetcher.cookie_jar().len(), 1);

    // 2. Simula execução da purga de cookies e cache para esse domínio
    let host = url.host_str().unwrap();
    let val = HeaderValue::from_static("\"cookies\", \"cache\"");
    let action = ClearSiteDataAction::parse(&val);

    if action.clear_cookies {
        fetcher.cookie_jar().clear_for_domain(host);
    }
    if action.clear_cache {
        fetcher.cache().invalidate(&url);
    }

    assert_eq!(fetcher.cookie_jar().len(), 0);
}
