#[path = "../src/ace/url/mod.rs"]
mod url_mod;
#[path = "../src/ace/url/types.rs"]
mod types;
#[path = "../src/ace/url/parser.rs"]
mod parser;
#[path = "../src/ace/url/percent_encoding.rs"]
mod percent_encoding;
#[path = "../src/ace/url/punycode.rs"]
mod punycode;
#[path = "../src/ace/url/search_params.rs"]
mod search_params;

use parser::parse;

#[test]
fn test_whatwg_compliance_host_parsing() {
    // IPv4
    let u = parse("http://127.0.0.1/", None).unwrap();
    assert_eq!(u.host_str(), Some("127.0.0.1".to_string()));

    // IPv6
    let u = parse("http://[2001:db8::1]/", None).unwrap();
    assert_eq!(u.host_str(), Some("[2001:db8::1]".to_string()));

    // IDN Punycode
    let u = parse("http://mañana.com/", None).unwrap();
    assert_eq!(u.host_str(), Some("xn--maana-pta.com".to_string()));
}

#[test]
fn test_whatwg_compliance_normalization() {
    // Scheme case
    let u = parse("HTTP://EXAMPLE.COM/", None).unwrap();
    assert_eq!(u.scheme, "http");
    assert_eq!(u.host_str(), Some("example.com".to_string()));

    // Backslash in special URLs
    let u = parse("https://google.com\\path", None).unwrap();
    assert_eq!(u.path(), "/path");

    // Path normalization
    let u = parse("https://example.com/foo/../bar", None).unwrap();
    assert_eq!(u.path(), "/bar");
}

#[test]
fn test_whatwg_relative_resolution() {
    let base = parse("https://example.com/a/b/c", None).unwrap();
    
    // Simple relative
    let u = base.join("d").unwrap();
    assert_eq!(u.to_string(), "https://example.com/a/b/d");

    // Double dot
    let u = base.join("../d").unwrap();
    assert_eq!(u.to_string(), "https://example.com/a/d");

    // Root relative
    let u = base.join("/d").unwrap();
    assert_eq!(u.to_string(), "https://example.com/d");
}

#[test]
fn test_whatwg_percent_encoding_robustness() {
    // Space in query
    let u = parse("https://ex.com/?q=a b", None).unwrap();
    assert_eq!(u.query.as_ref().map(|s| s.as_str()), Some("q=a%20b"));

    // Fragment normalization
    let u = parse("https://ex.com/#f o", None).unwrap();
    assert_eq!(u.fragment.as_ref().map(|s| s.as_str()), Some("f%20o"));
}
