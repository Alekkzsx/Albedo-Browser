use ace_core::security::Origin;

#[test]
fn test_origin_same_origin_policy() {
    let o1 = Origin::parse("https://example.com/path/to/page").unwrap();
    let o2 = Origin::parse("https://example.com/another/page").unwrap();
    let o3 = Origin::parse("http://example.com/path").unwrap(); // Esquema diferente
    let o4 = Origin::parse("https://example.com:8443/").unwrap(); // Porta diferente

    assert!(o1.same_origin(&o2));
    assert!(!o1.same_origin(&o3));
    assert!(!o1.same_origin(&o4));

    assert_eq!(o1.ascii_serialization(), "https://example.com");
    assert_eq!(o4.ascii_serialization(), "https://example.com:8443");
    assert!(o1.is_secure());
    assert!(!o3.is_secure());
}

#[test]
fn test_opaque_origins() {
    let op1 = Origin::new_opaque();
    let op2 = Origin::new_opaque();

    assert!(op1.is_opaque());
    assert_eq!(op1.ascii_serialization(), "null");
    // Origens opacas nunca são Same-Origin entre si
    assert!(!op1.same_origin(&op2));
    assert!(!op1.is_secure());
}

#[test]
fn test_localhost_is_secure() {
    let local = Origin::parse("http://localhost:3000/").unwrap();
    assert!(local.is_secure());

    let loopback = Origin::parse("http://127.0.0.1:8080/").unwrap();
    assert!(loopback.is_secure());
}
