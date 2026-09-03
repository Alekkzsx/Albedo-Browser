//! # Testes de Detecção de Encodings e Content Sniffing

use ace_net::encoding::{decode_to_string, extract_charset_from_content_type, sniff_bom};

#[test]
fn test_content_type_charset_parsing() {
    assert_eq!(
        extract_charset_from_content_type("text/html; charset=utf-8").as_deref(),
        Some("utf-8")
    );
    assert_eq!(
        extract_charset_from_content_type("text/html; charset=\"UTF-8\"").as_deref(),
        Some("utf-8")
    );
    assert_eq!(
        extract_charset_from_content_type("application/javascript; charset=iso-8859-1; version=2").as_deref(),
        Some("iso-8859-1")
    );
    assert_eq!(
        extract_charset_from_content_type("image/png").as_deref(),
        None
    );
}

#[test]
fn test_bom_detection_utf8() {
    let raw = [0xEF, 0xBB, 0xBF, b'<', b'h', b'1', b'>'];
    let detected = sniff_bom(&raw).expect("Deve detectar UTF-8 BOM");
    assert_eq!(detected.name, "utf-8");
    assert_eq!(detected.bom_offset, 3);

    let decoded = decode_to_string(&raw, None);
    assert_eq!(decoded, "<h1>");
}

#[test]
fn test_bom_detection_utf16le() {
    // "<h" em UTF-16LE: BOM=FF FE, '<'=3C 00, 'h'=68 00
    let raw = [0xFF, 0xFE, 0x3C, 0x00, 0x68, 0x00];
    let detected = sniff_bom(&raw).expect("Deve detectar UTF-16LE BOM");
    assert_eq!(detected.name, "utf-16le");

    let decoded = decode_to_string(&raw, None);
    assert_eq!(decoded, "<h");
}

#[test]
fn test_decode_windows_1252_special_characters() {
    // Caractere '€' em Windows-1252 é o byte 0x80
    let bytes = [0x80, b' ', b'1', b'0', b'0'];
    let decoded = decode_to_string(&bytes, Some("windows-1252"));
    assert_eq!(decoded, "€ 100");
}
