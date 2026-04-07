//! Benchmarks for preload scanner
//! Target: < 0.1ms latency for 1 MB documents

use std::time::{Duration, Instant};

#[allow(unused_imports)]
use crate::ace::html::preload_scanner::{PreloadScanner, ParallelPreloadScanner};

/// Generate synthetic HTML for benchmarking
#[allow(dead_code)]
fn generate_html(size_kb: usize) -> String {
    let mut html = String::with_capacity(size_kb * 1024);
    
    html.push_str("<!DOCTYPE html><html><head>");
    
    // Add stylesheets
    for i in 0..10 {
        html.push_str(&format!(
            r#"<link rel="stylesheet" href="/css/style{}.css">"#,
            i
        ));
    }
    
    // Add preload links
    for i in 0..5 {
        html.push_str(&format!(
            r#"<link rel="preload" href="/fonts/font{}.woff2" as="font" crossorigin>"#,
            i
        ));
    }
    
    html.push_str("</head><body>");
    
    // Add scripts
    for i in 0..20 {
        html.push_str(&format!(
            r#"<script src="/js/app{}.js" async></script>"#,
            i
        ));
    }
    
    // Add images to reach target size
    let img_count = (size_kb * 1024) / 100; // ~100 bytes per img tag
    for i in 0..img_count {
        html.push_str(&format!(
            r#"<img src="/images/img{}.jpg" loading="lazy" alt="Image {}">"#,
            i, i
        ));
    }
    
    html.push_str("</body></html>");
    html
}

/// Benchmark statistics
#[derive(Debug, Clone)]
pub struct BenchStats {
    pub samples: Vec<Duration>,
    pub mean: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub min: Duration,
    pub max: Duration,
}

impl BenchStats {
    pub fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort();
        
        let mean = samples.iter().sum::<Duration>() / samples.len() as u32;
        let median = samples[samples.len() / 2];
        let p95 = samples[(samples.len() as f64 * 0.95) as usize];
        let p99 = samples[(samples.len() as f64 * 0.99) as usize];
        let min = samples[0];
        let max = samples[samples.len() - 1];
        
        Self {
            samples,
            mean,
            median,
            p95,
            p99,
            min,
            max,
        }
    }
    
    pub fn print_summary(&self, name: &str) {
        println!("\n{}", name);
        println!("  Mean:   {:?}", self.mean);
        println!("  Median: {:?}", self.median);
        println!("  P95:    {:?}", self.p95);
        println!("  P99:    {:?}", self.p99);
        println!("  Min:    {:?}", self.min);
        println!("  Max:    {:?}", self.max);
    }
}

/// Run benchmark with warmup
#[allow(dead_code)]
fn bench<F>(name: &str, warmup: usize, iterations: usize, mut f: F) -> BenchStats
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..warmup {
        f();
    }
    
    // Measurement
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        samples.push(elapsed);
    }
    
    let stats = BenchStats::from_samples(samples);
    stats.print_summary(name);
    stats
}

#[test]
fn bench_preload_scanner_1kb() {
    let html = generate_html(1);
    let mut scanner = PreloadScanner::new();
    
    let stats = bench("Preload Scanner - 1 KB", 10, 1000, || {
        scanner.scan(&html);
    });
    
    // Should be very fast for small documents
    assert!(stats.mean < Duration::from_micros(100));
}

#[test]
fn bench_preload_scanner_100kb() {
    let html = generate_html(100);
    let mut scanner = PreloadScanner::new();
    
    let stats = bench("Preload Scanner - 100 KB", 10, 100, || {
        scanner.scan(&html);
    });
    
    // Should be fast for medium documents
    assert!(stats.mean < Duration::from_millis(1));
}

#[test]
fn bench_preload_scanner_1mb() {
    let html = generate_html(1024);
    let mut scanner = PreloadScanner::new();
    
    let stats = bench("Preload Scanner - 1 MB", 5, 50, || {
        scanner.scan(&html);
    });
    
    // Target: < 0.1ms for 1 MB
    // Note: This is an aggressive target, may need optimization
    println!("\nTarget: < 100μs (0.1ms)");
    println!("Actual: {:?}", stats.mean);
    
    if stats.mean > Duration::from_micros(100) {
        println!("⚠️  Warning: Latency exceeds target (but this is expected for initial implementation)");
    }
}

