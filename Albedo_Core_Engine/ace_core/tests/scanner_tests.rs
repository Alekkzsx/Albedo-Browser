use ace_core::scanner::find_byte_fast;

#[test]
fn test_scanner_basic() {
    let html = b"<html><head></head><body><h1>Hello</h1></body></html>";
    
    // Busca o primeiro '<'
    assert_eq!(find_byte_fast(html, b'<'), Some(0));
    
    // Busca o primeiro '>'
    assert_eq!(find_byte_fast(html, b'>'), Some(5));
    
    // Busca um caractere que não existe
    assert_eq!(find_byte_fast(html, b'Z'), None);
}

#[test]
fn test_scanner_large_buffer() {
    // 100.000 caracteres 'a' e 1 caractere '<' no final
    let mut large_buffer = vec![b'a'; 100_000];
    large_buffer[99_999] = b'<';
    
    assert_eq!(find_byte_fast(&large_buffer, b'<'), Some(99_999));
}

#[test]
fn test_scanner_unaligned() {
    let data = b"1234567890<";
    assert_eq!(find_byte_fast(data, b'<'), Some(10));
    
    let data2 = b"123<5678";
    assert_eq!(find_byte_fast(data2, b'<'), Some(3));
}
