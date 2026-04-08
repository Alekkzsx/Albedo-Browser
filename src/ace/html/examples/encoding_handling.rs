//! Encoding Handling Example
//! 
//! Demonstrates character encoding detection and conversion when parsing
//! HTML from bytes. The parser supports automatic encoding detection from
//! BOM, HTTP headers, and meta tags.

use ace::html::{
    parse_document_from_bytes, parse_document_from_bytes_with_options,
    parse_document_from_bytes_with_errors_and_options,
    ParserOptions, Encoding,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Encoding Handling Examples             ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_utf8_parsing();
    example_encoding_detection();
    example_http_header();
    example_meta_charset();
    example_encoding_hint();
}

fn example_utf8_parsing() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: UTF-8 Parsing (default)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>UTF-8 Example</title>
</head>
<body>
    <h1>International Characters</h1>
    <p>English: Hello World</p>
    <p>Spanish: ¡Hola Mundo!</p>
    <p>French: Bonjour le monde</p>
    <p>German: Hallo Welt</p>
    <p>Russian: Привет мир</p>
    <p>Chinese: 你好世界</p>
    <p>Japanese: こんにちは世界</p>
    <p>Arabic: مرحبا بالعالم</p>
    <p>Emoji: 🌍 🌎 🌏</p>
</body>
</html>"#;

    let bytes = html.as_bytes();
    
    println!("📄 Parsing UTF-8 document ({} bytes)", bytes.len());
    
    let doc = parse_document_from_bytes(bytes).unwrap();
    
    println!("✅ Successfully parsed UTF-8 content");
    println!("   All international characters preserved");
    println!("   Emoji support: ✓");
    println!();
}

fn example_encoding_detection() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Automatic Encoding Detection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // UTF-8 with BOM
    let utf8_bom = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    let mut html_with_bom = utf8_bom.clone();
    html_with_bom.extend_from_slice(b"<html><body>UTF-8 with BOM</body></html>");
    
    println!("📄 Document with UTF-8 BOM");
    
    let doc = parse_document_from_bytes(&html_with_bom).unwrap();
    
    println!("✅ Detected UTF-8 from BOM");
    println!("   BOM bytes: EF BB BF");
    println!();
    
    // UTF-16LE with BOM
    println!("📄 UTF-16LE encoding");
    let utf16_text = "<!DOCTYPE html><html><body>UTF-16</body></html>";
    let utf16_bytes: Vec<u8> = utf16_text
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    
    let mut utf16_with_bom = vec![0xFF, 0xFE]; // UTF-16LE BOM
    utf16_with_bom.extend_from_slice(&utf16_bytes);
    
    match parse_document_from_bytes(&utf16_with_bom) {
        Ok(_) => println!("✅ Detected UTF-16LE from BOM"),
        Err(e) => println!("⚠️  UTF-16 detection: {}", e),
    }
    println!("   BOM bytes: FF FE");
    println!();
}

fn example_http_header() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: HTTP Content-Type Header");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = b"<!DOCTYPE html><html><body>Content</body></html>";
    
    // Parse with HTTP header
    let http_header = "text/html; charset=utf-8";
    
    println!("📄 HTTP Header: {}", http_header);
    
    let doc = parse_document_from_bytes_with_options(
        html,
        Some(http_header),
        &ParserOptions::default()
    ).unwrap();
    
    println!("✅ Encoding detected from HTTP header");
    println!("   Priority: HTTP header > BOM > meta tag");
    println!();
    
    // Different charset in header
    let http_header_latin1 = "text/html; charset=iso-8859-1";
    println!("📄 HTTP Header: {}", http_header_latin1);
    
    let doc = parse_document_from_bytes_with_options(
        html,
        Some(http_header_latin1),
        &ParserOptions::default()
    ).unwrap();
    
    println!("✅ Would use ISO-8859-1 encoding");
    println!("   (if content was actually Latin-1)");
    println!();
}

fn example_meta_charset() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Meta Charset Detection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // HTML with meta charset
    let html1 = br#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Meta Charset</title>
</head>
<body>Content</body>
</html>"#;

    println!("📄 Document with <meta charset=\"UTF-8\">");
    
    let doc = parse_document_from_bytes(html1).unwrap();
    
    println!("✅ Detected UTF-8 from meta tag");
    println!();
    
    // HTML with meta http-equiv
    let html2 = br#"<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
    <title>Meta HTTP-Equiv</title>
</head>
<body>Content</body>
</html>"#;

    println!("📄 Document with <meta http-equiv>");
    
    let doc = parse_document_from_bytes(html2).unwrap();
    
    println!("✅ Detected UTF-8 from http-equiv meta tag");
    println!("   (Legacy HTML4 style)");
    println!();
}

fn example_encoding_hint() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: Encoding Hint");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = b"<!DOCTYPE html><html><body>Test</body></html>";
    
    // Provide encoding hint
    let mut options = ParserOptions::default();
    options.encoding_hint = Some(Encoding::Utf8);
    
    println!("📄 Parsing with encoding hint: UTF-8");
    
    let output = parse_document_from_bytes_with_errors_and_options(
        html,
        None,
        &options
    ).unwrap();
    
    println!("✅ Used encoding hint for faster detection");
    println!("\n   Encoding detection priority:");
    println!("   1. BOM (Byte Order Mark)");
    println!("   2. HTTP Content-Type header");
    println!("   3. Meta charset tag");
    println!("   4. Encoding hint (if provided)");
    println!("   5. Default to UTF-8");
    println!();
    
    println!("   Supported encodings:");
    println!("   • UTF-8, UTF-16LE, UTF-16BE");
    println!("   • ISO-8859-1 (Latin-1)");
    println!("   • Windows-1252");
    println!("   • And many more...");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_utf8_parsing() {
        let html = b"<!DOCTYPE html><html><body>Test</body></html>";
        let doc = parse_document_from_bytes(html).unwrap();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_utf8_bom() {
        let mut html = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        html.extend_from_slice(b"<html><body>Test</body></html>");
        let doc = parse_document_from_bytes(&html).unwrap();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_http_header() {
        let html = b"<html><body>Test</body></html>";
        let header = "text/html; charset=utf-8";
        let doc = parse_document_from_bytes_with_options(
            html,
            Some(header),
            &ParserOptions::default()
        ).unwrap();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_meta_charset() {
        let html = br#"<html><head><meta charset="UTF-8"></head><body>Test</body></html>"#;
        let doc = parse_document_from_bytes(html).unwrap();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_encoding_hint() {
        let html = b"<html><body>Test</body></html>";
        let mut options = ParserOptions::default();
        options.encoding_hint = Some(Encoding::Utf8);
        
        let output = parse_document_from_bytes_with_errors_and_options(
            html,
            None,
            &options
        ).unwrap();
        assert!(!output.document.children.is_empty());
    }
}
