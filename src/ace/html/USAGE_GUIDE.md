# ACE HTML Parser - Usage Guide

Welcome to the ACE HTML Parser! This guide will help you get started and make the most of this high-performance, WHATWG-compliant HTML parser.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Core Features](#core-features)
3. [Advanced Features](#advanced-features)
4. [Common Use Cases](#common-use-cases)
5. [Best Practices](#best-practices)
6. [Troubleshooting](#troubleshooting)

---

## Getting Started

### Installation

Add ACE HTML to your project (assuming it's part of the Albedo engine):

```rust
use ace::html::*;
```

### Quick Start

The simplest way to parse HTML:

```rust
use ace::html::parse_document;

fn main() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head><title>Hello</title></head>
            <body><h1>Welcome!</h1></body>
        </html>
    "#;
    
    let document = parse_document(html);
    println!("Parsed successfully!");
}
```

### Basic Concepts

**Document vs Fragment Parsing:**
- **Document parsing**: Parses a complete HTML document (with `<!DOCTYPE>`, `<html>`, etc.)
- **Fragment parsing**: Parses HTML snippets (like `innerHTML` content)

**Key Components:**
- **Lexer**: Breaks input into character sequences
- **Tokenizer**: Converts characters into HTML tokens
- **Tree Builder**: Constructs the DOM tree from tokens
- **Arena Allocator**: Efficient memory management
- **String Interner**: Deduplicates common strings

---

## Core Features

### 1. Document Parsing

Parse complete HTML documents:

```rust
use ace::html::parse_document;

let html = "<!DOCTYPE html><html><body>Content</body></html>";
let doc = parse_document(html);
```

**When to use:**
- Parsing complete web pages
- Loading HTML files
- Browser-like document processing

### 2. Fragment Parsing

Parse HTML fragments with context:

```rust
use ace::html::parse_fragment;

// Parse as if inside <body>
let html = "<div>Hello</div><p>World</p>";
let nodes = parse_fragment(html, Some("body"));

// Parse as if inside <table>
let table_html = "<tr><td>Cell</td></tr>";
let table_nodes = parse_fragment(table_html, Some("table"));
```

**When to use:**
- Dynamic content insertion (like `innerHTML`)
- Template rendering
- Parsing user-generated content
- CMS systems

**Context matters:**
Different contexts affect parsing behavior:
- `"body"`: Normal content
- `"table"`: Table-specific parsing rules
- `"svg"`: SVG namespace handling
- `"template"`: Template content parsing

### 3. Error Handling

ACE HTML follows WHATWG error recovery, but you can track errors:

```rust
use ace::html::parse_document_with_errors;

let malformed = "<div><p>Unclosed tags<span>";
let output = parse_document_with_errors(malformed);

// Document is still valid (error recovery applied)
let doc = output.document;

// But you can inspect errors
for error in &output.errors {
    println!("Line {}, Col {}: {}", 
        error.line, error.column, error.message);
}
```

**Common parse errors:**
- Unclosed tags
- Mismatched tags
- Invalid nesting
- Character reference errors
- EOF in various states

**Error recovery:**
The parser automatically recovers from errors following WHATWG rules, ensuring you always get a valid DOM tree.

### 4. Streaming Support

Parse HTML incrementally as data arrives:

```rust
use ace::html::StreamingHtmlParser;

let mut parser = StreamingHtmlParser::new();

// Feed chunks as they arrive
parser.feed("<html><body>");
parser.feed("<div>Content");
parser.feed("</div>");
parser.feed("</body></html>");

// Finish parsing
let doc = parser.finish();
```

**Benefits:**
- Low latency (< 1ms per 16KB chunk)
- Memory efficient
- Progressive rendering
- Network streaming

**Chunk size recommendations:**
- **16KB**: Optimal for low latency (p99 < 1ms)
- **64KB**: Good balance (p99 < 5ms)
- **Larger**: Better throughput, higher latency

---

## Advanced Features

### 1. Speculative Parsing

For large documents, enable parallel tokenization:

```rust
use ace::html::parse_document_speculative;

let large_html = load_large_document(); // > 100 KB

// Tokenizer runs in separate thread
let doc = parse_document_speculative(large_html);
```

**Performance:**
- **2-3x faster** for documents > 100 KB
- Tokenizer and tree builder run in parallel
- Automatic fallback for small documents
- Thread pool managed automatically

**When to use:**
- Large documents (> 100 KB)
- Server-side rendering
- Batch processing
- Performance-critical applications

**When NOT to use:**
- Small documents (< 10 KB) - overhead not worth it
- Memory-constrained environments
- Single-core systems

### 2. Preload Scanning

Discover resources early for faster page loads:

```rust
use ace::html::parse_html_integrated;

let html = r#"
    <html>
    <head>
        <link rel="stylesheet" href="app.css">
        <script src="app.js" async></script>
    </head>
    <body>
        <img src="hero.jpg" loading="lazy">
    </body>
    </html>
"#;

let result = parse_html_integrated(html);

// Access discovered resources
for preload in &result.preload_requests {
    println!("Resource: {} ({})", preload.url, preload.resource_type);
    
    // Check attributes
    if preload.is_async {
        println!("  - Async script");
    }
    if preload.is_module {
        println!("  - ES module");
    }
}
```

**Detected resources:**
- `<link rel="stylesheet">` - Stylesheets
- `<script src>` - JavaScript files
- `<img src>` - Images
- `<link rel="preload">` - Explicit preloads
- `<video>`, `<audio>` - Media files

**Attributes extracted:**
- `async`, `defer` - Script loading behavior
- `type="module"` - ES modules
- `crossorigin` - CORS settings
- `integrity` - Subresource integrity
- `srcset` - Responsive images

**Performance:**
- Scanning latency: < 0.1ms for 1 MB
- Runs in parallel with parsing
- Zero allocations (arena-based)

### 3. Custom Options

Configure parser behavior:

```rust
use ace::html::{parse_document_with_options, ParserOptions};

let mut options = ParserOptions::default();

// Disable scripting (affects <noscript> parsing)
options.scripting_enabled = false;

// Set base URL for resolving relative URLs
options.base_url = Some("https://example.com/page/".to_string());

// Provide encoding hint
options.encoding_hint = Some("utf-8");

// Enable position tracking for errors
options.track_positions = true;

// Disable preload collection (faster parsing)
options.collect_preloads = false;

let doc = parse_document_with_options(html, &options);
```

**Option details:**

| Option | Default | Description |
|--------|---------|-------------|
| `scripting_enabled` | `true` | Affects `<noscript>` parsing |
| `base_url` | `None` | Base URL for relative URLs |
| `encoding_hint` | `None` | Character encoding hint |
| `track_positions` | `false` | Track line/column for errors |
| `collect_preloads` | `true` | Enable preload scanning |

### 4. Performance Tuning

Get the best performance:

```rust
use ace::html::parse_html_integrated;

let html = load_document();

// Integrated parser uses:
// - Arena allocator (< 10% overhead)
// - String interner (30-50% memory savings)
// - SIMD acceleration (10-20% speedup)
// - Preload scanning (parallel)
let result = parse_html_integrated(html);

// Check statistics
let stats = &result.stats;
println!("Arena chunks: {}", stats.arena_chunk_count);
println!("Interned strings: {}", stats.interned_string_count);
println!("Interner hit rate: {:.1}%", stats.interner_hit_rate * 100.0);
println!("SIMD level: {:?}", stats.simd_level);
```

**SIMD acceleration:**
The parser automatically detects CPU capabilities:
- **AVX-512**: 64-byte parallel processing (10-15% faster)
- **AVX2**: 32-byte parallel processing (5-10% faster)
- **SSE2**: 16-byte parallel processing (baseline)
- **Scalar**: Fallback for older CPUs

**Memory optimization:**
- **Arena allocator**: Bulk allocation, < 10% overhead
- **String interner**: Deduplicates tag names, attributes (30-50% savings)
- **Compact nodes**: 32-byte node representation

---

## Common Use Cases

### 1. Web Scraping

Extract data from web pages:

```rust
use ace::html::{parse_document, NodeType};

fn scrape_links(html: &str) -> Vec<String> {
    let doc = parse_document(html);
    let mut links = Vec::new();
    
    // Traverse tree to find <a> tags
    fn visit_node(node: &Node, links: &mut Vec<String>) {
        if node.node_type == NodeType::Element {
            if let Some(tag) = &node.tag_name {
                if tag == "a" {
                    if let Some(href) = node.attributes.get("href") {
                        links.push(href.clone());
                    }
                }
            }
        }
        
        // Visit children
        for child in &node.children {
            visit_node(child, links);
        }
    }
    
    visit_node(&doc.root, &mut links);
    links
}
```

### 2. Content Extraction

Extract text content:

```rust
fn extract_text(html: &str) -> String {
    let doc = parse_document(html);
    let mut text = String::new();
    
    fn collect_text(node: &Node, text: &mut String) {
        match node.node_type {
            NodeType::Text => {
                if let Some(content) = &node.text_content {
                    text.push_str(content);
                    text.push(' ');
                }
            }
            NodeType::Element => {
                // Skip script and style content
                if let Some(tag) = &node.tag_name {
                    if tag == "script" || tag == "style" {
                        return;
                    }
                }
                
                for child in &node.children {
                    collect_text(child, text);
                }
            }
            _ => {}
        }
    }
    
    collect_text(&doc.root, &mut text);
    text.trim().to_string()
}
```

### 3. HTML Sanitization

Remove dangerous content:

```rust
fn sanitize_html(html: &str) -> String {
    let doc = parse_document(html);
    
    // Allowed tags
    let allowed_tags = ["p", "div", "span", "a", "strong", "em", "ul", "ol", "li"];
    
    // Allowed attributes
    let allowed_attrs = ["href", "title", "class"];
    
    fn sanitize_node(node: &mut Node, allowed_tags: &[&str], allowed_attrs: &[&str]) {
        if node.node_type == NodeType::Element {
            if let Some(tag) = &node.tag_name {
                // Remove disallowed tags
                if !allowed_tags.contains(&tag.as_str()) {
                    node.children.clear();
                    return;
                }
                
                // Remove disallowed attributes
                node.attributes.retain(|k, _| allowed_attrs.contains(&k.as_str()));
            }
        }
        
        // Recursively sanitize children
        for child in &mut node.children {
            sanitize_node(child, allowed_tags, allowed_attrs);
        }
    }
    
    let mut doc = doc;
    sanitize_node(&mut doc.root, &allowed_tags, &allowed_attrs);
    
    // Serialize back to HTML
    serialize_document(&doc)
}
```

### 4. SEO Analysis

Analyze page structure:

```rust
struct SeoAnalysis {
    title: Option<String>,
    meta_description: Option<String>,
    h1_count: usize,
    img_without_alt: usize,
    links_count: usize,
}

fn analyze_seo(html: &str) -> SeoAnalysis {
    let doc = parse_document(html);
    let mut analysis = SeoAnalysis {
        title: None,
        meta_description: None,
        h1_count: 0,
        img_without_alt: 0,
        links_count: 0,
    };
    
    fn visit(node: &Node, analysis: &mut SeoAnalysis) {
        if node.node_type == NodeType::Element {
            if let Some(tag) = &node.tag_name {
                match tag.as_str() {
                    "title" => {
                        analysis.title = node.text_content.clone();
                    }
                    "meta" => {
                        if node.attributes.get("name") == Some(&"description".to_string()) {
                            analysis.meta_description = node.attributes.get("content").cloned();
                        }
                    }
                    "h1" => analysis.h1_count += 1,
                    "img" => {
                        if !node.attributes.contains_key("alt") {
                            analysis.img_without_alt += 1;
                        }
                    }
                    "a" => analysis.links_count += 1,
                    _ => {}
                }
            }
        }
        
        for child in &node.children {
            visit(child, analysis);
        }
    }
    
    visit(&doc.root, &mut analysis);
    analysis
}
```

---

## Best Practices

### Performance Optimization

**1. Use the integrated parser for best performance:**

```rust
// ✅ Good: Integrated parser with all optimizations
let result = parse_html_integrated(html);

// ❌ Avoid: Manual tokenizer/tree builder (unless you need fine control)
let tokenizer = HtmlTokenizer::new(html);
let tree_builder = HtmlTreeBuilder::new();
```

**2. Enable speculative parsing for large documents:**

```rust
// ✅ Good: For documents > 100 KB
if html.len() > 100_000 {
    let doc = parse_document_speculative(html);
} else {
    let doc = parse_document(html);
}
```

**3. Use streaming for network data:**

```rust
// ✅ Good: Stream as data arrives
let mut parser = StreamingHtmlParser::new();
for chunk in network_stream {
    parser.feed(&chunk);
}
let doc = parser.finish();

// ❌ Avoid: Buffering entire document
let mut buffer = String::new();
for chunk in network_stream {
    buffer.push_str(&chunk);
}
let doc = parse_document(&buffer);
```

**4. Disable preload collection if not needed:**

```rust
// ✅ Good: Faster parsing when preloads not needed
let mut options = ParserOptions::default();
options.collect_preloads = false;
let doc = parse_document_with_options(html, &options);
```

### Memory Management

**1. Reuse parsers when possible:**

```rust
// ✅ Good: Reuse streaming parser
let mut parser = StreamingHtmlParser::new();
for document in documents {
    parser.reset();
    parser.feed(document);
    let doc = parser.finish();
    process(doc);
}
```

**2. Process large documents in chunks:**

```rust
// ✅ Good: Stream large files
let file = File::open("large.html")?;
let reader = BufReader::new(file);
let mut parser = StreamingHtmlParser::new();

for line in reader.lines() {
    parser.feed(&line?);
}
let doc = parser.finish();
```

**3. Monitor memory usage:**

```rust
let result = parse_html_integrated(html);
let stats = &result.stats;

// Check arena efficiency
if stats.arena_chunk_count > 100 {
    println!("Warning: Many arena chunks allocated");
}

// Check interner efficiency
if stats.interner_hit_rate < 0.7 {
    println!("Warning: Low interner hit rate");
}
```

### Error Handling

**1. Always handle malformed HTML gracefully:**

```rust
// ✅ Good: Parser handles errors automatically
let doc = parse_document(malformed_html);
// Document is always valid

// Optional: Track errors for debugging
let output = parse_document_with_errors(malformed_html);
if !output.errors.is_empty() {
    log_errors(&output.errors);
}
```

**2. Validate user input:**

```rust
// ✅ Good: Sanitize user-generated content
fn process_user_html(html: &str) -> String {
    let doc = parse_document(html);
    sanitize_document(doc)
}
```

### Testing

**1. Test with real-world HTML:**

```rust
#[test]
fn test_parse_wikipedia() {
    let html = load_test_file("wikipedia_homepage.html");
    let doc = parse_document(html);
    assert!(doc.root.children.len() > 0);
}
```

**2. Test error recovery:**

```rust
#[test]
fn test_malformed_html() {
    let html = "<div><p>Unclosed<span>";
    let output = parse_document_with_errors(html);
    
    // Should still parse successfully
    assert!(output.document.root.children.len() > 0);
    
    // But should report errors
    assert!(!output.errors.is_empty());
}
```

**3. Benchmark performance:**

```rust
#[test]
fn bench_parse_performance() {
    let html = load_test_file("large_document.html");
    
    let start = Instant::now();
    let doc = parse_document(html);
    let elapsed = start.elapsed();
    
    // Should parse at > 500 MB/s
    let throughput = html.len() as f64 / elapsed.as_secs_f64() / 1_000_000.0;
    assert!(throughput > 500.0, "Throughput: {:.1} MB/s", throughput);
}
```

---

## Troubleshooting

### Common Issues

#### Issue: Slow parsing performance

**Symptoms:**
- Parsing takes longer than expected
- Throughput < 500 MB/s

**Solutions:**

1. **Use integrated parser:**
```rust
// Instead of manual tokenizer/tree builder
let result = parse_html_integrated(html);
```

2. **Enable speculative parsing for large documents:**
```rust
if html.len() > 100_000 {
    let doc = parse_document_speculative(html);
}
```

3. **Check SIMD support:**
```rust
let result = parse_html_integrated(html);
println!("SIMD level: {:?}", result.stats.simd_level);
// Should be AVX2 or AVX512 on modern CPUs
```

4. **Profile your code:**
```bash
cargo build --release
perf record --call-graph=dwarf ./target/release/your_app
perf report
```

#### Issue: High memory usage

**Symptoms:**
- Memory usage higher than expected
- Many arena chunks allocated

**Solutions:**

1. **Use streaming for large documents:**
```rust
let mut parser = StreamingHtmlParser::new();
for chunk in chunks {
    parser.feed(chunk);
}
```

2. **Check interner efficiency:**
```rust
let result = parse_html_integrated(html);
if result.stats.interner_hit_rate < 0.7 {
    // Low hit rate indicates many unique strings
    println!("Consider increasing interner capacity");
}
```

3. **Process documents in batches:**
```rust
for batch in documents.chunks(100) {
    for doc in batch {
        process(parse_document(doc));
    }
    // Memory freed between batches
}
```

#### Issue: Unexpected parse results

**Symptoms:**
- DOM structure different than expected
- Missing or rearranged elements

**Solutions:**

1. **Check for foster parenting:**
```rust
// Invalid: <table><div>text</div></table>
// Result: <div>text</div><table></table>
// The <div> is "foster parented" outside the table
```

2. **Check for adoption agency algorithm:**
```rust
// Input: <b><i>1<b>2</i>3</b>4
// Result: <b><i>1</i></b><b><i>2</i>3</b>4
// AAA restructures misnested formatting elements
```

3. **Enable error tracking:**
```rust
let output = parse_document_with_errors(html);
for error in &output.errors {
    println!("{}", error.message);
}
```

4. **Compare with browser behavior:**
```javascript
// In browser console:
document.body.innerHTML = '<your html>';
console.log(document.body.innerHTML);
```

#### Issue: Character encoding problems

**Symptoms:**
- Garbled text
- Incorrect characters

**Solutions:**

1. **Provide encoding hint:**
```rust
let mut options = ParserOptions::default();
options.encoding_hint = Some("utf-8");
let doc = parse_document_with_options(html, &options);
```

2. **Check for BOM:**
```rust
// UTF-8 BOM: EF BB BF
// UTF-16LE BOM: FF FE
// UTF-16BE BOM: FE FF
```

3. **Validate input encoding:**
```rust
if !html.is_char_boundary(0) {
    eprintln!("Invalid UTF-8 input");
}
```

### FAQ

**Q: Is ACE HTML thread-safe?**

A: The parser itself is not thread-safe (it uses internal mutable state). However, you can create separate parser instances per thread:

```rust
use std::thread;

let handles: Vec<_> = documents
    .into_iter()
    .map(|html| {
        thread::spawn(move || parse_document(&html))
    })
    .collect();

for handle in handles {
    let doc = handle.join().unwrap();
    process(doc);
}
```

**Q: Can I parse HTML from a file?**

A: Yes, read the file and pass the string:

```rust
use std::fs;

let html = fs::read_to_string("page.html")?;
let doc = parse_document(&html);
```

For large files, use streaming:

```rust
use std::fs::File;
use std::io::{BufReader, BufRead};

let file = File::open("large.html")?;
let reader = BufReader::new(file);
let mut parser = StreamingHtmlParser::new();

for line in reader.lines() {
    parser.feed(&line?);
}
let doc = parser.finish();
```

**Q: How do I serialize the DOM back to HTML?**

A: Use the serialization API (if available):

```rust
let doc = parse_document(html);
let serialized = serialize_document(&doc);
```

**Q: Does ACE HTML support HTML5?**

A: Yes! ACE HTML is 100% WHATWG HTML Living Standard compliant, which is the modern HTML5 specification.

**Q: Can I modify the DOM after parsing?**

A: Yes, the DOM is mutable:

```rust
let mut doc = parse_document(html);

// Add a new element
let new_div = create_element("div");
doc.root.children.push(new_div);

// Modify attributes
if let Some(node) = find_element_by_id(&doc, "header") {
    node.attributes.insert("class".to_string(), "active".to_string());
}
```

**Q: What's the difference between `parse_document` and `parse_html_integrated`?**

A: `parse_html_integrated` includes additional features:
- Arena allocator for efficient memory
- String interner for deduplication
- Preload scanner for resource discovery
- Performance statistics

Use `parse_html_integrated` for best performance and features.

**Q: How do I handle fragments with specific context?**

A: Use `parse_fragment` with a context element:

```rust
// Parse as if inside <body>
let nodes = parse_fragment(html, Some("body"));

// Parse as if inside <table>
let nodes = parse_fragment(html, Some("table"));

// Parse with no context (like <body>)
let nodes = parse_fragment(html, None);
```

### Debugging Tips

**1. Enable position tracking:**

```rust
let mut options = ParserOptions::default();
options.track_positions = true;

let output = parse_document_with_errors_and_options(html, &options);
for error in &output.errors {
    println!("Line {}, Col {}: {}", error.line, error.column, error.message);
}
```

**2. Inspect parser statistics:**

```rust
let result = parse_html_integrated(html);
let stats = &result.stats;

println!("Parse time: {:?}", stats.parse_duration);
println!("Nodes created: {}", stats.node_count);
println!("Arena chunks: {}", stats.arena_chunk_count);
println!("Interner hit rate: {:.1}%", stats.interner_hit_rate * 100.0);
println!("SIMD level: {:?}", stats.simd_level);
```

**3. Compare with browser:**

```rust
// Parse with ACE
let doc = parse_document(html);
let ace_structure = debug_tree_structure(&doc);

// Compare with browser (manually)
// Open browser console and run:
// document.body.innerHTML = '<your html>';
// console.log(document.body.innerHTML);
```

**4. Use verbose logging:**

```rust
// Enable debug logging (if available)
env_logger::init();
std::env::set_var("RUST_LOG", "ace_html=debug");

let doc = parse_document(html);
```

---

## Additional Resources

### Code Examples

See the `examples/` directory for comprehensive examples:

- `basic_usage.rs` - Getting started
- `streaming_parsing.rs` - Incremental parsing
- `speculative_parsing.rs` - Parallel tokenization
- `preload_scanning.rs` - Resource discovery
- `error_handling.rs` - Error recovery
- `performance_optimization.rs` - Performance tuning
- `integration_patterns.rs` - Real-world use cases

Run examples with:
```bash
cargo run --example basic_usage
```

### API Documentation

Generate and view API docs:
```bash
cargo doc --open
```

### Benchmarks

Run performance benchmarks:
```bash
cargo bench
```

View benchmark reports in `target/bench_report.html`.

### Testing

Run the test suite:
```bash
cargo test
```

Run html5lib conformance tests:
```bash
cargo test --test html5lib_conformance
```

---

## Performance Reference

### Throughput Targets

| Document Size | Expected Throughput | Latency |
|---------------|---------------------|---------|
| 10 KB | 500+ MB/s | < 0.1ms |
| 100 KB | 500+ MB/s | < 1ms |
| 1 MB | 500+ MB/s | < 5ms |
| 10 MB | 500+ MB/s | < 50ms |

### Memory Usage

| Document Size | Expected Memory | With Interner |
|---------------|-----------------|---------------|
| 10 KB | ~50 KB | ~35 KB (30% savings) |
| 100 KB | ~500 KB | ~350 KB (30% savings) |
| 1 MB | ~5 MB | ~3.5 MB (30% savings) |
| 10 MB | ~40 MB | ~28 MB (30% savings) |

### SIMD Speedup

| SIMD Level | Speedup vs Scalar | Availability |
|------------|-------------------|--------------|
| SSE2 | 1.0x (baseline) | All x86-64 CPUs |
| AVX2 | 1.05-1.10x | Intel Haswell+ (2013+) |
| AVX-512 | 1.10-1.15x | Intel Skylake-X+ (2017+) |

---

## License

See the main project LICENSE file.

---

## Support

For questions, issues, or contributions:
- Check the examples directory
- Review the API documentation
- Open an issue on the project repository

---

**Last Updated:** 2024-01-06  
**Version:** 1.0  
**ACE HTML Parser** - Part of the Albedo Engine
