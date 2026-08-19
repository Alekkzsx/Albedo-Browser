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

    // Teste de constantes de conveniência
    assert_eq!(MimeType::text_html().essence(), "text/html");
    assert_eq!(MimeType::text_css().essence(), "text/css");
    assert_eq!(MimeType::application_json().essence(), "application/json");
    assert_eq!(MimeType::image_svg().essence(), "image/svg+xml");
    assert_eq!(MimeType::font_woff2().essence(), "font/woff2");
    assert_eq!(MimeType::video_mp4().essence(), "video/mp4");
}

#[test]
fn test_mime_sniffing() {
    assert_eq!(sniff_mime_type(b"\x89PNG\r\n\x1a\n\x00\x00"), "image/png");
    assert_eq!(sniff_mime_type(b"\xFF\xD8\xFF\xE0\x00\x10"), "image/jpeg");
    assert_eq!(sniff_mime_type(b"GIF89a\x01\x00"), "image/gif");
    assert_eq!(sniff_mime_type(b"wOFF\x00\x01"), "font/woff");
    assert_eq!(sniff_mime_type(b"wOF2\x00\x01"), "font/woff2");
    assert_eq!(sniff_mime_type(b"%PDF-1.7\n"), "application/pdf");

    // Sniffing de mídia expandido
    assert_eq!(sniff_mime_type(b"\x00\x00\x00\x18ftypisom"), "video/mp4");
    assert_eq!(sniff_mime_type(b"\x1A\x45\xDF\xA3\x01\x00"), "video/webm");
    assert_eq!(sniff_mime_type(b"ID3\x04\x00\x00"), "audio/mpeg");
    assert_eq!(
        sniff_mime_type(b"RIFF\x00\x00\x00\x00WAVEfmt "),
        "audio/wav"
    );

    // Sniffing de texto/HTML/SVG
    assert_eq!(
        sniff_mime_type(b"<!DOCTYPE html><html><body></body></html>"),
        "text/html"
    );
    assert_eq!(sniff_mime_type(b"  \n\t <html lang=\"en\">"), "text/html");
    assert_eq!(
        sniff_mime_type(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"),
        "image/svg+xml"
    );
    assert_eq!(
        sniff_mime_type(b"<?xml version=\"1.0\"?>"),
        "application/xml"
    );
    assert_eq!(sniff_mime_type(b"Hello plain text world"), "text/plain");

    // Novos formatos de imagem e fontes
    assert_eq!(sniff_mime_type(b"\x00\x00\x00\x1cftypavif"), "image/avif");
    assert_eq!(sniff_mime_type(b"\x00\x00\x01\x00\x01\x00"), "image/x-icon");
    assert_eq!(sniff_mime_type(b"BM\x36\x00\x00\x00"), "image/bmp");
    assert_eq!(sniff_mime_type(b"II*\x00\x08\x00"), "image/tiff");
    assert_eq!(sniff_mime_type(b"\x00\x01\x00\x00\x00"), "font/ttf");
    assert_eq!(sniff_mime_type(b"OTTO\x00\x01"), "font/otf");
    assert_eq!(
        sniff_mime_type(b"<!-- comment --><svg></svg>"),
        "image/svg+xml"
    );
}

