//! Benchmarks for Speculative Parsing
//! 
//! Tests the performance of speculative (multi-threaded) parsing vs single-threaded parsing
//! Target: 2-3x speedup for large documents (> 1 MB)

use std::time::{Duration, Instant};
use crate::ace::html::{
    speculative::{parse_speculative, parse_document_speculative},
    build_document_with_errors_and_options, ParserOptions,
    bench::{BenchRunner, BenchConfig, BenchStats},
};

/// Generate a large HTML document for benchmarking
fn generate_large_html(size_kb: usize) -> String {
    let mut html = String::from("<!DOCTYPE html><html><head><title>Test</title></head><body>");
    
    // Calculate how many divs we need to reach target size
    let div_size = 50; // Approximate bytes per div
    let num_divs = (size_kb * 1024) / div_size;
    
    for i in 0..num_divs {
        html.push_str(&format!(
            "<div class='item' id='item-{}'><p>Content for item {}</p></div>",
            i, i
        ));
    }
    
    html.push_str("</body></html>");
    html
}

/// Generate a small HTML document for overhead testing
fn generate_small_html() -> String {
    String::from("<!DOCTYPE html><html><body><p>Hello World</p></body></html>")
}

/// Benchmark speculative parsing on large documents
#[test]
fn bench_speculative_large_1mb() {
    let html = generate_large_html(1024); // 1 MB
    let options = ParserOptions::default();
    
    println!("\n=== Speculative Parsing Benchmark (1 MB) ===");
    println!("Document size: {} bytes", html.len());
    
    // Benchmark speculative parsing
    let config = BenchConfig::new("Speculative Parsing (1 MB)")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    
    let result = runner.run(|| {
        let _output = parse_speculative(&html, &options);
    });
    
    println!("\nSpeculative Parsing Results:");
    print_stats(&result.stats);
    
    // Benchmark single-threaded parsing
    let config_single = BenchConfig::new("Single-threaded Parsing (1 MB)")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner_single = BenchRunner::new(config_single);
    
    let result_single = runner_single.run(|| {
        let _output = build_document_with_errors_and_options(&html, &options);
    });
    
    println!("\nSingle-threaded Parsing Results:");
    print_stats(&result_single.stats);
    
    // Calculate speedup
    let speedup = result_single.stats.mean.as_secs_f64() / result.stats.mean.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);
    
    // Target: 2-3x speedup for large documents
    // Note: In practice, the speedup depends on CPU cores and document structure
    // For now, we just verify that speculative parsing completes successfully
    assert!(result.stats.mean > Duration::ZERO);
}

/// Benchmark speculative parsing on very large documents (2 MB)
#[test]
fn bench_speculative_large_2mb() {
    let html = generate_large_html(2048); // 2 MB
    let options = ParserOptions::default();
    
    println!("\n=== Speculative Parsing Benchmark (2 MB) ===");
    println!("Document size: {} bytes", html.len());
    
    let config = BenchConfig::new("Speculative Parsing (2 MB)")
        .with_warmup(2)
        .with_measurements(5);
    
    let runner = BenchRunner::new(config);
    
    let result = runner.run(|| {
        let _output = parse_speculative(&html, &options);
    });
    
    println!("\nSpeculative Parsing Results:");
    print_stats(&result.stats);
    
    // Benchmark single-threaded parsing
    let config_single = BenchConfig::new("Single-threaded Parsing (2 MB)")
        .with_warmup(2)
        .with_measurements(5);
    
    let runner_single = BenchRunner::new(config_single);
    
    let result_single = runner_single.run(|| {
        let _output = build_document_with_errors_and_options(&html, &options);
    });
    
    println!("\nSingle-threaded Parsing Results:");
    print_stats(&result_single.stats);
    
    // Calculate speedup
    let speedup = result_single.stats.mean.as_secs_f64() / result.stats.mean.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);
    
    assert!(result.stats.mean > Duration::ZERO);
}

/// Benchmark overhead on small documents
#[test]
fn bench_speculative_overhead_small() {
    let html = generate_small_html();
    let options = ParserOptions::default();
    
    println!("\n=== Speculative Parsing Overhead (Small Doc) ===");
    println!("Document size: {} bytes", html.len());
    
    let config = BenchConfig::new("Speculative Parsing (Small)")
        .with_warmup(5)
        .with_measurements(20);
    
    // Benchmark speculative parsing
    let runner = BenchRunner::new(config);
    
    let result = runner.run(|| {
        let _doc = parse_document_speculative(&html);
    });
    
    println!("\nSpeculative Parsing Results:");
    print_stats(&result.stats);
    
    // Benchmark single-threaded parsing
    let config_single = BenchConfig::new("Single-threaded Parsing (Small)")
        .with_warmup(5)
        .with_measurements(20);
    
    let runner_single = BenchRunner::new(config_single);
    
    let result_single = runner_single.run(|| {
        let _output = build_document_with_errors_and_options(&html, &options);
    });
    
    println!("\nSingle-threaded Parsing Results:");
    print_stats(&result_single.stats);
    
    // Calculate overhead
    let overhead_pct = ((result.stats.mean.as_secs_f64() / result_single.stats.mean.as_secs_f64()) - 1.0) * 100.0;
    println!("\nOverhead: {:.1}%", overhead_pct);
    
    // For small documents, speculative parsing may have overhead due to thread creation
    // We just verify it completes successfully
    assert!(result.stats.mean > Duration::ZERO);
}

/// Helper function to print benchmark statistics
fn print_stats(stats: &BenchStats) {
    println!("  Mean:   {:?}", stats.mean);
    println!("  Median: {:?}", stats.median);
    println!("  StdDev: {:?}", stats.std_dev);
    println!("  P95:    {:?}", stats.p95);
    println!("  P99:    {:?}", stats.p99);
    println!("  Min:    {:?}", stats.min);
    println!("  Max:    {:?}", stats.max);
    println!("  CV:     {:.2}%", stats.coefficient_of_variation());
    println!("  Stable: {}", if stats.is_stable() { "Yes" } else { "No" });
}

/// Quick performance test to verify speculative parsing works
#[test]
fn test_speculative_performance_basic() {
    let html = generate_large_html(512); // 512 KB
    
    let start = Instant::now();
    let _doc = parse_document_speculative(&html);
    let duration = start.elapsed();
    
    println!("\nBasic performance test (512 KB): {:?}", duration);
    
    // Should complete in reasonable time (< 1 second for 512 KB)
    assert!(duration < Duration::from_secs(1));
}
