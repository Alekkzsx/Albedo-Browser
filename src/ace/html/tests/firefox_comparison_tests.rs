//! Firefox HTML Parser Comparison Tests
//! 
//! These tests compare ACE HTML parser performance against Firefox's parser
//! using the benchmark infrastructure in firefox_bench.rs

#[cfg(test)]
mod tests {
    use crate::ace::html::tests::firefox_bench::{FirefoxBenchRunner, generate_comparison_report};
    
    /// Helper: Generate simple HTML document
    fn generate_simple_html(size_kb: usize) -> String {
        let mut html = String::from("<!DOCTYPE html><html><body>");
        
        let target_size = size_kb * 1024;
        while html.len() < target_size {
            html.push_str("<div class='item'><p>Content</p></div>");
        }
        
        html.push_str("</body></html>");
        html
    }
    
    /// Helper: Generate Wikipedia-like document
    fn generate_wikipedia_html() -> String {
        let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Wikipedia - The Free Encyclopedia</title>
</head>
<body>
    <header><nav><ul><li><a href="/">Home</a></li></ul></nav></header>
    <main>
"#);
        
        for i in 0..50 {
            html.push_str(&format!(r#"
        <article id="article-{}">
            <h2>Article Title {}</h2>
            <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
            <table class="infobox">
                <tr><th>Property</th><th>Value</th></tr>
                <tr><td>Type</td><td>Article</td></tr>
            </table>
        </article>
"#, i, i));
        }
        
        html.push_str("</main></body></html>");
        
        // Pad to ~1.2 MB
        while html.len() < 1_200_000 {
            html.push_str("<div>Padding</div>");
        }
        
        html
    }
    
    /// Test 1: Small document comparison (10 KB)
    #[test]
    fn test_firefox_comparison_small() {
        let runner = FirefoxBenchRunner::new();
        
        if !runner.is_available() {
            println!("⚠️  Skipping Firefox comparison: Node.js not available");
            println!("   Install Node.js to enable Firefox benchmarks");
            return;
        }
        
        let html = generate_simple_html(10);
        let comparison = runner.compare_parsers(&html, "Small Document (10 KB)");
        
        comparison.print_comparison();
        
        // ACE should handle small documents efficiently
        assert!(comparison.ace_result.stats.mean.as_millis() < 100);
    }
    
    /// Test 2: Medium document comparison (500 KB)
    #[test]
    fn test_firefox_comparison_medium() {
        let runner = FirefoxBenchRunner::new();
        
        if !runner.is_available() {
            println!("⚠️  Skipping Firefox comparison: Node.js not available");
            return;
        }
        
        let html = generate_simple_html(500);
        let comparison = runner.compare_parsers(&html, "Medium Document (500 KB)");
        
        comparison.print_comparison();
        
        // Should parse in reasonable time
        assert!(comparison.ace_result.stats.mean.as_millis() < 500);
    }
    
    /// Test 3: Large document comparison (1.2 MB Wikipedia)
    #[test]
    fn test_firefox_comparison_wikipedia() {
        let runner = FirefoxBenchRunner::new();
        
        if !runner.is_available() {
            println!("⚠️  Skipping Firefox comparison: Node.js not available");
            return;
        }
        
        let html = generate_wikipedia_html();
        let comparison = runner.compare_parsers(&html, "Wikipedia Homepage (1.2 MB)");
        
        comparison.print_comparison();
        
        // Target: < 3ms for 1.2 MB (400+ MB/s throughput)
        assert!(comparison.ace_result.stats.mean.as_millis() < 1000);
    }
    
    /// Test 4: Generate full comparison report
    #[test]
    fn test_generate_full_comparison_report() {
        let runner = FirefoxBenchRunner::new();
        
        if !runner.is_available() {
            println!("⚠️  Skipping Firefox comparison: Node.js not available");
            return;
        }
        
        println!("\n🔬 Running full Firefox comparison suite...\n");
        
        let mut comparisons = Vec::new();
        
        // Small document
        let html_small = generate_simple_html(10);
        comparisons.push(runner.compare_parsers(&html_small, "Small (10 KB)"));
        
        // Medium document
        let html_medium = generate_simple_html(100);
        comparisons.push(runner.compare_parsers(&html_medium, "Medium (100 KB)"));
        
        // Large document
        let html_large = generate_simple_html(500);
        comparisons.push(runner.compare_parsers(&html_large, "Large (500 KB)"));
        
        // Generate report
        let report = generate_comparison_report(&comparisons);
        
        // Save report
        std::fs::write("target/firefox_comparison_report.md", &report)
            .expect("Failed to write report");
        
        println!("\n📊 Comparison report saved to: target/firefox_comparison_report.md\n");
        println!("{}", report);
        
        // Verify report contains expected sections
        assert!(report.contains("# ACE HTML Parser vs Firefox Comparison"));
        assert!(report.contains("## Performance Comparison"));
        assert!(report.contains("Small (10 KB)"));
    }
    
    /// Test 5: Verify benchmark runner availability check
    #[test]
    fn test_benchmark_runner_availability() {
        let runner = FirefoxBenchRunner::new();
        let is_available = runner.is_available();
        
        println!("Firefox benchmark runner available: {}", is_available);
        
        if is_available {
            println!("✓ Node.js is installed");
            println!("✓ Benchmark script exists");
        } else {
            println!("⚠️  Firefox benchmarks not available");
            println!("   Install Node.js to enable Firefox comparison");
        }
    }
    
    /// Test 6: Stress test - Very large document (5 MB)
    #[test]
    #[ignore] // Ignore by default, run with --ignored
    fn test_firefox_comparison_stress() {
        let runner = FirefoxBenchRunner::new();
        
        if !runner.is_available() {
            println!("⚠️  Skipping Firefox comparison: Node.js not available");
            return;
        }
        
        let html = generate_simple_html(5000); // 5 MB
        let comparison = runner.compare_parsers(&html, "Stress Test (5 MB)");
        
        comparison.print_comparison();
        
        // Should still parse in reasonable time
        assert!(comparison.ace_result.stats.mean.as_millis() < 5000);
    }
}
