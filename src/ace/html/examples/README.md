# ACE HTML Parser - Code Examples

This directory contains comprehensive examples demonstrating how to use the ACE HTML parser in various scenarios.

## Running Examples

To run any example:

```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example basic_usage
cargo run --example streaming_parsing
```

## Available Examples

### 1. Basic Usage (`basic_usage.rs`)

**What it demonstrates:**
- Simple document parsing
- Integrated parser with arena allocation and string interning
- Preload scanner integration
- Parser statistics and metrics

**Key features:**
- Arena allocator for efficient memory management
- String interner for deduplicating common strings
- Automatic preload resource discovery
- Performance metrics

**Run:**
```bash
cargo run --example basic_usage
```

---

### 2. Fragment Parsing (`fragment_parsing.rs`)

**What it demonstrates:**
- Parsing HTML fragments (like `innerHTML`)
- Different context elements (body, table, SVG)
- Fragment parsing with error handling

**Use cases:**
- Dynamic content insertion
- Template rendering
- Content management systems

**Run:**
```bash
cargo run --example fragment_parsing
```

---

### 3. Error Handling (`error_handling.rs`)

**What it demonstrates:**
- Parse error detection and recovery
- Handling malformed HTML
- Position tracking for errors
- WHATWG-compliant error recovery

**Key concepts:**
- Unclosed tags
- Mismatched tags
- Invalid nesting (Adoption Agency Algorithm)
- Character reference errors

**Run:**
```bash
cargo run --example error_handling
```

---

### 4. Streaming Parsing (`streaming_parsing.rs`)

**What it demonstrates:**
- Incremental parsing as data arrives
- Chunked parsing (16KB chunks)
- Network simulation with delays
- Progressive rendering patterns

**Use cases:**
- Network streaming
- Large document processing
- Low-latency parsing
- Progressive web apps

**Run:**
```bash
cargo run --example streaming_parsing
```

---

### 5. Speculative Parsing (`speculative_parsing.rs`)

**What it demonstrates:**
- Parallel tokenization in separate thread
- Performance comparison with single-threaded parsing
- Speedup measurements (2-3x for large documents)
- Fallback behavior for small documents

**Performance benefits:**
- 2-3x faster for documents > 100 KB
- Tokenizer and tree builder run in parallel
- Automatic thread pool management

**Run:**
```bash
cargo run --example speculative_parsing
```

---

### 6. Preload Scanning (`preload_scanning.rs`)

**What it demonstrates:**
- Early resource discovery (CSS, JS, images)
- Different resource types
- Responsive images (srcset)
- Integration with parser

**Use cases:**
- Browser preload optimization
- Resource prefetching
- Performance optimization
- Critical resource identification

**Run:**
```bash
cargo run --example preload_scanning
```

---

### 7. Custom Options (`custom_options.rs`)

**What it demonstrates:**
- Parser configuration options
- Scripting enabled/disabled
- Base URL resolution
- Encoding hints
- Position tracking
- Preload collection control

**Configuration options:**
- `scripting_enabled`: Affects `<noscript>` parsing
- `base_url`: For resolving relative URLs
- `encoding_hint`: Character encoding detection
- `track_positions`: Line/column tracking for errors
- `collect_preloads`: Enable/disable preload scanning

**Run:**
```bash
cargo run --example custom_options
```

---

### 8. DOM Traversal (`dom_traversal.rs`)

**What it demonstrates:**
- Tree traversal patterns
- Finding elements by tag name
- Extracting text content
- Accessing attributes
- Tree statistics

**Common patterns:**
- Recursive tree walking
- Element querying
- Text extraction
- Attribute access
- Tree analysis

**Run:**
```bash
cargo run --example dom_traversal
```

---

### 9. Performance Optimization (`performance_optimization.rs`)

**What it demonstrates:**
- Arena allocation benefits
- String interning efficiency
- SIMD detection and usage
- Speculative parsing speedup
- Memory efficiency

**Optimization techniques:**
- Arena allocator (< 10% overhead)
- String interning (30-50% memory savings)
- SIMD acceleration (10-20% speedup)
- Speculative parsing (2-3x speedup)

**Run:**
```bash
cargo run --example performance_optimization
```

---

### 10. Encoding Handling (`encoding_handling.rs`)

**What it demonstrates:**
- UTF-8 parsing (default)
- Automatic encoding detection
- BOM (Byte Order Mark) detection
- HTTP Content-Type header
- Meta charset detection
- Encoding hints

**Supported encodings:**
- UTF-8, UTF-16LE, UTF-16BE
- ISO-8859-1 (Latin-1)
- Windows-1252
- And many more...

