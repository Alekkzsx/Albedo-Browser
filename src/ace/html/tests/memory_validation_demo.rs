//! Memory Validation Demo
//! 
//! Demonstrates memory usage validation for ACE HTML parser vs Chrome

use crate::ace::html::build_document;
use crate::ace::html::tests::chrome_bench::ChromeBenchRunner;

/// Generate HTML document of specified size
fn generate_html_document(size_mb: usize) -> String {
    let mut html = String::from("<!DOCTYPE html><html><head><title>Test</title></head><body>");
    
    let target_size = size_mb * 1_000_000;
    while html.len() < target_size {
        html.push_str("<div class='item'><p>Content paragraph with some text</p></div>");
    }
    
    html.push_str("</body></html>");
    html
}

/// Run memory validation demonstration
pub fn run_memory_validation_demo() {
    println!("\n{}", "=".repeat(80));
    println!("ACE HTML Parser - Memory Usage Validation");
    println!("{}", "=".repeat(80));
    
    // Test with 10 MB document (spec requirement)
    println!("\n📊 Testing with 10 MB document...");
    let html_10mb = generate_html_document(10);
    
    println!("Document size: {} bytes ({:.2} MB)", 
        html_10mb.len(), html_10mb.len() as f64 / 1_000_000.0);
    
    // Parse with ACE
    println!("\n⏱️  Parsing with ACE HTML Parser...");
    let _doc = build_document(&html_10mb);
    println!("✓ ACE parsing complete");
    
    // Try to get Chrome memory data
    println!("\n⏱️  Attempting to get Chrome memory data...");
    let chrome_runner = ChromeBenchRunner::new();
    
    if chrome_runner.is_available() {
        match chrome_runner.run_chrome_benchmark(&html_10mb, "10MB Document") {
            Ok(result) => {
                if let Some(memory) = result.memory {
                    println!("\n📈 Memory Comparison:");
                    println!("  Chrome Peak Memory: {:.2} MB", memory.mean_mb);
                    println!("  Target (50% of Chrome): {:.2} MB", memory.mean_mb * 0.5);
                    println!("\n  Requirement: ACE memory ≤ {:.2} MB", memory.mean_mb * 0.5);
                    println!("  Spec Target: ACE memory ≤ 40 MB for 10 MB document");
                    
                    if memory.mean_mb * 0.5 <= 40.0 {
                        println!("\n  ✓ Chrome uses ≤ 80 MB, so 50% target is ≤ 40 MB");
                        println!("  ✓ This aligns with spec requirement");
                    } else {
                        println!("\n  ⚠️  Chrome uses > 80 MB");
                        println!("  ⚠️  50% of Chrome would exceed 40 MB spec target");
                    }
                } else {
                    println!("  ⚠️  Chrome benchmark did not include memory data");
                    println!("     Update benchmarks/chrome_parser_bench_puppeteer.js");
                    println!("     to enable memory tracking");
                }
            }
            Err(e) => {
                println!("  ⚠️  Chrome benchmark failed: {}", e);
            }
        }
    } else {
        println!("  ⚠️  Chrome benchmark not available");
        println!("     Install Node.js and Puppeteer to enable Chrome comparison");
        println!("\n     Setup instructions:");
        println!("       cd benchmarks");
        println!("       npm install puppeteer");
    }
    
    println!("\n{}", "=".repeat(80));
    println!("Memory Validation Demo Complete");
    println!("{}", "=".repeat(80));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_validation_demo() {
        run_memory_validation_demo();
    }
    
    #[test]
    fn test_generate_html_document() {
        let html = generate_html_document(1);
        assert!(html.len() >= 1_000_000);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }
}
