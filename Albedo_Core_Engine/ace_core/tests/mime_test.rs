use ace_core::net::{sniff_mime_type, MimeType};

#[test]
fn test_mime_type_parsing_and_classification() {
    let html_mime = MimeType::parse("text/html; charset=UTF-8").unwrap();
    assert_eq!(html_mime.essence(), "text/html");
    assert_eq!(html_mime.charset(), Some("UTF-8"));
    assert!(html_mime.is_html());
    assert!(!html_mime.is_css());

    let css_mime = MimeType::parse("text/css").unwrap();
    assert_eq!(css_mime.essence(), "text/css");
    assert!(css_mime.is_css());
    assert!(!css_mime.is_html());

    let js_mime = MimeType::parse("application/javascript; charset=\"utf-8\"").unwrap();
    assert_eq!(js_mime.essence(), "application/javascript");
    assert_eq!(js_mime.charset(), Some("utf-8"));
    assert!(js_mime.is_javascript());

    let json_mime = MimeType::parse("application/ld+json").unwrap();
    assert!(json_mime.is_json());
}

#[test]
fn test_mime_sniffing() {
    assert_eq!(sniff_mime_type(b"\x89PNG\r\n\x1a\n\x00\x00"), "image/png");
    assert_eq!(sniff_mime_type(b"\xFF\xD8\xFF\xE0\x00\x10"), "image/jpeg");
    assert_eq!(sniff_mime_type(b"GIF89a\x01\x00"), "image/gif");
    assert_eq!(sniff_mime_type(b"wOFF\x00\x01"), "font/woff");
    assert_eq!(sniff_mime_type(b"%PDF-1.7\n"), "application/pdf");

    assert_eq!(sniff_mime_type(b"<!DOCTYPE html><html><body></body></html>"), "text/html");
    assert_eq!(sniff_mime_type(b"  \n\t <html lang=\"en\">"), "text/html");
    assert_eq!(sniff_mime_type(b"<?xml version=\"1.0\"?>"), "application/xml");
    assert_eq!(sniff_mime_type(b"Hello plain text world"), "text/plain");
}