**Run:**
```bash
cargo run --example encoding_handling
```

---

### 11. Integration Patterns (`integration_patterns.rs`)

**What it demonstrates:**
- Web scraper pattern
- Content extraction
- HTML sanitization
- Link checking
- SEO analysis

**Real-world use cases:**
- Web scraping and data extraction
- Content management systems
- Security (XSS prevention)
- Link validation
- SEO tools

**Run:**
```bash
cargo run --example integration_patterns
```

---

### 12. Browser Comparison (`browser_comparison_demo.rs`)

**What it demonstrates:**
- Side-by-side comparison with Chrome and Firefox
- Performance benchmarking
- Report generation

**Requirements:**
- Node.js installed
- Puppeteer (for Chrome) or Playwright (for Firefox)

**Setup:**
```bash
cd benchmarks
npm install puppeteer  # For Chrome
npm install playwright # For Firefox
```

**Run:**
```bash
cargo run --example browser_comparison_demo
```

---

### 13. Report Generation (`report_generation_demo.rs`)

**What it demonstrates:**
- Benchmark report generation
- HTML report output
- Performance metrics visualization

**Run:**
```bash
cargo run --example report_generation_demo
```

---

## Example Categories

### Getting Started
- `basic_usage.rs` - Start here!
- `fragment_parsing.rs` - Fragment parsing basics
- `dom_traversal.rs` - Working with the DOM tree

### Advanced Features
- `streaming_parsing.rs` - Incremental parsing
- `speculative_parsing.rs` - Parallel tokenization
- `preload_scanning.rs` - Resource discovery

### Configuration
- `custom_options.rs` - Parser configuration
- `encoding_handling.rs` - Character encoding

### Performance
- `performance_optimization.rs` - Optimization techniques
- `browser_comparison_demo.rs` - Browser benchmarking

### Real-World Applications
- `integration_patterns.rs` - Common integration patterns
- `error_handling.rs` - Error recovery

---

## Common Patterns

### Basic Document Parsing

```rust
use ace::html::parse_document;

let html = "<!DOCTYPE html><html><body>Hello</body></html>";
let doc = parse_document(html);
```

### Parsing with Options

```rust
use ace::html::{parse_document_with_options, ParserOptions};

let mut options = ParserOptions::default();
options.scripting_enabled = false;

let doc = parse_document_with_options(html, &options);
```

### Fragment Parsing

```rust
use ace::html::{parse_fragment, FragmentContext};

let html = "<div>Fragment</div>";
let context = FragmentContext::new("body");
let nodes = parse_fragment(html, Some("body"));
```

### Error Handling

```rust
use ace::html::parse_document_with_errors;

let html = "<div><p>Unclosed";
let output = parse_document_with_errors(html);

for error in &output.errors {
    println!("Error: {}", error.message);
}
```

### Streaming Parsing

```rust
use ace::html::StreamingHtmlParser;

let mut parser = StreamingHtmlParser::new();
parser.feed("<html><body>");
parser.feed("<div>Content</div>");
parser.feed("</body></html>");

let doc = parser.finish();
```

### Integrated Parsing (Recommended)

```rust
use ace::html::parse_html_integrated;

let html = "<!DOCTYPE html><html><body>Content</body></html>";
let result = parse_html_integrated(html);

println!("Nodes: {}", count_nodes(&result.document));
println!("Preloads: {}", result.preload_requests.len());
println!("Arena chunks: {}", result.stats.arena_chunk_count);
```

---

## Performance Tips

1. **Use integrated parser** for best performance (arena + interner + SIMD)
2. **Enable speculative parsing** for large documents (> 100 KB)
3. **Use streaming** for network data or very large documents
4. **Disable preload collection** if not needed (faster parsing)
5. **Provide encoding hint** for faster encoding detection

---

## Testing

All examples include unit tests. Run tests with:

```bash
cargo test --example <example_name>
```

Or run all example tests:

```bash
cargo test --examples
```

---

## Documentation

For detailed API documentation, see:

```bash
cargo doc --open
```

Or visit the main documentation at `src/ace/html/mod.rs`.

---

## Contributing

When adding new examples:

1. Create a new `.rs` file in this directory
2. Include comprehensive comments and documentation
3. Add multiple examples demonstrating different aspects
4. Include unit tests
5. Update this README with the new example
6. Follow the existing example structure

---

## License

See the main project LICENSE file.

---

## Questions?

For questions or issues:
- Check the main documentation
- Review existing examples
- Open an issue on the project repository

---

**Last updated:** 2024-01-06
