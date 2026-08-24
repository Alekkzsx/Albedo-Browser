use ace_core::security::Origin;

#[test]
fn test_schemeful_site_from_standard_origin() {
    let origin = Origin::parse("https://mail.google.com").unwrap();
    let site = origin.to_site();

    assert_eq!(site.registrable_domain(), "google.com");
    assert_eq!(site.to_string(), "https://google.com");
    assert!(!site.is_opaque());
}

#[test]
fn test_schemeful_site_multipart_tld() {
    let origin1 = Origin::parse("https://news.bbc.co.uk").unwrap();
    let site1 = origin1.to_site();
    assert_eq!(site1.registrable_domain(), "bbc.co.uk");
    assert_eq!(site1.to_string(), "https://bbc.co.uk");

    let origin2 = Origin::parse("https://www.ufrj.br").unwrap();
    let site2 = origin2.to_site();
    assert_eq!(site2.registrable_domain(), "ufrj.br");
}

#[test]
fn test_same_site_comparison() {
    let origin_a = Origin::parse("https://auth.example.com").unwrap();
    let origin_b = Origin::parse("https://store.example.com:8443").unwrap();
    let origin_c = Origin::parse("http://example.com").unwrap(); // Esquema diferente

    assert!(!origin_a.same_origin(&origin_b)); // Cross-Origin
    assert!(origin_a.is_same_site(&origin_b)); // Same-Site!

    assert!(!origin_a.is_same_site(&origin_c)); // Schemeful Site diferente (https vs http)
}

#[test]
fn test_opaque_site() {
    let opaque_origin1 = Origin::new_opaque();
    let opaque_origin2 = Origin::new_opaque();

    let site1 = opaque_origin1.to_site();
    let site2 = opaque_origin2.to_site();

    assert!(site1.is_opaque());
    assert_eq!(site1.to_string(), "null");
    assert!(!site1.same_site(&site2)); // Sites opacos nunca são Same-Site
}
