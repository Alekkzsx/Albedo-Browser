use ace_core::net::{percent_encode, percent_encode_byte, PercentEncodeSet};

#[test]
fn test_percent_encode_c0_control() {
    assert!(PercentEncodeSet::C0Control.contains(0x00));
    assert!(PercentEncodeSet::C0Control.contains(0x1F));
    assert!(!PercentEncodeSet::C0Control.contains(b' '));
    assert!(PercentEncodeSet::C0Control.contains(0x80));

    let encoded = percent_encode("Hello\x00World", PercentEncodeSet::C0Control);
    assert_eq!(encoded, "Hello%00World");
}

#[test]
fn test_percent_encode_query_and_path() {
    let raw = "user/repo?tag=1.0#section";

    let path_encoded = percent_encode(raw, PercentEncodeSet::Path);
    // No Path percent-encode set: '?' e '#' são codificados
    assert_eq!(path_encoded, "user/repo%3Ftag=1.0%23section");

    let query_encoded = percent_encode("q=hello world&tag=#1", PercentEncodeSet::Query);
    assert_eq!(query_encoded, "q=hello%20world&tag=%231");
}

#[test]
fn test_percent_encode_byte_lookup() {
    assert_eq!(
        percent_encode_byte(b' ', PercentEncodeSet::Query),
        Some(*b"%20")
    );
    assert_eq!(percent_encode_byte(b'a', PercentEncodeSet::Query), None);
}