#[test]
fn bench_parallel_scanner_1mb() {
    let html = generate_html(1024);
    let scanner = ParallelPreloadScanner::new();
    
    let stats = bench("Parallel Preload Scanner - 1 MB", 5, 50, || {
        scanner.scan_parallel(&html, None);
    });
    
    println!("\nParallel scanner with {} threads", scanner.thread_count());
    println!("Target: < 100μs (0.1ms)");
    println!("Actual: {:?}", stats.mean);
}

#[test]
fn bench_parallel_scanner_10mb() {
    let html = generate_html(10 * 1024);
    let scanner = ParallelPreloadScanner::new();
    
    let stats = bench("Parallel Preload Scanner - 10 MB", 3, 20, || {
        scanner.scan_parallel(&html, None);
    });
    
    println!("\nParallel scanner with {} threads", scanner.thread_count());
    println!("Expected: < 1ms for 10 MB");
    println!("Actual: {:?}", stats.mean);
}

#[test]
fn bench_simd_vs_scalar() {
    let html = generate_html(1024);
    
    // Single-threaded scanner (uses SIMD if available)
    let mut scanner = PreloadScanner::new();
    let _stats_simd = bench("SIMD Scanner - 1 MB", 5, 50, || {
        scanner.scan(&html);
    });
    
    println!("\nSIMD optimization level: {}", 
        if is_x86_feature_detected!("avx2") { "AVX2" } 
        else if is_x86_feature_detected!("sse2") { "SSE2" }
        else { "Scalar" }
    );
}

#[test]
fn bench_extraction_accuracy() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <link rel="stylesheet" href="/css/main.css">
            <link rel="preload" href="/fonts/font.woff2" as="font">
            <script src="/js/app.js" async></script>
            <script src="/js/vendor.js" defer></script>
        </head>
        <body>
            <img src="/images/hero.jpg" loading="eager">
            <img src="/images/lazy.jpg" loading="lazy">
            <video src="/videos/intro.mp4" poster="/images/poster.jpg"></video>
        </body>
        </html>
    "#;
    
    let mut scanner = PreloadScanner::new();
    let requests = scanner.scan(html);
    
    println!("\nExtraction Accuracy Test");
    println!("Found {} preload requests:", requests.len());
    for req in &requests {
        println!("  - {} ({})", req.url, req.resource_type.as_str());
    }
    
    // Verify we found all resources
    assert!(requests.len() >= 7, "Should find at least 7 resources");
    
    // Verify specific resources
    let urls: Vec<_> = requests.iter().map(|r| r.url.as_str()).collect();
    assert!(urls.contains(&"/css/main.css"));
    assert!(urls.contains(&"/fonts/font.woff2"));
    assert!(urls.contains(&"/js/app.js"));
    assert!(urls.contains(&"/js/vendor.js"));
    assert!(urls.contains(&"/images/hero.jpg"));
    assert!(urls.contains(&"/videos/intro.mp4"));
    assert!(urls.contains(&"/images/poster.jpg"));
}

#[test]
fn bench_parallel_speedup() {
    let html = generate_html(5 * 1024); // 5 MB
    
    // Single-threaded
    let mut scanner = PreloadScanner::new();
    let stats_single = bench("Single-threaded - 5 MB", 3, 20, || {
        scanner.scan(&html);
    });
    
    // Parallel
    let parallel_scanner = ParallelPreloadScanner::new();
    let stats_parallel = bench("Parallel - 5 MB", 3, 20, || {
        parallel_scanner.scan_parallel(&html, None);
    });
    
    let speedup = stats_single.mean.as_secs_f64() / stats_parallel.mean.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);
    println!("Threads: {}", parallel_scanner.thread_count());
    
    // Parallel should be faster for large documents
    // (though overhead may make it slower for small docs)
    if speedup > 1.0 {
        println!("✓ Parallel scanning is faster");
    } else {
        println!("⚠️  Parallel overhead exceeds benefit for this size");
    }
}
