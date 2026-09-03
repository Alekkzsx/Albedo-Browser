//! # Testes de Priorização Extensível HTTP (RFC 9218) e Injeção de Cabeçalhos Padrão

use ace_net::priority::PriorityLevel;
use ace_net::request::{Request, RequestDestination};
use http::header::ACCEPT_ENCODING;

#[test]
fn test_rfc9218_header_values() {
    assert_eq!(PriorityLevel::VeryHigh.to_rfc9218_header().to_str().unwrap(), "u=0");
    assert_eq!(PriorityLevel::High.to_rfc9218_header().to_str().unwrap(), "u=1");
    assert_eq!(PriorityLevel::Medium.to_rfc9218_header().to_str().unwrap(), "u=3, i");
    assert_eq!(PriorityLevel::Low.to_rfc9218_header().to_str().unwrap(), "u=5, i");
    assert_eq!(PriorityLevel::Lowest.to_rfc9218_header().to_str().unwrap(), "u=7, i");
}

#[test]
fn test_request_builder_auto_injects_priority_and_accept_encoding() {
    let req = Request::get("https://example.com/main.css")
        .unwrap()
        .destination(RequestDestination::Style)
        .build();

    // Injeção automática de RFC 9218 Priority header
    let priority_val = req.headers.get("priority").expect("Deve conter cabeçalho priority");
    assert_eq!(priority_val.to_str().unwrap(), "u=1");

    // Injeção automática de Accept-Encoding com codecs modernos
    let accept_enc = req.headers.get(ACCEPT_ENCODING).expect("Deve conter Accept-Encoding");
    assert_eq!(accept_enc.to_str().unwrap(), "gzip, deflate, br");
}
