use ace_net::cookie::Cookie;
use ace_net::http::auth::{HttpAuthChallenge, HttpAuthCredentials, HttpAuthManager};
use ace_net::http::request::RequestBuilder;
use ace_net::telemetry::nqe::{EffectiveConnectionType, NetworkQualityEstimator};
use ace_net::transport::DohHappyEyeballsResolver;
use ace_net::{CacheEntry, ResourceFetcher};
use bytes::Bytes;
use http::header::{HeaderMap, HeaderValue, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, WWW_AUTHENTICATE};
use http::{Method, StatusCode};
use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use url::Url;

// =========================================================================
// 1. Testes de Blindagem de Prefixos de Cookies e SameSite=None (RFC 6265bis)
// =========================================================================

#[test]
fn test_cookie_prefix_and_samesite_none_enforcement() {
    let https_url = Url::parse("https://bank.example.com/dashboard").unwrap();
    let http_url = Url::parse("http://bank.example.com/dashboard").unwrap();
    let now = SystemTime::now();

    // 1. __Host- válido: HTTPS, Secure, Path=/, sem Domain
    let valid_host = Cookie::parse("__Host-session=abc123; Secure; Path=/", &https_url, None, now);
    assert!(valid_host.is_some(), "__Host- válido DEVE ser aceito");
    let c = valid_host.unwrap();
    assert_eq!(c.name, "__Host-session");
    assert_eq!(c.value, "abc123");
    assert_eq!(c.path, "/");

    // 2. __Host- inválido com Domain especificado -> Deve ser rejeitado (RFC 6265bis §4.1.3)
    let invalid_host_domain = Cookie::parse(
        "__Host-session=abc123; Secure; Domain=example.com; Path=/",
        &https_url,
        None,
        now,
    );
    assert!(invalid_host_domain.is_none(), "__Host- com atributo Domain DEVE ser rejeitado");

    // 3. __Host- inválido com Path diferente de "/" -> Deve ser rejeitado
    let invalid_host_path = Cookie::parse(
        "__Host-session=abc123; Secure; Path=/dashboard",
        &https_url,
        None,
        now,
    );
    assert!(invalid_host_path.is_none(), "__Host- com Path != / DEVE ser rejeitado");

    // 4. __Host- inválido sem flag Secure -> Deve ser rejeitado
    let invalid_host_no_sec = Cookie::parse("__Host-session=abc123; Path=/", &https_url, None, now);
    assert!(invalid_host_no_sec.is_none(), "__Host- sem Secure DEVE ser rejeitado");

    // 5. __Host- inválido emitido por HTTP inseguro -> Deve ser rejeitado
    let invalid_host_http = Cookie::parse("__Host-session=abc123; Secure; Path=/", &http_url, None, now);
    assert!(invalid_host_http.is_none(), "__Host- sob HTTP DEVE ser rejeitado");

    // 6. __Secure- válido: HTTPS, Secure, Path arbitrário permitido
    let valid_secure = Cookie::parse("__Secure-token=xyz; Secure; Path=/api", &https_url, None, now);
    assert!(valid_secure.is_some(), "__Secure- com Secure sobre HTTPS DEVE ser aceito");

    // 7. __Secure- inválido sem flag Secure -> Deve ser rejeitado
    let invalid_secure_no_sec = Cookie::parse("__Secure-token=xyz; Path=/api", &https_url, None, now);
    assert!(invalid_secure_no_sec.is_none(), "__Secure- sem Secure DEVE ser rejeitado");

    // 8. __Secure- inválido sob HTTP -> Deve ser rejeitado
    let invalid_secure_http = Cookie::parse("__Secure-token=xyz; Secure; Path=/api", &http_url, None, now);
    assert!(invalid_secure_http.is_none(), "__Secure- sob HTTP DEVE ser rejeitado");

    // 9. SameSite=None sem Secure -> DEVE ser rejeitado (RFC 6265bis §5.4)
    let invalid_samesite_none = Cookie::parse("tracker=1; SameSite=None", &https_url, None, now);
    assert!(invalid_samesite_none.is_none(), "SameSite=None sem Secure DEVE ser rejeitado");

    // 10. SameSite=None sob HTTP -> DEVE ser rejeitado
    let invalid_samesite_http = Cookie::parse("tracker=1; SameSite=None; Secure", &http_url, None, now);
    assert!(invalid_samesite_http.is_none(), "SameSite=None sob HTTP DEVE ser rejeitado");

    // 11. SameSite=None com Secure sobre HTTPS -> Válido
    let valid_samesite_none = Cookie::parse("tracker=1; SameSite=None; Secure", &https_url, None, now);
    assert!(valid_samesite_none.is_some(), "SameSite=None com Secure sobre HTTPS DEVE ser aceito");
}

