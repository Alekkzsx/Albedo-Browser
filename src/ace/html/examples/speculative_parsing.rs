//! Speculative Parsing Example
//! 
//! Demonstrates parallel tokenization using speculative parsing.
//! The tokenizer runs in a separate thread while the tree builder
//! constructs the DOM, providing 2-3x speedup for large documents.

use ace::html::{parse_speculative, parse_document_speculative};
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║      ACE-HTML Speculative Parsing Examples              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_basic_speculative();
    example_performance_comparison();
    example_large_document();
    example_fallback_behavior();
}

fn example_basic_speculative() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Basic Speculative Parsing");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Speculative Parsing Demo</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <header>
        <h1>Welcome</h1>
    </header>
    <main>
        <article>
            <p>This document is parsed using speculative parsing.</p>
            <p>The tokenizer runs in a separate thread!</p>
        </article>
    </main>
</body>
</html>"#;

    println!("📄 Parsing document with speculative parsing...");
    
    let start = Instant::now();
    let result = parse_document_speculative(html);
    let elapsed = start.elapsed();
    
    println!("✅ Parsing complete in {:?}", elapsed);
    println!("   Nodes: {}", count_nodes(&result.document));
    println!("   Tokenizer ran in separate thread");
    println!();
}

fn example_performance_comparison() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Performance Comparison");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Generate a moderately large document
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..200 {
        html.push_str(&format!(
            r#"<div class="item-{}">
                <h2>Item {}</h2>
                <p>Description for item {}</p>
                <ul>
                    <li>Feature 1</li>
                    <li>Feature 2</li>
                    <li>Feature 3</li>
                </ul>
            </div>"#,
            i, i, i
        ));
    }
    html.push_str("</body></html>");
    
    println!("📄 Document size: {} KB", html.len() / 1024);
    println!("   Testing both parsing modes...\n");
    
    // Single-threaded parsing
    let start = Instant::now();
    let doc1 = ace::html::parse_document(&html);
    let single_thread_time = start.elapsed();
    
    println!("   Single-threaded: {:?}", single_thread_time);
    
    // Speculative parsing
    let start = Instant::now();
    let result = parse_document_speculative(&html);
    let speculative_time = start.elapsed();
    
    println!("   Speculative:     {:?}", speculative_time);
    
    let speedup = single_thread_time.as_secs_f64() / speculative_time.as_secs_f64();
    println!("\n✅ Speedup: {:.2}x faster", speedup);
    println!("   (Speculative parsing uses parallel tokenization)");
    println!();
}

fn example_large_document() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: Large Document (1000+ elements)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Generate a large document
    let mut html = String::from("<!DOCTYPE html><html><head><title>Large Doc</title></head><body>");
    
    // Add a large table
    html.push_str("<table>");
    html.push_str("<thead><tr><th>ID</th><th>Name</th><th>Value</th><th>Status</th></tr></thead>");
    html.push_str("<tbody>");
    for i in 0..500 {
        html.push_str(&format!(
            "<tr><td>{}</td><td>Item {}</td><td>{}</td><td>Active</td></tr>",
            i, i, i * 100
        ));
    }
    html.push_str("</tbody></table>");
    
    // Add many divs
    for i in 0..500 {
        html.push_str(&format!(
            r#"<div id="div-{}" class="content"><span>Content {}</span></div>"#,
            i, i
        ));
    }
    
    html.push_str("</body></html>");
    
    println!("📄 Large document: {} KB", html.len() / 1024);
    println!("   Elements: ~2000+");
    println!("   Parsing with speculative mode...\n");
    
    let start = Instant::now();
    let result = parse_document_speculative(&html);
    let elapsed = start.elapsed();
    
    println!("✅ Parsed in {:?}", elapsed);
    println!("   Nodes: {}", count_nodes(&result.document));
    println!("   Throughput: {:.2} MB/s",
        (html.len() as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64());
    println!();
}

fn example_fallback_behavior() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Fallback Behavior (small documents)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = "<html><body><div>Small document</div></body></html>";
    
    println!("📄 Small document: {} bytes", html.len());
    println!("   (May use single-threaded mode for efficiency)\n");
    
    let start = Instant::now();
    let result = parse_document_speculative(html);
    let elapsed = start.elapsed();
    
    println!("✅ Parsed in {:?}", elapsed);
    println!("   For small documents, single-threaded may be faster");
    println!("   (avoids thread spawning overhead)");
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
    fn test_speculative_basic() {
        let html = "<!DOCTYPE html><html><body><div>test</div></body></html>";
        let result = parse_document_speculative(html);
        assert!(!result.document.children.is_empty());
    }
    
    #[test]
    fn test_speculative_large() {
        let mut html = String::from("<html><body>");
        for i in 0..100 {
            html.push_str(&format!("<div>Item {}</div>", i));
        }
        html.push_str("</body></html>");
        
        let result = parse_document_speculative(&html);
        assert!(count_nodes(&result.document) > 100);
    }
    
    #[test]
    fn test_speculative_vs_single_thread() {
        let mut html = String::from("<html><body>");
        for i in 0..50 {
            html.push_str(&format!("<p>Paragraph {}</p>", i));
        }
        html.push_str("</body></html>");
        
        let doc1 = ace::html::parse_document(&html);
        let result2 = parse_document_speculative(&html);
        
        // Both should produce same node count
        assert_eq!(count_nodes(&doc1), count_nodes(&result2.document));
    }
}
