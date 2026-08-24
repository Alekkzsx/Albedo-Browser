use ace_core::text::{
    collapse_html_whitespace, count_utf16_units, is_html_whitespace, trim_html_whitespace,
    utf16_offset_to_utf8_byte, utf8_byte_to_utf16_offset,
};

#[test]
fn test_utf8_utf16_mapping_with_emojis_and_accents() {
    // String mista: "A" (1 byte, 1 utf16), "é" (2 bytes, 1 utf16), "🦀" (4 bytes, 2 utf16)
    let s = "Aé🦀B";
    assert_eq!(s.len(), 1 + 2 + 4 + 1); // 8 bytes UTF-8
    assert_eq!(count_utf16_units(s), 1 + 1 + 2 + 1); // 5 code units UTF-16

    // Início de "🦀" está em byte offset 3
    let u16_offset = utf8_byte_to_utf16_offset(s, 3);
    assert_eq!(u16_offset, 2); // 'A' (1) + 'é' (1)

    // Fim de "🦀" está em byte offset 7
    let u16_offset_end = utf8_byte_to_utf16_offset(s, 7);
    assert_eq!(u16_offset_end, 4); // 'A' (1) + 'é' (1) + '🦀' (2)

    // Conversão reversa: índice UTF-16 4 -> byte offset 7 ('B')
    let byte_offset = utf16_offset_to_utf8_byte(s, 4);
    assert_eq!(byte_offset, 7);
    assert_eq!(&s[byte_offset..], "B");
}

#[test]
fn test_html_whitespace_rules() {
    assert!(is_html_whitespace(' '));
    assert!(is_html_whitespace('\t'));
    assert!(is_html_whitespace('\n'));
    assert!(is_html_whitespace('\r'));
    assert!(is_html_whitespace('\x0C')); // Form feed
    assert!(!is_html_whitespace('A'));
    assert!(!is_html_whitespace('\u{00A0}')); // Non-breaking space NÃO é whitespace HTML

    let raw = "\n\t  <div>  Hello   \n\t World </div>  \n";
    let trimmed = trim_html_whitespace(raw);
    assert_eq!(trimmed, "<div>  Hello   \n\t World </div>");

    let collapsed = collapse_html_whitespace("Hello \t\n  World!");
    assert_eq!(collapsed, "Hello World!");
}
