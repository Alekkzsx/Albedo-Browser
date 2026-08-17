use ace_core::error::{AceError, SourceLocation};

#[test]
fn test_error_formatting_and_location() {
    let loc = SourceLocation::with_url("https://example.com/index.html", 42, 15, 1024);
    assert_eq!(format!("{}", loc), "https://example.com/index.html:42:15 (byte 1024)");

    let err = AceError::parse("Tag '<unknown>' inválida", loc.clone(), true);
    assert!(err.is_recoverable());
    assert_eq!(err.category(), "parse");
    assert_eq!(err.source_location(), Some(&loc));

    let msg = format!("{}", err);
    assert!(msg.contains("Tag '<unknown>' inválida"));
    assert!(msg.contains("https://example.com/index.html:42:15"));

    let net_err = AceError::network("https://api.example.com/data", "Connection reset", Some(503));
    assert_eq!(net_err.category(), "network");
    assert!(!net_err.is_recoverable());
    assert_eq!(net_err.source_location(), None);

    let net_msg = format!("{}", net_err);
    assert!(net_msg.contains("https://api.example.com/data"));
    assert!(net_msg.contains("Connection reset"));
}
