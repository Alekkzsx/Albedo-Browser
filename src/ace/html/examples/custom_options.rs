//! Custom Parser Options Example
//! 
//! Demonstrates how to configure the parser with custom options
//! for different use cases and requirements.

use ace::html::{
    parse_document_with_options, parse_fragment_with_context,
    ParserOptions, FragmentContext, Encoding,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Custom Options Examples                ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_scripting_disabled();
    example_base_url();
    example_encoding_hint();
    example_position_tracking();
    example_preload_collection();
}

fn example_scripting_disabled() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Scripting Disabled");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Scripting Test</title>
</head>
<body>
    <noscript>
        <p>JavaScript is disabled</p>
        <style>body { background: red; }</style>
    </noscript>
    <script>
        console.log("This won't affect parsing");
    </script>
</body>
</html>"#;

    // Parse with scripting disabled
    let mut options = ParserOptions::default();
    options.scripting_enabled = false;
    
    let doc = parse_document_with_options(html, &options);
    
    println!("✅ Parsed with scripting disabled");
    println!("   <noscript> content is parsed as regular HTML");
    println!("   (In browsers with JS disabled, noscript content is visible)");
    println!();
    
    // Compare with scripting enabled
    let mut options_enabled = ParserOptions::default();
    options_enabled.scripting_enabled = true;
    
    let doc_enabled = parse_document_with_options(html, &options_enabled);
    
    println!("   With scripting enabled:");
    println!("   <noscript> content is treated as raw text");
    println!();
}

fn example_base_url() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Base URL Resolution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <base href="https://example.com/path/">
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <a href="page.html">Link</a>
    <img src="image.png" alt="Image">
</body>
</html>"#;

    let mut options = ParserOptions::default();
    options.base_url = Some("https://example.com/".to_string());
    options.source_url = Some("https://example.com/index.html".to_string());
    
    let doc = parse_document_with_options(html, &options);
    
    println!("✅ Parsed with base URL configuration");
    println!("   Base URL: https://example.com/");
    println!("   Source URL: https://example.com/index.html");
    println!();
    println!("   Relative URLs will be resolved against base:");
    println!("   • styles.css → https://example.com/path/styles.css");
    println!("   • page.html → https://example.com/path/page.html");
    println!("   • image.png → https://example.com/path/image.png");
    println!();
}

fn example_encoding_hint() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: Encoding Hint");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Encoding Test</title>
</head>
<body>
    <p>Special characters: é, ñ, 中文, 日本語</p>
</body>
</html>"#;

    // Provide encoding hint
    let mut options = ParserOptions::default();
    options.encoding_hint = Some(Encoding::Utf8);
    
    let doc = parse_document_with_options(html, &options);
    
    println!("✅ Parsed with UTF-8 encoding hint");
    println!("   Encoding hint helps with faster detection");
    println!("   Parser will still respect <meta charset> if present");
    println!();
    
    // Other encoding examples
    println!("   Supported encodings:");
    println!("   • UTF-8 (default)");
    println!("   • UTF-16LE, UTF-16BE");
    println!("   • ISO-8859-1 (Latin-1)");
    println!("   • Windows-1252");
    println!("   • And many more...");
    println!();
}

fn example_position_tracking() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Position Tracking");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <div>
        <p>Line 5
        <span>Line 6</p>
    </div>
</body>
</html>"#;

    // Enable position tracking
    let mut options = ParserOptions::default();
    options.track_positions = true;
    
    let output = ace::html::parse_document_with_errors_and_options(html, &options);
    
    println!("✅ Parsed with position tracking enabled");
    println!("   Errors: {}\n", output.errors.len());
    
    if !output.errors.is_empty() {
        println!("   Error locations:");
        for (i, error) in output.errors.iter().take(3).enumerate() {
            if let (Some(line), Some(col)) = (error.line, error.column) {
                println!("   {}. Line {}, Column {}: {}",
                    i + 1, line, col, error.message);
            }
        }
    }
    
    println!("\n   Position tracking is useful for:");
    println!("   • Developer tools");
    println!("   • Error reporting");
    println!("   • Source maps");
    println!("   • Debugging");
    println!();
}

fn example_preload_collection() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: Preload Collection Control");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="styles.css">
    <script src="app.js"></script>
</head>
<body>
    <img src="image.png" alt="Image">
</body>
</html>"#;

    // Disable preload collection
    let mut options_no_preload = ParserOptions::default();
    options_no_preload.collect_preloads = false;
    
    let result1 = ace::html::parse_html_integrated_with_options(html, &options_no_preload);
    
    println!("✅ Parsed without preload collection");
    println!("   Preload requests: {}", result1.preload_requests.len());
    println!("   (Preload scanning disabled for faster parsing)");
    println!();
    
    // Enable preload collection
    let mut options_with_preload = ParserOptions::default();
    options_with_preload.collect_preloads = true;
    
    let result2 = ace::html::parse_html_integrated_with_options(html, &options_with_preload);
    
    println!("   Parsed with preload collection");
    println!("   Preload requests: {}", result2.preload_requests.len());
    println!("   (Resources discovered for early loading)");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scripting_disabled() {
        let html = "<noscript><p>test</p></noscript>";
        let mut options = ParserOptions::default();
        options.scripting_enabled = false;
        
        let doc = parse_document_with_options(html, &options);
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_base_url() {
        let html = r#"<a href="page.html">Link</a>"#;
        let mut options = ParserOptions::default();
        options.base_url = Some("https://example.com/".to_string());
        
        let doc = parse_document_with_options(html, &options);
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_encoding_hint() {
        let html = "<!DOCTYPE html><html><body>Test</body></html>";
        let mut options = ParserOptions::default();
        options.encoding_hint = Some(Encoding::Utf8);
        
        let doc = parse_document_with_options(html, &options);
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_position_tracking() {
        let html = "<div><p>test</div>";
        let mut options = ParserOptions::default();
        options.track_positions = true;
        
        let output = ace::html::parse_document_with_errors_and_options(html, &options);
        // Should track positions for errors
        if !output.errors.is_empty() {
            assert!(output.errors[0].line.is_some());
        }
    }
}
