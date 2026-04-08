//! Micro benchmarks for HTML parser components
//! 
//! Tests individual components: Lexer, Tokenizer, Tree Builder, SIMD operations

#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use crate::ace::html::{
    HtmlTokenizer, build_document,
    bench::{BenchRunner, BenchConfig},
};

// ============================================================================
// LEXER BENCHMARKS (10 casos)
// ============================================================================

/// Benchmark 1: Simple tag parsing
#[test]
fn bench_lexer_simple_tag() {
    let html = "<div></div>".repeat(100);
    
    let config = BenchConfig::new("Lexer: Simple Tags")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Simple Tags ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 2: Attributes parsing
#[test]
fn bench_lexer_attributes() {
    let html = r#"<div class="test" id="main" data-value="123"></div>"#.repeat(100);
    
    let config = BenchConfig::new("Lexer: Attributes")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Attributes ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(20));
}

/// Benchmark 3: Text content
#[test]
fn bench_lexer_text_content() {
    let html = "<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>".repeat(100);
    
    let config = BenchConfig::new("Lexer: Text Content")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Text Content ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(15));
}

/// Benchmark 4: Comments
#[test]
fn bench_lexer_comments() {
    let html = "<!-- This is a comment -->".repeat(100);
    
    let config = BenchConfig::new("Lexer: Comments")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Comments ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 5: Character references
#[test]
fn bench_lexer_entities() {
    let html = "<p>&lt;&gt;&amp;&quot;&apos;</p>".repeat(100);
    
    let config = BenchConfig::new("Lexer: Character References")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Character References ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(15));
}

/// Benchmark 6: DOCTYPE
#[test]
fn bench_lexer_doctype() {
    let html = "<!DOCTYPE html>".repeat(100);
    
    let config = BenchConfig::new("Lexer: DOCTYPE")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: DOCTYPE ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(5));
}

/// Benchmark 7: Script tags
#[test]
fn bench_lexer_script() {
    let html = "<script>var x = 1; console.log(x);</script>".repeat(100);
    
    let config = BenchConfig::new("Lexer: Script Tags")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Script Tags ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(15));
}

/// Benchmark 8: Self-closing tags
#[test]
fn bench_lexer_self_closing() {
    let html = "<img src='test.jpg' /><br /><hr />".repeat(100);
    
    let config = BenchConfig::new("Lexer: Self-closing Tags")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Self-closing Tags ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 9: Nested tags
#[test]
fn bench_lexer_nested() {
    let html = "<div><span><a><b><i>text</i></b></a></span></div>".repeat(100);
    
    let config = BenchConfig::new("Lexer: Nested Tags")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Nested Tags ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(15));
}

/// Benchmark 10: Mixed content
#[test]
fn bench_lexer_mixed() {
    let html = r#"
        <!DOCTYPE html>
        <div class="container">
            <p>Text with &lt;entities&gt;</p>
            <!-- Comment -->
            <script>var x = 1;</script>
            <img src="test.jpg" />
        </div>
    "#.repeat(50);
    
    let config = BenchConfig::new("Lexer: Mixed Content")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Lexer Benchmark: Mixed Content ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(25));
}

// ============================================================================
// TOKENIZER BENCHMARKS (10 casos)
// ============================================================================

/// Benchmark 1: Simple document
#[test]
fn bench_tokenizer_simple() {
    let html = "<html><body><p>Hello</p></body></html>";
    
    let config = BenchConfig::new("Tokenizer: Simple Document")
        .with_warmup(10)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(html);
        let mut count = 0;
        while tokenizer.next_token().is_some() {
            count += 1;
        }
        assert!(count > 0);
    });
    
    println!("\n=== Tokenizer Benchmark: Simple Document ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_micros(500));
}

