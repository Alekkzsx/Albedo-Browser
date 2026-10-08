//! # Testes de Integração SOTA: Fase 4 - MIME Sniffing, nosniff, CORP e Mixed Content Level 2
//!
//! Valida a conformidade normativa do `ace_net` com os padrões WHATWG MIME Sniffing,
//! W3C Cross-Origin Resource Policy (CORP), W3C Cross-Origin Isolation (COOP/COEP)
//! e W3C Mixed Content Level 2.

use ace_core::security::origin::Origin;
use ace_net::cache::NetworkIsolationKey;
use ace_net::http::request::{Request, RequestDestination};
use ace_net::http::sniffing::{
    is_nosniff_header_present, sniff_mime_type, validate_nosniff_content_type,
};
use ace_net::security::isolation::{
    extract_corp_policy, validate_corp, CoepPolicy, CoopPolicy, CorpPolicy, IsolationContext,
};
use ace_net::security::mixed_content::{
    check_and_apply_mixed_content, evaluate_mixed_content, MixedContentDecision,
};
use ace_net::{create_default_fetcher, NetError};
use http::HeaderMap;
use url::Url;

#[test]
fn test_whatwg_mime_sniffing_matrix() {
    // 1. PNG sniffing mesmo com Content-Type text/plain ou ausente
    let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01];
    assert_eq!(sniff_mime_type(None, &png_magic, false), "image/png");
    assert_eq!(
        sniff_mime_type(Some("text/plain"), &png_magic, false),
        "image/png"
    );

    // 2. JPEG sniffing
    let jpeg_magic = [0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43];
    assert_eq!(sniff_mime_type(None, &jpeg_magic, false), "image/jpeg");

    // 3. GIF sniffing
    let gif_magic = b"GIF89a\x01\x00\x01\x00";
    assert_eq!(sniff_mime_type(None, gif_magic, false), "image/gif");

    // 4. WebP sniffing (RIFF....WEBP)
    let webp_magic = b"RIFF\x20\x00\x00\x00WEBPVP8 ";
    assert_eq!(sniff_mime_type(None, webp_magic, false), "image/webp");

    // 5. HTML sniffing com espaços iniciais
    let html_magic = b"   \r\n\t  <!DOCTYPE html><html><head><title>Albedo</title></head></html>";
    assert_eq!(sniff_mime_type(None, html_magic, false), "text/html");

    // 6. XML sniffing
    let xml_magic = b"<?xml version=\"1.0\" encoding=\"utf-8\"?><catalog></catalog>";
    assert_eq!(sniff_mime_type(None, xml_magic, false), "application/xml");

    // 7. WebAssembly
    let wasm_magic = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    assert_eq!(sniff_mime_type(None, &wasm_magic, false), "application/wasm");

    // 8. PDF
    let pdf_magic = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";
    assert_eq!(sniff_mime_type(None, pdf_magic, false), "application/pdf");
}

#[test]
fn test_nosniff_header_and_validation() {
    let mut headers = HeaderMap::new();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    assert!(is_nosniff_header_present(&headers));

    let mut headers_case = HeaderMap::new();
    headers_case.insert("x-content-type-options", "NoSniff ".parse().unwrap());
    assert!(is_nosniff_header_present(&headers_case));

    // Com nosniff, se o servidor disser text/plain, o sniffer NÃO deve sobrescrever
    let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(
        sniff_mime_type(Some("text/plain"), &png_magic, true),
        "text/plain"
    );

    // Validação estrita de destinos de execução sob nosniff
    // Script com image/png ou text/plain deve ser bloqueado
    assert!(!validate_nosniff_content_type(RequestDestination::Script, "image/png"));
    assert!(!validate_nosniff_content_type(RequestDestination::Script, "text/plain"));
    assert!(!validate_nosniff_content_type(RequestDestination::Script, "text/html"));
    assert!(validate_nosniff_content_type(RequestDestination::Script, "application/javascript"));
    assert!(validate_nosniff_content_type(RequestDestination::Script, "text/javascript"));

    // Style com text/html deve ser bloqueado
    assert!(!validate_nosniff_content_type(RequestDestination::Style, "text/html"));
    assert!(validate_nosniff_content_type(RequestDestination::Style, "text/css"));
}

