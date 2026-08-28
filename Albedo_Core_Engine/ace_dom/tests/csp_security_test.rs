use ace_dom::security::{CSPPolicy, TrustedTypePolicy};

#[test]
fn test_csp_policy_parsing_and_inline_script_verification() {
    let csp_header = "default-src 'self'; script-src 'nonce-rAnd0m123' 'unsafe-eval' https://cdn.example.com; style-src 'self' 'unsafe-inline'";
    let policy = CSPPolicy::parse(csp_header);

    // 1. Script com nonce válido deve ser permitido
    assert!(policy.allows_inline_script(Some("console.log('hi');"), Some("rAnd0m123")));

    // 2. Script com nonce inválido deve ser bloqueado
    assert!(!policy.allows_inline_script(Some("console.log('hacked');"), Some("invalid-nonce")));

    // 3. Script sem nonce nem hash deve ser bloqueado (pois script-src não tem 'unsafe-inline')
    assert!(!policy.allows_inline_script(Some("evil();"), None));

    // 4. Script de domínio autorizado (CDN) deve ser permitido
    assert!(policy.allows_script_src("https://cdn.example.com/lib.js"));

    // 5. Script de domínio terceiro deve ser bloqueado
    assert!(!policy.allows_script_src("https://malicious.com/tracker.js"));
}

#[test]
fn test_trusted_types_policy_creation_and_enforcement() {
    let policy = TrustedTypePolicy::new("default")
        .with_create_html(|s| format!("[Sanitized: {s}]").into());

    let trusted = policy.create_html("<b>Safe markup</b>");
    assert_eq!(trusted.as_str(), "[Sanitized: <b>Safe markup</b>]");
}
