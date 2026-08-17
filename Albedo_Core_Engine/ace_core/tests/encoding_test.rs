use ace_core::text::{detect_bom, WebEncoding};

#[test]
fn test_bom_detection() {
    let utf8_doc = b"\xEF\xBB\xBF<!DOCTYPE html><html></html>";
    assert_eq!(detect_bom(utf8_doc), Some((WebEncoding::Utf8, 3)));

    let utf16le_doc = b"\xFF\xFE<\x00h\x00t\x00m\x00l\x00";
    assert_eq!(detect_bom(utf16le_doc), Some((WebEncoding::Utf16Le, 2)));

    let utf16be_doc = b"\xFE\xFF\x00<\x00h\x00t\x00m\x00l";
    assert_eq!(detect_bom(utf16be_doc), Some((WebEncoding::Utf16Be, 2)));

    let plain_doc = b"<!DOCTYPE html>";
    assert_eq!(detect_bom(plain_doc), None);
}

#[test]
fn test_web_encoding_labels() {
    assert_eq!(WebEncoding::from_label("utf-8"), Some(WebEncoding::Utf8));
    assert_eq!(WebEncoding::from_label("latin1"), Some(WebEncoding::Iso8859_1));
    assert_eq!(WebEncoding::from_label("windows-1252"), Some(WebEncoding::Windows1252));
    assert_eq!(WebEncoding::from_label("sjis"), Some(WebEncoding::ShiftJis));
    assert_eq!(WebEncoding::from_label("invalid_encoding"), None);
}
