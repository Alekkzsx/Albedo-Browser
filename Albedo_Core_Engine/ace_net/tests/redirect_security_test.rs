//! # Testes de Segurança e Resolução de Redirecionamentos (3xx)

use ace_net::error::NetError;
use ace_net::redirect::{handle_redirect, RedirectAction};
use ace_net::request::Request;
use bytes::Bytes;
use http::header::{AUTHORIZATION, COOKIE, LOCATION};
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use std::collections::HashSet;

#[test]
fn test_redirect_301_302_post_to_get_conversion() {
    let req = Request::post("https://example.com/submit")
        .unwrap()
        .body(Bytes::from_static(b"field=value"))
        .build();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(LOCATION, HeaderValue::from_static("/result"));

    let mut visited = HashSet::new();
    let action = handle_redirect(
        &req,
        StatusCode::MOVED_PERMANENTLY,
        &resp_headers,
        &mut visited,
        20,
    )
    .unwrap();

    match action {
        RedirectAction::Follow(follow) => {
            assert_eq!(follow.new_url.as_str(), "https://example.com/result");
            assert_eq!(follow.new_method, Method::GET);
            assert!(follow.new_body.is_none());
        }
        _ => panic!("Esperado Follow redirect"),
    }
}

#[test]
fn test_redirect_307_308_preserves_post_body() {
    let req = Request::post("https://example.com/api/v1/update")
        .unwrap()
        .body(Bytes::from_static(b"{\"status\":\"active\"}"))
        .build();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(LOCATION, HeaderValue::from_static("https://example.com/api/v2/update"));

    let mut visited = HashSet::new();
    let action = handle_redirect(
        &req,
        StatusCode::PERMANENT_REDIRECT,
        &resp_headers,
        &mut visited,
        20,
    )
    .unwrap();

    match action {
        RedirectAction::Follow(follow) => {
            assert_eq!(follow.new_url.as_str(), "https://example.com/api/v2/update");
            assert_eq!(follow.new_method, Method::POST);
            assert_eq!(follow.new_body.unwrap().as_ref(), b"{\"status\":\"active\"}");
        }
        _ => panic!("Esperado Follow redirect"),
    }
}

#[test]
fn test_cross_origin_redirect_stripping() {
    let mut req_builder = Request::get("https://private.corp.com/internal").unwrap();
    req_builder = req_builder.header(AUTHORIZATION, HeaderValue::from_static("Bearer token_secret"));
    req_builder = req_builder.header(COOKIE, HeaderValue::from_static("session_id=12345"));
    req_builder = req_builder.header(HeaderName::from_static("x-custom-header"), HeaderValue::from_static("allowed"));
    let req = req_builder.build();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(LOCATION, HeaderValue::from_static("https://public-cdn.com/file"));

    let mut visited = HashSet::new();
    let action = handle_redirect(
        &req,
        StatusCode::FOUND,
        &resp_headers,
        &mut visited,
        20,
    )
    .unwrap();

    match action {
        RedirectAction::Follow(follow) => {
            assert!(!follow.new_headers.contains_key(AUTHORIZATION));
            assert!(!follow.new_headers.contains_key(COOKIE));
            assert!(follow.new_headers.contains_key("x-custom-header"));
        }
        _ => panic!("Esperado Follow redirect"),
    }
}

#[test]
fn test_redirect_loop_exhaustion() {
    let req = Request::get("https://example.com/hop1").unwrap().build();
    let mut visited = HashSet::new();
    // Simula 20 saltos já ocorridos
    for i in 0..20 {
        visited.insert(format!("https://example.com/hop{}", i));
    }

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(LOCATION, HeaderValue::from_static("/hop21"));

    let result = handle_redirect(
        &req,
        StatusCode::FOUND,
        &resp_headers,
        &mut visited,
        20,
    );

    assert!(matches!(result, Err(NetError::TooManyRedirects(20))));
}