/// Benchmark 2: Table structure
#[test]
fn bench_tokenizer_table() {
    let html = r#"
        <table>
            <tr><td>Cell 1</td><td>Cell 2</td></tr>
            <tr><td>Cell 3</td><td>Cell 4</td></tr>
        </table>
    "#.repeat(10);
    
    let config = BenchConfig::new("Tokenizer: Table Structure")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: Table Structure ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 3: List structure
#[test]
fn bench_tokenizer_list() {
    let html = "<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>".repeat(20);
    
    let config = BenchConfig::new("Tokenizer: List Structure")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: List Structure ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 4: Form elements
#[test]
fn bench_tokenizer_forms() {
    let html = r#"
        <form>
            <input type="text" name="username" />
            <input type="password" name="password" />
            <button type="submit">Submit</button>
        </form>
    "#.repeat(20);
    
    let config = BenchConfig::new("Tokenizer: Form Elements")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: Form Elements ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(15));
}

/// Benchmark 5: Whitespace handling
#[test]
fn bench_tokenizer_whitespace() {
    let html = "  <div>  \n  <p>  Text  </p>  \n  </div>  ".repeat(50);
    
    let config = BenchConfig::new("Tokenizer: Whitespace")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: Whitespace ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 6: Error recovery
#[test]
fn bench_tokenizer_errors() {
    let html = "<div><p>Unclosed<div>Nested</p></div>".repeat(50);
    
    let config = BenchConfig::new("Tokenizer: Error Recovery")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: Error Recovery ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 7: SVG content
#[test]
fn bench_tokenizer_svg() {
    let html = r#"<svg><circle cx="50" cy="50" r="40" /></svg>"#.repeat(50);
    
    let config = BenchConfig::new("Tokenizer: SVG Content")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: SVG Content ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 8: Template tags
#[test]
fn bench_tokenizer_template() {
    let html = "<template><div>Template content</div></template>".repeat(50);
    
    let config = BenchConfig::new("Tokenizer: Template Tags")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: Template Tags ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 9: CDATA sections
#[test]
fn bench_tokenizer_cdata() {
    let html = "<![CDATA[Some data content]]>".repeat(100);
    
    let config = BenchConfig::new("Tokenizer: CDATA Sections")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(&html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: CDATA Sections ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 10: Complex document
#[test]
fn bench_tokenizer_complex() {
    let html = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Test</title>
        </head>
        <body>
            <header><h1>Title</h1></header>
            <main>
                <article>
                    <p>Content with &lt;entities&gt;</p>
                    <ul><li>Item 1</li><li>Item 2</li></ul>
                </article>
            </main>
            <footer><!-- Comment --></footer>
        </body>
        </html>
    "#;
    
    let config = BenchConfig::new("Tokenizer: Complex Document")
        .with_warmup(10)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let mut tokenizer = HtmlTokenizer::new(html);
        while tokenizer.next_token().is_some() {}
    });
    
    println!("\n=== Tokenizer Benchmark: Complex Document ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(5));
}

// ============================================================================
// TREE BUILDER BENCHMARKS (10 casos)
// ============================================================================

/// Benchmark 1: Simple tree
#[test]
fn bench_tree_builder_simple() {
    let html = "<html><body><p>Hello</p></body></html>";
    
    let config = BenchConfig::new("Tree Builder: Simple Tree")
        .with_warmup(10)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(html);
    });
    
    println!("\n=== Tree Builder Benchmark: Simple Tree ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(1));
}

/// Benchmark 2: Deep nesting
#[test]
fn bench_tree_builder_deep() {
    let mut html = String::new();
    for _ in 0..50 {
        html.push_str("<div>");
    }
    html.push_str("Content");
    for _ in 0..50 {
        html.push_str("</div>");
    }
    
    let config = BenchConfig::new("Tree Builder: Deep Nesting")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: Deep Nesting ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(5));
}

/// Benchmark 3: Wide tree
#[test]
fn bench_tree_builder_wide() {
    let mut html = String::from("<div>");
    for i in 0..100 {
        html.push_str(&format!("<span>Item {}</span>", i));
    }
    html.push_str("</div>");
    
    let config = BenchConfig::new("Tree Builder: Wide Tree")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: Wide Tree ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(5));
}

/// Benchmark 4: Table construction
#[test]
fn bench_tree_builder_table() {
    let mut html = String::from("<table>");
    for i in 0..20 {
        html.push_str("<tr>");
        for j in 0..5 {
            html.push_str(&format!("<td>Cell {},{}</td>", i, j));
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");
    
    let config = BenchConfig::new("Tree Builder: Table")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: Table ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 5: List construction
#[test]
fn bench_tree_builder_list() {
    let mut html = String::from("<ul>");
    for i in 0..100 {
        html.push_str(&format!("<li>Item {}</li>", i));
    }
    html.push_str("</ul>");
    
    let config = BenchConfig::new("Tree Builder: List")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: List ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(5));
}

/// Benchmark 6: Adoption agency algorithm
#[test]
fn bench_tree_builder_aaa() {
    let html = "<b><i><b></b></i></b>".repeat(50);
    
    let config = BenchConfig::new("Tree Builder: AAA")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: AAA ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 7: Foster parenting
#[test]
fn bench_tree_builder_foster() {
    let html = "<table><div>Text</div></table>".repeat(50);
    
    let config = BenchConfig::new("Tree Builder: Foster Parenting")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: Foster Parenting ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 8: Template elements
#[test]
fn bench_tree_builder_template() {
    let html = "<template><div>Content</div></template>".repeat(50);
    
    let config = BenchConfig::new("Tree Builder: Template")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: Template ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 9: Foreign content (SVG)
#[test]
fn bench_tree_builder_svg() {
    let html = r#"<svg><circle cx="50" cy="50" r="40" /></svg>"#.repeat(50);
    
    let config = BenchConfig::new("Tree Builder: SVG")
        .with_warmup(10)
        .with_measurements(50);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    println!("\n=== Tree Builder Benchmark: SVG ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 10: Full document
#[test]
fn bench_tree_builder_full() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test</title>
            <meta charset="UTF-8">
        </head>
        <body>
            <header><h1>Title</h1></header>
            <main>
                <article>
                    <p>Paragraph 1</p>
                    <p>Paragraph 2</p>
                </article>
            </main>
            <footer>Footer</footer>
        </body>
        </html>
    "#;
    
    let config = BenchConfig::new("Tree Builder: Full Document")
        .with_warmup(10)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(html);
    });
    
    println!("\n=== Tree Builder Benchmark: Full Document ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(2));
}

// ============================================================================
// Helper Functions
// ============================================================================

#[allow(dead_code)]
fn print_stats(stats: &crate::ace::html::bench::BenchStats) {
    println!("  Mean:   {:?}", stats.mean);
    println!("  Median: {:?}", stats.median);
    println!("  P95:    {:?}", stats.p95);
    println!("  P99:    {:?}", stats.p99);
    println!("  CV:     {:.2}%", stats.coefficient_of_variation());
}


// ============================================================================
// SIMD BENCHMARKS (5 casos)
// ============================================================================

/// Benchmark 1: Scalar whitespace detection (baseline)
#[test]
fn bench_simd_scalar_whitespace() {
    let text = "   \t\n\r  Hello   World   \t\n  ".repeat(1000);
    
    let config = BenchConfig::new("SIMD: Scalar Whitespace (Baseline)")
        .with_warmup(20)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let bytes = text.as_bytes();
        let mut count = 0;
        for &b in bytes {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                count += 1;
            }
        }
        assert!(count > 0);
    });
    
    println!("\n=== SIMD Benchmark: Scalar Whitespace ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 2: Character search (scalar)
#[test]
fn bench_simd_scalar_char_search() {
    let text = "<div class='test'>Content</div>".repeat(1000);
    
    let config = BenchConfig::new("SIMD: Scalar Character Search")
        .with_warmup(20)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let bytes = text.as_bytes();
        let mut count = 0;
        for &b in bytes {
            if b == b'>' {
                count += 1;
            }
        }
        assert!(count > 0);
    });
    
    println!("\n=== SIMD Benchmark: Scalar Character Search ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 3: Tag boundary detection
#[test]
fn bench_simd_tag_boundaries() {
    let html = "<div><span><p><a><b><i>".repeat(500);
    
    let config = BenchConfig::new("SIMD: Tag Boundary Detection")
        .with_warmup(20)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let bytes = html.as_bytes();
        let mut count = 0;
        for &b in bytes {
            if b == b'<' || b == b'>' {
                count += 1;
            }
        }
        assert!(count > 0);
    });
    
    println!("\n=== SIMD Benchmark: Tag Boundaries ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 4: Entity detection
#[test]
fn bench_simd_entity_scan() {
    let html = "Text with &lt;entities&gt; and &amp; more &quot;entities&quot;".repeat(500);
    
    let config = BenchConfig::new("SIMD: Entity Scanning")
        .with_warmup(20)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let bytes = html.as_bytes();
        let mut count = 0;
        for &b in bytes {
            if b == b'&' || b == b';' {
                count += 1;
            }
        }
        assert!(count > 0);
    });
    
    println!("\n=== SIMD Benchmark: Entity Scanning ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}

/// Benchmark 5: Multi-character pattern matching
#[test]
fn bench_simd_pattern_matching() {
    let html = r#"<div class="test" id="main" data-value="123">"#.repeat(500);
    
    let config = BenchConfig::new("SIMD: Pattern Matching")
        .with_warmup(20)
        .with_measurements(100);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let bytes = html.as_bytes();
        let mut count = 0;
        for &b in bytes {
            // Match quote characters and equals signs
            if b == b'"' || b == b'\'' || b == b'=' {
                count += 1;
            }
        }
        assert!(count > 0);
    });
    
    println!("\n=== SIMD Benchmark: Pattern Matching ===");
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(10));
}
