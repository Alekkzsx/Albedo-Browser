use ace_core::net::parse_data_url;

#[test]
fn test_parse_simple_text_data_url() {
    let url = "data:text/plain;charset=utf-8,Hello%20Albedo%20Browser!";
    let record = parse_data_url(url).unwrap();

    assert_eq!(record.mime_type.essence(), "text/plain");
    assert_eq!(record.mime_type.get_param("charset"), Some("utf-8"));
    assert_eq!(String::from_utf8(record.body).unwrap(), "Hello Albedo Browser!");
    assert!(!record.is_base64);
}

#[test]
fn test_parse_base64_png_data_url() {
    // 1x1 pixel PNG transparente
    let url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let record = parse_data_url(url).unwrap();

    assert_eq!(record.mime_type.essence(), "image/png");
    assert!(record.is_base64);
    assert!(!record.body.is_empty());
    // Verifica magic bytes do PNG: 0x89 'P' 'N' 'G'
    assert_eq!(&record.body[0..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn test_parse_data_url_default_mime() {
    let url = "data:,HelloWorld";
    let record = parse_data_url(url).unwrap();

    assert_eq!(record.mime_type.essence(), "text/plain");
    assert_eq!(record.mime_type.get_param("charset"), Some("US-ASCII"));
    assert_eq!(String::from_utf8(record.body).unwrap(), "HelloWorld");
}

#[test]
fn test_parse_invalid_data_url() {
    assert!(parse_data_url("http://example.com").is_err());
    assert!(parse_data_url("data:no-comma-here").is_err());
}