// =========================================================================
// 2. Testes de Network Quality Estimator (NQE) e Network Client Hints
// =========================================================================

#[test]
fn test_nqe_ewma_decay_and_client_hints() {
    let nqe = NetworkQualityEstimator::new();
    assert_eq!(nqe.effective_connection_type(), EffectiveConnectionType::Unknown);

    // Observação 1: RTT de 40ms, 500 KB em 100ms (~40.000 kbps) -> 4G
    nqe.record_observation(Duration::from_millis(40), 500_000, Duration::from_millis(100));
    assert_eq!(nqe.effective_connection_type(), EffectiveConnectionType::FourG);
    assert!(nqe.http_rtt().unwrap().as_millis() <= 50);

    // Injeção de Client Hints com quantização
    let mut headers = HeaderMap::new();
    nqe.inject_network_client_hints(&mut headers);

    assert_eq!(headers.get("ect").unwrap(), "4g");
    let rtt_val: u64 = headers.get("rtt").unwrap().to_str().unwrap().parse().unwrap();
    // Deve ser arredondado para múltiplo de 25ms
    assert_eq!(rtt_val % 25, 0, "RTT deve ser quantizado em múltiplos de 25ms");

    // Observação de latência de transporte (TCP/TLS connect time)
    nqe.record_transport_rtt(Duration::from_millis(18));
    assert!(nqe.transport_rtt().is_some());

    // Simula degradação severa da rede: RTT de 2500ms, 2 KB em 4s (~4 kbps)
    let slow_nqe = NetworkQualityEstimator::new();
    slow_nqe.record_observation(Duration::from_millis(2500), 2000, Duration::from_secs(4));
    assert_eq!(slow_nqe.effective_connection_type(), EffectiveConnectionType::Slow2G);

    let mut slow_headers = HeaderMap::new();
    slow_nqe.inject_network_client_hints(&mut slow_headers);
    assert_eq!(slow_headers.get("ect").unwrap(), "slow-2g");
}

// =========================================================================
// 3. Testes de Suporte a Cache-Control: immutable (RFC 8246)
// =========================================================================

#[test]
fn test_cache_control_immutable_directive() {
    let now = SystemTime::now();

    let mut imm_headers = HeaderMap::new();
    imm_headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    let imm_entry = CacheEntry::new(
        Url::parse("https://cdn.example.com/app.abcdef12.js").unwrap(),
        StatusCode::OK,
        imm_headers,
        Bytes::from_static(b"console.log('immutable app');"),
        now,
        now,
    );
    assert!(imm_entry.is_immutable(), "Deve identificar diretiva immutable");
    assert!(imm_entry.is_fresh(now));

    let mut mut_headers = HeaderMap::new();
    mut_headers.insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=3600"));
    let mut_entry = CacheEntry::new(
        Url::parse("https://cdn.example.com/app.js").unwrap(),
        StatusCode::OK,
        mut_headers,
        Bytes::from_static(b"console.log('mutable app');"),
        now,
        now,
    );
    assert!(!mut_entry.is_immutable(), "Não deve identificar immutable quando ausente");
}

// =========================================================================
// 4. Testes de HTTP Authentication Manager (RFC 7235 / 7617) e 401 Auto-Retry
// =========================================================================

