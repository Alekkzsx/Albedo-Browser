use ace_dom::tokenizer::simd::{
    fast_ascii_eq_ignore_case, find_comment_dash, find_html_text_delimiter, find_quote,
    find_tag_close, find_unquoted_attr_end, skip_ascii_whitespace_simd,
};

#[test]
fn test_simd_html_text_delimiter_searching() {
    let plain_text = b"This is a very long segment of text without any tags or entities here.";
    assert_eq!(find_html_text_delimiter(plain_text), None);

    let with_tag = b"Hello world <div> is here";
    assert_eq!(find_html_text_delimiter(with_tag), Some(12));

    let with_entity = b"Price is 50 &amp; 100";
    assert_eq!(find_html_text_delimiter(with_entity), Some(12));
}

#[test]
fn test_simd_whitespace_and_delimiters() {
    let ws = b"   \t\n\r  hello";
    assert_eq!(skip_ascii_whitespace_simd(ws), 8);

    let tag = b"id=\"foo\" class='bar'> rest";
    assert_eq!(find_quote(tag), Some(3));
    assert_eq!(find_tag_close(tag), Some(20));
    assert_eq!(find_comment_dash(b"some -- comment"), Some(5));
    assert_eq!(find_unquoted_attr_end(b"foo=bar> baz"), Some(7));

    assert!(fast_ascii_eq_ignore_case(b"div", b"DIV"));
    assert!(fast_ascii_eq_ignore_case(b"SECTION", b"section"));
    assert!(!fast_ascii_eq_ignore_case(b"span", b"div"));
}
