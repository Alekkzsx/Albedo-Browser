use ace_core::cursor::{ByteCursor, CharCursor};

#[test]
fn test_char_cursor_location_and_consumption() {
    let code = "div {\n  color: red;\n}";
    let mut cursor = CharCursor::new(code);

    assert_eq!(cursor.line(), 1);
    assert_eq!(cursor.column(), 1);
    assert_eq!(cursor.byte_offset(), 0);

    let tag = cursor.consume_while(|c| c.is_alphabetic());
    assert_eq!(tag, "div");
    assert_eq!(cursor.column(), 4);

    assert!(cursor.starts_with(" {"));
    assert!(cursor.consume_prefix(" {\n"));
    assert_eq!(cursor.line(), 2);
    assert_eq!(cursor.column(), 1);

    cursor.consume_while(|c| c.is_whitespace());
    assert_eq!(cursor.column(), 3);

    let prop = cursor.consume_while(|c| c.is_alphabetic());
    assert_eq!(prop, "color");

    let loc = cursor.location();
    assert_eq!(loc.line, 2);
    assert_eq!(loc.column, 8);
}

#[test]
fn test_byte_cursor_binary_reading() {
    let data = [0x12, 0x34, 0xAB, 0xCD, 0x56, 0x78, 0x9A, 0xBC];
    let mut cursor = ByteCursor::new(&data);

    assert_eq!(cursor.read_u16_be(), Some(0x1234));
    assert_eq!(cursor.pos(), 2);
    assert_eq!(cursor.read_u32_be(), Some(0xABCD5678));
    assert_eq!(cursor.pos(), 6);
    assert_eq!(cursor.read_bytes(2), Some(&[0x9A, 0xBC][..]));
    assert!(cursor.is_eof());
    assert_eq!(cursor.read_bytes(1), None);
}
