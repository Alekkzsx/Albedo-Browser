//! Suíte de Testes de Segurança Web (Milestone M2 - R3)
//!
//! Validação de conformidade normativa RFC 6454, RFC 9110 §10.1.4, W3C CSP3 §6.7.2,
//! e mitigação rigorosa de CWE-330 e CWE-200.

use ace_core::security::{
    compute_referrer, matches_domain_pattern, Origin, ReferrerPolicy, UnguessableToken,
};
use rustc_hash::FxHashSet;

#[test]
fn test_unguessable_token_csprng_entropy_and_uniqueness() {
    let mut tokens = FxHashSet::default();
    const NUM_TOKENS: usize = 1_000;

    for _ in 0..NUM_TOKENS {
        let token = UnguessableToken::new();
        assert!(!token.is_empty(), "Token CSPRNG não pode ser nulo");
        assert!(token.high() != 0 || token.low() != 0);

        let hex = token.to_hex();
        assert_eq!(hex.len(), 32, "Hex deve ter exatamente 32 caracteres");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "Hex deve ser estritamente minúsculo: {}",
            hex
        );

        let inserted = tokens.insert((token.high(), token.low()));
        assert!(inserted, "Colisão detectada no CSPRNG UnguessableToken!");
    }

    assert_eq!(tokens.len(), NUM_TOKENS);
}

#[test]
fn test_unguessable_token_formatting_and_raw() {
    let token = UnguessableToken::from_raw(0x0123456789abcdef, 0xfedcba9876543210);
    assert_eq!(token.high(), 0x0123456789abcdef);
    assert_eq!(token.low(), 0xfedcba9876543210);

    let hex = token.to_hex();
    assert_eq!(hex, "0123456789abcdeffedcba9876543210");
    assert_eq!(format!("{}", token), "0123456789abcdeffedcba9876543210");
    assert_eq!(
        format!("{:?}", token),
        "UnguessableToken(0123456789abcdeffedcba9876543210)"
    );

    let empty = UnguessableToken::from_raw(0, 0);
    assert!(empty.is_empty());
}

#[test]
fn test_compute_referrer_cwe_200_subdomain_prefix_confusion() {
    let current_origin = Origin::parse("https://example.com").unwrap();
    let current_url = "https://example.com/checkout?item=42&user=secret";
    let evil_subdomain_target = "https://example.com.evil.com/phishing";

    // Política SameOrigin: DEVE bloquear totalmente o envio para o domínio malicioso
    let ref_same = compute_referrer(
        &current_origin,
        current_url,
        evil_subdomain_target,
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(
        ref_same, None,
        "Vulnerabilidade CWE-200: Subdomain Prefix Confusion permitiu envio de Referer para origem não confiável"
    );

    // Política StrictOriginWhenCrossOrigin: DEVE enviar APENAS a origem, nunca path ou query
    let ref_cross = compute_referrer(
        &current_origin,
        current_url,
        evil_subdomain_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(
        ref_cross,
        Some("https://example.com".to_string()),
        "Deve conter estritamente a origem serializada sem path/query"
    );
}

#[test]
fn test_compute_referrer_userinfo_and_fragment_stripping() {
    let origin = Origin::parse("https://example.com").unwrap();
    let current_url = "https://admin:SuperSecretPassword123@example.com/private/dashboard#section4";
    let same_target = "https://example.com/api/v1/resource";

    let ref_result = compute_referrer(
        &origin,
        current_url,
        same_target,
        ReferrerPolicy::SameOrigin,
    );

    // As credenciais de autenticação e o fragmento '#' DEVEM ser rigorosamente expurgados
    assert_eq!(
        ref_result,
        Some("https://example.com/private/dashboard".to_string()),
        "Vazamento de credenciais (userinfo) ou fragmento no cabeçalho Referer"
    );
}

#[test]
fn test_compute_referrer_downgrade_and_schemes() {
    let https_origin = Origin::parse("https://secure.example.com").unwrap();
    let https_url = "https://secure.example.com/page";
    let http_target = "http://insecure.example.com/api";

    // Downgrade HTTPS -> HTTP bloqueado
    assert_eq!(
        compute_referrer(
            &https_origin,
            https_url,
            http_target,
            ReferrerPolicy::NoReferrerWhenDowngrade
        ),
        None
    );
    assert_eq!(
        compute_referrer(
            &https_origin,
            https_url,
            http_target,
            ReferrerPolicy::StrictOrigin
        ),
        None
    );
    assert_eq!(
        compute_referrer(
            &https_origin,
            https_url,
            http_target,
            ReferrerPolicy::StrictOriginWhenCrossOrigin
        ),
        None
    );

    // Esquemas não-HTTP rejeitados
    let file_origin = Origin::parse("file:///local/file.html").unwrap();
    assert_eq!(
        compute_referrer(
            &file_origin,
            "file:///local/file.html",
            "https://example.com",
            ReferrerPolicy::UnsafeUrl
        ),
        None
    );
}

#[test]
fn test_origin_port_serialization_rfc_6454() {
    // Arquivos locais não devem ter ':0'
    let file_origin = Origin::parse("file:///C:/Users/test/index.html").unwrap();
    assert_eq!(file_origin.ascii_serialization(), "file://");

    // Esquemas customizados não devem ter ':0' quando porta for omitida
    let custom_origin = Origin::parse("albedo://settings").unwrap();
    assert_eq!(custom_origin.ascii_serialization(), "albedo://settings");

    // Portas padrão omitidas
    let http_std = Origin::parse("http://example.com:80/").unwrap();
    assert_eq!(http_std.ascii_serialization(), "http://example.com");

    let https_std = Origin::parse("https://example.com:443/").unwrap();
    assert_eq!(https_std.ascii_serialization(), "https://example.com");

    // Portas não-padrão preservadas
    let http_custom = Origin::parse("http://example.com:8080/").unwrap();
    assert_eq!(http_custom.ascii_serialization(), "http://example.com:8080");

    let https_custom = Origin::parse("https://example.com:8443/").unwrap();
    assert_eq!(https_custom.ascii_serialization(), "https://example.com:8443");
}

#[test]
fn test_matches_domain_pattern_w3c_csp3() {
    // Wildcard casa subdomínios
    assert!(matches_domain_pattern("*.example.com", "sub.example.com"));
    assert!(matches_domain_pattern("*.example.com", "a.b.c.example.com"));
    assert!(matches_domain_pattern("*.example.com", "api-v2.example.com"));

    // W3C CSP3 §6.7.2: *.example.com NUNCA casa com o apex domain
    assert!(!matches_domain_pattern("*.example.com", "example.com"));

    // Não deve casar com sufixos parciais sem ponto separador
    assert!(!matches_domain_pattern("*.example.com", "notexample.com"));
    assert!(!matches_domain_pattern("*.example.com", "badexample.com"));
    assert!(!matches_domain_pattern("*.example.com", ".example.com"));

    // Não deve casar com domínios prefixados no alvo
    assert!(!matches_domain_pattern("*.example.com", "example.com.attacker.com"));

    // Case-insensitivity
    assert!(matches_domain_pattern("*.EXAMPLE.COM", "SUB.example.com"));
    assert!(matches_domain_pattern("*.example.com", "SUB.EXAMPLE.COM"));

    // Padrões exatos e globais
    assert!(matches_domain_pattern("example.com", "example.com"));
    assert!(matches_domain_pattern("example.com", "EXAMPLE.COM"));
    assert!(matches_domain_pattern("*", "anything.domain.org"));
}
