//! Synthetic stress tests for HTML parser
//! 
//! Tests extreme cases: large tables, deep nesting, many attributes, entity-heavy

#[cfg(test)]
use crate::ace::html::{
    build_document,
    bench::{BenchRunner, BenchConfig},
};
#[cfg(test)]
use std::time::Duration;

// ============================================================================
// SYNTHETIC STRESS TESTS
// ============================================================================

/// Generate large table (10K rows)
#[allow(dead_code)]
fn generate_large_table() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head><title>Large Table Test</title></head>
<body>
<table id="large-table">
    <thead>
        <tr>
            <th>ID</th>
            <th>Name</th>
            <th>Value</th>
            <th>Status</th>
            <th>Date</th>
        </tr>
    </thead>
    <tbody>
"#);
    
    // Generate 10,000 rows
    for i in 0..10_000 {
        html.push_str(&format!(
            "        <tr><td>{}</td><td>Item {}</td><td>{}</td><td>Active</td><td>2026-04-07</td></tr>\n",
            i, i, i * 100
        ));
    }
    
    html.push_str(r#"    </tbody>
</table>
</body>
</html>
"#);
    
    html
}

/// Generate deeply nested structure (100 levels - reasonable limit)
#[allow(dead_code)]
fn generate_deep_nesting() -> String {
    let mut html = String::from("<!DOCTYPE html><html><body>");
    
    // Create 100 levels of nesting (more reasonable than 1000)
    for i in 0..100 {
        html.push_str(&format!("<div id='level-{}'>", i));
    }
    
    html.push_str("Deep content");
    
    // Close all divs
    for _ in 0..100 {
        html.push_str("</div>");
    }
    
    html.push_str("</body></html>");
    html
}

/// Generate elements with many attributes (100 per element)
#[allow(dead_code)]
fn generate_many_attributes() -> String {
    let mut html = String::from("<!DOCTYPE html><html><body>");
    
    // Generate 100 elements, each with 100 attributes
    for i in 0..100 {
        html.push_str(&format!("<div id='elem-{}'", i));
        
        // Add 100 attributes
        for j in 0..100 {
            html.push_str(&format!(" data-attr-{}='value-{}'", j, j));
        }
        
        html.push_str(&format!(">Element {}</div>", i));
    }
    
    html.push_str("</body></html>");
    html
}

/// Generate entity-heavy document (50% entities)
#[allow(dead_code)]
fn generate_entity_heavy() -> String {
    let mut html = String::from("<!DOCTYPE html><html><body>");
    
    // Generate text with 50% character entities
    for i in 0..1000 {
        html.push_str("<p>");
        
        // Alternate between text and entities
        for j in 0..50 {
            if j % 2 == 0 {
                html.push_str("Text ");
            } else {
                html.push_str("&lt;&gt;&amp;&quot;&apos; ");
            }
        }
        
        html.push_str(&format!("Paragraph {}</p>", i));
    }
    
    html.push_str("</body></html>");
    html
}

// ============================================================================
// BENCHMARK TESTS
// ============================================================================

/// Benchmark 1: Large table (10K rows)
#[test]
fn bench_stress_large_table() {
    let html = generate_large_table();
    
    println!("\n=== Stress Test: Large Table (10K rows) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    println!("Rows: 10,000");
    
    let config = BenchConfig::new("Large Table (10K rows)")
        .with_warmup(2)
        .with_measurements(5);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    // Should handle large tables efficiently
    assert!(result.stats.mean < Duration::from_millis(500));
}

/// Benchmark 2: Deep nesting (100 levels)
#[test]
fn bench_stress_deep_nesting() {
    let html = generate_deep_nesting();
    
    println!("\n=== Stress Test: Deep Nesting (100 levels) ===");
    println!("Document size: {} bytes", html.len());
    println!("Nesting depth: 100 levels");
    
    let config = BenchConfig::new("Deep Nesting (100 levels)")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    // Should handle deep nesting without stack overflow
    assert!(result.stats.mean < Duration::from_millis(50));
}

/// Benchmark 3: Many attributes (100 per element)
#[test]
fn bench_stress_many_attributes() {
    let html = generate_many_attributes();
    
    println!("\n=== Stress Test: Many Attributes (100 per element) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    println!("Elements: 100, Attributes per element: 100");
    
    let config = BenchConfig::new("Many Attributes")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    // Should handle many attributes efficiently
    assert!(result.stats.mean < Duration::from_millis(200));
}

/// Benchmark 4: Entity heavy (50% entities)
#[test]
fn bench_stress_entity_heavy() {
    let html = generate_entity_heavy();
    
    println!("\n=== Stress Test: Entity Heavy (50% entities) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    println!("Paragraphs: 1,000 with heavy entity usage");
    
    let config = BenchConfig::new("Entity Heavy")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    // Should handle entity decoding efficiently
    assert!(result.stats.mean < Duration::from_millis(300));
}

// ============================================================================
// Helper Functions
// ============================================================================

#[cfg(test)]
#[allow(dead_code)]
fn print_stats(stats: &crate::ace::html::bench::BenchStats) {
    println!("  Mean:   {:?}", stats.mean);
    println!("  Median: {:?}", stats.median);
    println!("  P95:    {:?}", stats.p95);
    println!("  P99:    {:?}", stats.p99);
    println!("  Min:    {:?}", stats.min);
    println!("  Max:    {:?}", stats.max);
    println!("  CV:     {:.2}%", stats.coefficient_of_variation());
    println!("  Stable: {}", if stats.is_stable() { "Yes" } else { "No" });
}
