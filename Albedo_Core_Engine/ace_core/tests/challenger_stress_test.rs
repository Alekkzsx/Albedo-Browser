//! # Adversarial Stress Tests for Milestone M2 (Web Security & Net Hardening)
//!
//! Developed by Challenger M2 Agent to stress-test security boundaries,
//! CWE mitigations, RFC compliance, and parser resilience.

use ace_core::net::{is_safe_url_scheme, parse_data_uri, percent_decode, sniff_mime_type, MimeType};
use ace_core::security::{
    compute_referrer, matches_domain_pattern, Origin, ReferrerPolicy, UnguessableToken,
};
use rustc_hash::FxHashSet;

// ============================================================================
// 1. UnguessableToken (CWE-330): Entropy, Uniqueness, Distribution & Edge Cases
// ============================================================================

#[test]
fn test_adversarial_unguessable_token_distribution() {
    let mut seen = FxHashSet::default();
    const SAMPLES: usize = 10_000;
    let mut bit_counts = [0usize; 128];

    for _ in 0..SAMPLES {
        let token = UnguessableToken::new();
        assert!(!token.is_empty(), "Token must never be empty");
        
        let high = token.high();
        let low = token.low();
        assert!(high != 0 || low != 0, "Token bits must not be all zero");

        // Collect bit frequencies to test entropy distribution
        for bit in 0..64 {
            if (high & (1 << bit)) != 0 {
                bit_counts[bit] += 1;
            }
            if (low & (1 << bit)) != 0 {
                bit_counts[64 + bit] += 1;
            }
        }

        let inserted = seen.insert((high, low));
        assert!(inserted, "Cryptographic collision detected in UnguessableToken!");
    }

    assert_eq!(seen.len(), SAMPLES);

    // Each bit in a uniform 128-bit distribution over 10,000 samples should have ~5,000 ones
    // We allow a generous statistical margin [3500, 6500] (p < 10^-15 for any bit to fail this)
    for (i, &count) in bit_counts.iter().enumerate() {
        assert!(
            count >= 3500 && count <= 6500,
            "Bit {} has non-uniform distribution: count = {} out of {}",
            i,
            count,
            SAMPLES
        );
    }
}

#[test]
fn test_adversarial_unguessable_token_formatting_edge_cases() {
    // Check extreme values
    let zero_token = UnguessableToken::from_raw(0, 0);
    assert!(zero_token.is_empty());
    assert_eq!(zero_token.to_hex(), "00000000000000000000000000000000");
    assert_eq!(zero_token.to_hex().len(), 32);

    let max_token = UnguessableToken::from_raw(u64::MAX, u64::MAX);
    assert!(!max_token.is_empty());
    assert_eq!(max_token.to_hex(), "ffffffffffffffffffffffffffffffff");

    let mixed_token = UnguessableToken::from_raw(0x0000000000000001, 0x0000000000000002);
    assert_eq!(mixed_token.to_hex(), "00000000000000010000000000000002");
}

// ============================================================================
// 2. compute_referrer (CWE-200, RFC 9110 §10.1.4): Subdomains, Credentials, Downgrade
// ============================================================================