#[test]
fn test_corp_cross_origin_isolation_policies() {
    let origin_main = Origin::parse("https://albedo.org").unwrap();
    let origin_sub = Origin::parse("https://cdn.albedo.org").unwrap();
    let origin_attacker = Origin::parse("https://evil.com").unwrap();

    let target_url = Url::parse("https://albedo.org/api/data.json").unwrap();

    // 1. Same-Origin
    assert!(validate_corp(Some(&origin_main), &target_url, Some(CorpPolicy::SameOrigin)).is_ok());
    assert!(validate_corp(Some(&origin_sub), &target_url, Some(CorpPolicy::SameOrigin)).is_err());
    assert!(validate_corp(Some(&origin_attacker), &target_url, Some(CorpPolicy::SameOrigin)).is_err());

    // 2. Same-Site
    assert!(validate_corp(Some(&origin_main), &target_url, Some(CorpPolicy::SameSite)).is_ok());
    assert!(validate_corp(Some(&origin_sub), &target_url, Some(CorpPolicy::SameSite)).is_ok());
    assert!(validate_corp(Some(&origin_attacker), &target_url, Some(CorpPolicy::SameSite)).is_err());

    // 3. Cross-Origin
    assert!(validate_corp(Some(&origin_attacker), &target_url, Some(CorpPolicy::CrossOrigin)).is_ok());

    // 4. Header extraction
    let mut headers = HeaderMap::new();
    headers.insert("cross-origin-resource-policy", "same-origin".parse().unwrap());
    assert_eq!(extract_corp_policy(&headers), Some(CorpPolicy::SameOrigin));
}

#[test]
fn test_coop_coep_and_cross_origin_isolated_status() {
    let context_secure = IsolationContext::new(CoopPolicy::SameOrigin, CoepPolicy::RequireCorp);
    assert!(context_secure.is_cross_origin_isolated());

    let context_credentialless = IsolationContext::new(CoopPolicy::SameOrigin, CoepPolicy::Credentialless);
    assert!(context_credentialless.is_cross_origin_isolated());

    let context_unsafe = IsolationContext::new(CoopPolicy::UnsafeNone, CoepPolicy::RequireCorp);
    assert!(!context_unsafe.is_cross_origin_isolated());
}

#[test]
fn test_mixed_content_level_2_matrix() {
    let secure_parent = Origin::parse("https://bank.com").unwrap();
    let insecure_parent = Origin::parse("http://insecure-news.org").unwrap();

    // 1. Sub-recurso passivo (imagem) em página segura -> Auto-Upgrade silencioso para HTTPS
    let img_target = Url::parse("http://cdn.bank.com/logo.png").unwrap();
    let decision = evaluate_mixed_content(Some(&secure_parent), &img_target, RequestDestination::Image, false);
    match decision {
        MixedContentDecision::Upgraded(upgraded) => {
            assert_eq!(upgraded.scheme(), "https");
            assert_eq!(upgraded.as_str(), "https://cdn.bank.com/logo.png");
        }
        other => panic!("Esperado Upgraded, obtido {:?}", other),
    }

    // 2. Sub-recurso ativo (script) em página segura sem CSP upgrade -> Bloqueio estrito
    let script_target = Url::parse("http://cdn.bank.com/wallet.js").unwrap();
    let decision_active = evaluate_mixed_content(Some(&secure_parent), &script_target, RequestDestination::Script, false);
    match decision_active {
        MixedContentDecision::Blocked(reason) => {
            assert!(reason.contains("Conteúdo Misto Ativo"));
        }
        other => panic!("Esperado Blocked, obtido {:?}", other),
    }

    // 3. Sub-recurso ativo com CSP upgrade-insecure-requests ativo -> Auto-Upgrade para HTTPS
    let decision_active_csp = evaluate_mixed_content(Some(&secure_parent), &script_target, RequestDestination::Script, true);
    match decision_active_csp {
        MixedContentDecision::Upgraded(upgraded) => {
            assert_eq!(upgraded.scheme(), "https");
            assert_eq!(upgraded.as_str(), "https://cdn.bank.com/wallet.js");
        }
        other => panic!("Esperado Upgraded via CSP, obtido {:?}", other),
    }

    // 4. Página insegura carregando recurso HTTP -> Permitido
    let decision_insecure = evaluate_mixed_content(Some(&insecure_parent), &script_target, RequestDestination::Script, false);
    assert_eq!(decision_insecure, MixedContentDecision::Allowed);
}

#[tokio::test]
async fn test_fetcher_mixed_content_pipeline_interception() {
    let fetcher = create_default_fetcher().unwrap();

    let top_origin = Origin::parse("https://secure.albedo.dev").unwrap();
    let nik = NetworkIsolationKey::new(top_origin, Origin::new_opaque(), true);

    // Requisição ativa de Script em HTTP apontando para servidor externo
    let req = Request::get("http://third-party.com/analytics.js")
        .unwrap()
        .destination(RequestDestination::Script)
        .network_isolation_key(nik.clone())
        .build();

    let result = fetcher.fetch(req).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        NetError::SecurityViolation(msg) => {
            assert!(msg.contains("Conteúdo Misto Ativo bloqueado"));
        }
        other => panic!("Esperado SecurityViolation de Mixed Content, obtido {:?}", other),
    }

    // Requisição passiva de Imagem em HTTP -> É promovida silenciosamente para HTTPS antes de chamar a rede
    let mut img_url = Url::parse("http://example.com/banner.png").unwrap();
    let applied = check_and_apply_mixed_content(
        Some(&nik.top_frame_origin),
        &mut img_url,
        RequestDestination::Image,
        false,
    );
    assert!(applied.is_ok());
    assert_eq!(img_url.as_str(), "https://example.com/banner.png");
}