#[tokio::test]
async fn test_http_auth_challenge_parsing_and_manager() {
    let header_str = "Basic realm=\"Restricted Administration\", charset=\"UTF-8\"";
    let challenge = HttpAuthChallenge::parse(header_str).expect("Parsing do desafio deve funcionar");
    assert_eq!(challenge.scheme, "Basic");
    assert_eq!(challenge.realm, "Restricted Administration");

    let manager = HttpAuthManager::new();
    let origin = "https://admin.corp";
    manager.cache().insert(
        origin,
        "Restricted Administration",
        HttpAuthCredentials::new("admin", "secret_pass"),
    );

    let auth_header = manager.resolve_authorization_header(origin, &challenge);
    assert!(auth_header.is_some());
    // "admin:secret_pass" base64 -> "YWRtaW46c2VjcmV0X3Bhc3M="
    assert_eq!(auth_header.unwrap(), "Basic YWRtaW46c2VjcmV0X3Bhc3M=");
}

#[tokio::test]
async fn test_resource_fetcher_401_challenge_auto_retry() {
    use http_body_util::Full;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    // Servidor mock: primeira requisição retorna 401 Unauthorized com WWW-Authenticate;
    // segunda requisição contendo cabeçalho Authorization correto retorna 200 OK.
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let attempts_inner = attempts_clone.clone();
                let io = hyper_util::rt::TokioIo::new(stream);
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                                let _attempt_idx = attempts_inner.fetch_add(1, Ordering::SeqCst);
                                async move {
                                    if let Some(auth) = req.headers().get(AUTHORIZATION) {
                                        if auth.to_str().unwrap() == "Basic dXNlcjpwYXNzMTIz" {
                                            // Credenciais válidas!
                                            let mut resp = hyper::Response::new(Full::new(Bytes::from("Authenticated Access Granted")));
                                            *resp.status_mut() = StatusCode::OK;
                                            resp.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                                            return Ok::<_, Infallible>(resp);
                                        }
                                    }

                                    // Sem autorização ou credenciais inválidas: retorna 401
                                    let mut resp = hyper::Response::new(Full::new(Bytes::from("Unauthorized")));
                                    *resp.status_mut() = StatusCode::UNAUTHORIZED;
                                    resp.headers_mut().insert(
                                        WWW_AUTHENTICATE,
                                        HeaderValue::from_static("Basic realm=\"Secure Vault\""),
                                    );
                                    Ok::<_, Infallible>(resp)
                                }
                            }),
                        )
                        .await;
                });
            } else {
                break;
            }
        }
    });

    let fetcher = ResourceFetcher::new().unwrap();
    let url_str = format!("http://{}:{}/vault/secrets", addr.ip(), addr.port());
    let origin_str = format!("http://{}:{}", addr.ip(), addr.port());

    // Preenche o cache de autenticação com credenciais para o realm "Secure Vault"
    fetcher.auth_manager().cache().insert(
        &origin_str,
        "Secure Vault",
        HttpAuthCredentials::new("user", "pass123"),
    );

    let req = RequestBuilder::new(Url::parse(&url_str).unwrap(), Method::GET).build();
    let response = fetcher.fetch(req).await.expect("Fetch deve concluir com sucesso após retry");

    // O status final entregue para o chamador DEVE ser 200 OK (após auto-retry)
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.as_bytes().as_ref(), b"Authenticated Access Granted");
    // O servidor recebeu exatamente 2 tentativas (1 sem auth -> 401, 1 com auth -> 200)
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

// =========================================================================
// 5. Testes de Resiliência de DNS (DoH com Fallback de Sistema Operacional)
// =========================================================================

#[tokio::test]
async fn test_dns_resolver_fallback_creation() {
    let resolver = DohHappyEyeballsResolver::new();
    let _cloned = resolver.clone();
    let empty = DohHappyEyeballsResolver::interleave_happy_eyeballs(vec![]);
    assert!(empty.is_empty());
}

