use ace_core::string::AceString;

#[test]
fn test_sso_creation() {
    let s = AceString::from_str("Hello");
    assert_eq!(s.len(), 5);
    if let AceString::Inline { len, data } = s {
        assert_eq!(len, 5);
        assert_eq!(&data[..5], b"Hello");
    } else {
        panic!("String curta deveria usar SSO (Inline)");
    }
    assert_eq!(s.to_string(), "Hello");
}

#[test]
fn test_latin1_creation() {
    // Uma string repetida longa para quebrar o limite do SSO (23 bytes)
    let raw = "Hello World! This is a long pure ASCII string.";
    let s = AceString::from_str(raw);
    
    if let AceString::Latin1(vec) = &s {
        assert_eq!(vec.len(), raw.len());
    } else {
        panic!("String longa ASCII deveria ser armazenada como Latin1 no Heap");
    }
    assert_eq!(s.to_string(), raw);
}

#[test]
fn test_utf16_creation() {
    let raw = "Hello 🌍 World!";
    let s = AceString::from_str(raw);
    
    if let AceString::Utf16(vec) = &s {
        // "Hello " (6) + "🌍" (2 u16 surrogates) + " World!" (7) = 15 code units
        assert_eq!(vec.len(), 15);
    } else {
        panic!("String com Emojis deveria ser promovida a UTF16 no Heap");
    }
    assert_eq!(s.to_string(), raw);
}
