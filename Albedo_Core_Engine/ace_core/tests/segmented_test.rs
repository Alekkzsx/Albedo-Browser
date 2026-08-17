use ace_core::text::SegmentedString;

#[test]
fn test_segmented_string_basic_advance() {
    let mut s = SegmentedString::from_static_str("hello world");
    assert_eq!(s.peek(), Some('h'));
    assert_eq!(s.advance(), Some('h'));
    assert_eq!(s.advance(), Some('e'));
    assert_eq!(s.peek(), Some('l'));

    let remaining = s.consume_while(|c| c != ' ');
    assert_eq!(remaining, "llo");
    assert_eq!(s.advance(), Some(' '));
    assert_eq!(s.consume_while(|_| true), "world");
    assert!(s.is_eof());
}

#[test]
fn test_segmented_string_unconsume_push_front() {
    let mut s = SegmentedString::from_static_str("bar");
    assert_eq!(s.advance(), Some('b'));
    assert_eq!(s.advance(), Some('a'));

    // Devolve 'a' e 'b'
    s.push_front_char('a');
    s.push_front_char('b');

    assert_eq!(s.advance(), Some('b'));
    assert_eq!(s.advance(), Some('a'));
    assert_eq!(s.advance(), Some('r'));
    assert!(s.is_eof());
}

#[test]
fn test_segmented_string_push_front_str() {
    let mut s = SegmentedString::from_static_str("world");
    s.push_front_str("hello ");

    let consumed = s.consume_while(|_| true);
    assert_eq!(consumed, "hello world");
    assert!(s.is_eof());
}

#[test]
fn test_segmented_string_streaming_chunks() {
    let mut s = SegmentedString::new();
    assert!(s.is_eof());

    s.append_chunk("<div");
    s.append_chunk(" class=");
    s.append_chunk("\"main\">");

    assert!(!s.is_eof());
    assert!(s.starts_with("<div"));
    assert!(s.consume_prefix("<div"));
    assert_eq!(s.advance(), Some(' '));

    let attr = s.consume_while(|c| c != '=');
    assert_eq!(attr, "class");
    assert_eq!(s.advance(), Some('='));
    assert_eq!(s.advance(), Some('"'));

    let val = s.consume_while(|c| c != '"');
    assert_eq!(val, "main");
    assert_eq!(s.advance(), Some('"'));
    assert_eq!(s.advance(), Some('>'));
    assert!(s.is_eof());
}

#[test]
fn test_segmented_string_source_location_tracking() {
    let mut s = SegmentedString::from_static_str("line 1\nline 2\nline 3").with_url("https://example.com");

    let loc = s.location();
    assert_eq!(loc.line, 1);
    assert_eq!(loc.column, 1);

    s.consume_while(|c| c != '\n');
    assert_eq!(s.advance(), Some('\n'));

    let loc = s.location();
    assert_eq!(loc.line, 2);
    assert_eq!(loc.column, 1);
}

#[test]
fn test_segmented_string_peek_at() {
    let s = SegmentedString::from_static_str("abcdef");
    assert_eq!(s.peek_at(0), Some('a'));
    assert_eq!(s.peek_at(1), Some('b'));
    assert_eq!(s.peek_at(5), Some('f'));
    assert_eq!(s.peek_at(6), None);
}
