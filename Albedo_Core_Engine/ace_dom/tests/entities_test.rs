//! # Bateria de Testes de Entidades HTML (ace_dom)

use ace_dom::entities::{decode_character_reference, resolve_named_entity, resolve_numeric_entity};

#[test]
fn test_resolve_named_entities() {
    assert_eq!(resolve_named_entity("amp;"), Some("&"));
    assert_eq!(resolve_named_entity("lt;"), Some("<"));
    assert_eq!(resolve_named_entity("gt;"), Some(">"));
    assert_eq!(resolve_named_entity("quot;"), Some("\""));
    assert_eq!(resolve_named_entity("apos;"), Some("'"));
    assert_eq!(resolve_named_entity("copy;"), Some("©"));
    assert_eq!(resolve_named_entity("euro;"), Some("€"));
    assert_eq!(resolve_named_entity("trade;"), Some("™"));
    assert_eq!(resolve_named_entity("ccedil;"), Some("ç"));
    assert_eq!(resolve_named_entity("atilde;"), Some("ã"));
    assert_eq!(resolve_named_entity("infin;"), Some("∞"));
}

#[test]
fn test_resolve_numeric_entities() {
    // Caractere ASCII 'A'
    assert_eq!(resolve_numeric_entity(65), Some('A'));
    // Emoji Grinning Face U+1F600
    assert_eq!(resolve_numeric_entity(0x1F600), Some('😀'));
    // Mapeamentos legados Windows-1252
    assert_eq!(resolve_numeric_entity(0x80), Some('€')); // Euro
    assert_eq!(resolve_numeric_entity(0x93), Some('“')); // Left double quote
    assert_eq!(resolve_numeric_entity(0x94), Some('”')); // Right double quote
    // Inválidos / Nulos
    assert_eq!(resolve_numeric_entity(0), Some('\u{FFFD}'));
    assert_eq!(resolve_numeric_entity(0xD800), Some('\u{FFFD}'));
}

#[test]
fn test_decode_character_reference_stream() {
    let (res, consumed) = decode_character_reference("amp;resto").expect("amp;");
    assert_eq!(res.as_str(), "&");
    assert_eq!(consumed, 4);

    let (res_hex, c_hex) = decode_character_reference("#x1F600;tail").expect("hex emoji");
    assert_eq!(res_hex.as_str(), "😀");
    assert_eq!(c_hex, 9);

    let (res_dec, c_dec) = decode_character_reference("#169;tail").expect("dec copy");
    assert_eq!(res_dec.as_str(), "©");
    assert_eq!(c_dec, 5);
}
