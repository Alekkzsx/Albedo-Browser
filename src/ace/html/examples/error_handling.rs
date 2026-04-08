//! Error Handling Example
//! 
//! Demonstrates how to handle parse errors and malformed HTML.
//! The parser follows WHATWG error recovery rules to produce valid DOM
//! even from invalid HTML.

use ace::html::{
    parse_document_with_errors, parse_document_with_errors_and_options,
    ParserOptions, AceHtmlErrorCode,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Error Handling Examples                ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_unclosed_tags();
    example_mismatched_tags();
    example_invalid_nesting();
    example_character_references();
    example_error_recovery();
}

fn example_unclosed_tags() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Unclosed Tags");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Unclosed Tags
<body>
    <div>
        <p>Paragraph without closing tag
        <p>Another paragraph
    </div>
</body>
</html>"#;

    let output = parse_document_with_errors(html);
    
    println!("✅ Document parsed successfully (with error recovery)");
    println!("   Nodes: {}", count_nodes(&output.document));
    println!("   Errors: {}", output.errors.len());
    
    if !output.errors.is_empty() {
        println!("\n   Parse errors:");
        for (i, error) in output.errors.iter().enumerate() {
            println!("   {}. {} (line {}, col {})",
                i + 1,
                error.message,
                error.line.unwrap_or(0),
                error.column.unwrap_or(0)
            );
        }
    }
    println!();
}

fn example_mismatched_tags() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Mismatched Tags");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<div>
    <span>Text</div>
</span>"#;

    let output = parse_document_with_errors(html);
    
    println!("✅ Recovered from mismatched tags");
    println!("   Errors: {}", output.errors.len());
    
    for error in &output.errors {
        println!("   • {}", error.message);
    }
    println!();
}

fn example_invalid_nesting() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: Invalid Nesting (Adoption Agency Algorithm)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // This triggers the Adoption Agency Algorithm
    let html = r#"<b><i>Bold and italic<b>More bold</i>End</b>"#;

    let output = parse_document_with_errors(html);
    
    println!("✅ Applied Adoption Agency Algorithm");
    println!("   Input: {}", html);
    println!("   Errors: {}", output.errors.len());
    println!("   (DOM structure corrected automatically)");
    println!();
}

fn example_character_references() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Invalid Character References");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<p>Invalid: &unknown; &nbsp &lt</p>"#;

    let output = parse_document_with_errors(html);
    
    println!("✅ Handled invalid character references");
    println!("   Errors: {}", output.errors.len());
    
    for error in &output.errors {
        if error.message.contains("character reference") {
            println!("   • {}", error.message);
        }
    }
    println!();
}

fn example_error_recovery() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: Complex Error Recovery");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
<body>
    <table>
        <div>Invalid div in table</div>
        <tr>
            <td>Cell 1</td>
            <td>Cell 2
        </tr>
    </table>
    <p>Text outside table
</body>"#;

    let mut options = ParserOptions::default();
    options.track_positions = true;
    
    let output = parse_document_with_errors_and_options(html, &options);
    
    println!("✅ Complex error recovery applied");
    println!("   Total errors: {}", output.errors.len());
    println!("   Document still valid: {}", !output.document.children.is_empty());
    
    println!("\n   Error summary:");
    let mut error_types = std::collections::HashMap::new();
    for error in &output.errors {
        *error_types.entry(&error.message).or_insert(0) += 1;
    }
    
    for (msg, count) in error_types.iter().take(5) {
        println!("   • {} ({}x)", msg, count);
    }
    
    println!("\n   ℹ️  Despite {} errors, the document was successfully parsed", output.errors.len());
    println!("      and a valid DOM tree was constructed.");
    println!();
}

fn count_nodes(doc: &ace::html::HtmlDocument) -> usize {
    fn count_children(children: &[ace::html::HtmlNode]) -> usize {
        children.iter().map(|node| {
            match node {
                ace::html::HtmlNode::Element(el) => 1 + count_children(&el.children),
                _ => 1,
            }
        }).sum()
    }
    count_children(&doc.children)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unclosed_tags_recovery() {
        let html = "<div><p>text";
        let output = parse_document_with_errors(html);
        assert!(!output.document.children.is_empty());
    }
    
    #[test]
    fn test_mismatched_tags() {
        let html = "<div><span>text</div></span>";
        let output = parse_document_with_errors(html);
        assert!(output.errors.len() > 0);
    }
    
    #[test]
    fn test_invalid_character_reference() {
        let html = "&unknown;";
        let output = parse_document_with_errors(html);
        // Should have error but still parse
        assert!(!output.document.children.is_empty());
    }
}