#[test]
fn test_adversarial_compute_referrer_edge_cases() {
    let origin = Origin::parse("https://user:pass@example.com:443/secret/path").unwrap();
    let current_url = "https://admin:super_secret@example.com/checkout/step2?token=xyz123#payment";

    // 1. Same-Origin must strip credentials and fragment
    let ref_same = compute_referrer(
        &origin,
        current_url,
        "https://example.com/dashboard",
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(
        ref_same,
        Some("https://example.com/checkout/step2?token=xyz123".to_string())
    );

    // 2. Cross-origin subdomain attack (example.com.evil.com) under SameOrigin must be None
    let evil_target = "https://example.com.evil.com/leak";
    let ref_evil = compute_referrer(
        &origin,
        current_url,
        evil_target,
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(ref_evil, None);

    // 3. Cross-origin subdomain attack under StrictOriginWhenCrossOrigin must ONLY reveal origin
    let ref_strict = compute_referrer(
        &origin,
        current_url,
        evil_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_strict, Some("https://example.com".to_string()));

    // 4. Downgrade HTTPS -> HTTP must be blocked under StrictOriginWhenCrossOrigin
    let insecure_target = "http://example.com/insecure";
    let ref_downgrade = compute_referrer(
        &origin,
        current_url,
        insecure_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_downgrade, None);

    // 5. Downgrade HTTPS -> HTTP under NoReferrerWhenDowngrade must be blocked
    let ref_no_downgrade = compute_referrer(
        &origin,
        current_url,
        insecure_target,
        ReferrerPolicy::NoReferrerWhenDowngrade,
    );
    assert_eq!(ref_no_downgrade, None);

    // 6. NoReferrer policy must ALWAYS return None
    assert_eq!(
        compute_referrer(
            &origin,
            current_url,
            "https://example.com/same",
            ReferrerPolicy::NoReferrer
        ),
        None
    );

    // 7. Non-HTTP current_url (e.g. data: or file:) must return None
    let file_origin = Origin::parse("file:///C:/doc.html").unwrap();
    assert_eq!(
        compute_referrer(
            &file_origin,
            "file:///C:/doc.html",
            "https://example.com",
            ReferrerPolicy::UnsafeUrl
        ),
        None
    );
}

// ============================================================================
// 3. matches_domain_pattern (W3C CSP3 §6.7.2): Wildcard Matching & Rejections
// ============================================================================

#[test]
fn test_adversarial_matches_domain_pattern_csp3() {
    // Valid wildcards matching subdomains
    assert!(matches_domain_pattern("*.example.com", "sub.example.com"));
    assert!(matches_domain_pattern("*.example.com", "a.b.example.com"));
    assert!(matches_domain_pattern("*.example.com", "deep.nested.sub.example.com"));
    assert!(matches_domain_pattern("*.EXAMPLE.COM", "sub.example.com"));
    assert!(matches_domain_pattern("*.example.com", "SUB.EXAMPLE.COM"));

    // W3C CSP3 §6.7.2: *.example.com must NOT match the apex domain
    assert!(!matches_domain_pattern("*.example.com", "example.com"));
    assert!(!matches_domain_pattern("*.EXAMPLE.COM", "EXAMPLE.COM"));

    // Prefix confusion attacks
    assert!(!matches_domain_pattern("*.example.com", "evil-example.com"));
    assert!(!matches_domain_pattern("*.example.com", "notexample.com"));
    assert!(!matches_domain_pattern("*.example.com", "fakeexample.com"));
    assert!(!matches_domain_pattern("*.example.com", ".example.com"));
    assert!(!matches_domain_pattern("*.example.com", "example.com.attacker.com"));

    // Global wildcard
    assert!(matches_domain_pattern("*", "anything.com"));
    assert!(matches_domain_pattern("*", "sub.domain.org"));

    // Exact matches
    assert!(matches_domain_pattern("example.com", "example.com"));
    assert!(matches_domain_pattern("example.com", "EXAMPLE.COM"));
    assert!(!matches_domain_pattern("example.com", "sub.example.com"));
}

// ============================================================================
// 4. Net Parser & MIME Sniffing: Malformed percent-decoding & 512B UTF-8 splits
// ============================================================================

#[test]
fn test_adversarial_percent_decode_robustness() {
    // Malformed sequences that should not lose data or panic
    assert_eq!(percent_decode("100%_concluido"), "100%_concluido");
    assert_eq!(percent_decode("ratio%"), "ratio%");
    assert_eq!(percent_decode("ratio%%"), "ratio%%");
    assert_eq!(percent_decode("ratio%1"), "ratio%1");
    assert_eq!(percent_decode("ratio%1Z"), "ratio%1Z");
    assert_eq!(percent_decode("ratio%ZZ"), "ratio%ZZ");
    assert_eq!(percent_decode("%%%20%%%"), "%% %%%");
    assert_eq!(percent_decode("user%40example.com"), "user@example.com");
}

#[test]
fn test_adversarial_mime_sniffing_and_boundary_conditions() {
    // Empty buffer
    assert_eq!(sniff_mime_type(&[]), "text/plain");

    // Pure binary non-UTF8 without magic numbers
    let garbage = [0xFF, 0xFE, 0xFD, 0x00, 0x11, 0x22];
    assert_eq!(sniff_mime_type(&garbage), "application/octet-stream");

    // HTML padded so that multibyte UTF-8 splits across the 512-byte boundary
    let mut html_padded = Vec::new();
    html_padded.extend_from_slice(b"<!DOCTYPE html><html><body>");
    while html_padded.len() < 510 {
        html_padded.push(b'x');
    }
    // 3-byte UTF-8 character '日' (0xE6, 0x97, 0xA5) split at 512
    html_padded.extend_from_slice(&[0xE6, 0x97, 0xA5]);
    html_padded.extend_from_slice(b"</body></html>");
    assert_eq!(sniff_mime_type(&html_padded), "text/html");

    // SVG padded similarly
    let mut svg_padded = Vec::new();
    svg_padded.extend_from_slice(b"<svg xmlns=\"http://www.w3.org/2000/svg\">");
    while svg_padded.len() < 511 {
        svg_padded.push(b' ');
    }
    // 4-byte UTF-8 emoji split at 512
    svg_padded.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80]);
    svg_padded.extend_from_slice(b"</svg>");
    assert_eq!(sniff_mime_type(&svg_padded), "image/svg+xml");
}
