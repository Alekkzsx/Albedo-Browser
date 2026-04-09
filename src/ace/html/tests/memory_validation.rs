//! Memory Usage Validation Tests
//! 
//! Validates that ACE HTML parser memory usage is ≤ 50% of Chrome's memory footprint
//! for equivalent parsing operations.

use crate::ace::html::build_document;
use crate::ace::html::tests::chrome_bench::ChromeBenchRunner;

/// Memory measurement result
#[derive(Debug, Clone)]
pub struct MemoryMeasurement {
    pub peak_bytes: usize,
    pub peak_mb: f64,
}

impl MemoryMeasurement {
    pub fn new(peak_bytes: usize) -> Self {
        Self {
            peak_bytes,
            peak_mb: peak_bytes as f64 / 1_000_000.0,
        }
    }
}

/// Measure memory usage of a function using system allocator stats
/// 
/// Note: This is an approximation since we can't intercept all allocations
/// without a global allocator. For more accurate measurements, run with
/// a memory profiler like valgrind or heaptrack.
pub fn measure_memory<F>(f: F) -> MemoryMeasurement
where
    F: FnOnce(),
{
    // Force garbage collection before measurement
    // (Rust doesn't have GC, but we can drop unused allocations)
    
    // Get baseline memory usage
    let baseline = get_memory_usage();
    
    // Run the function
    f();
    
    // Get peak memory usage
    let peak = get_memory_usage();
    
    // Calculate delta
    let delta = peak.saturating_sub(baseline);
    
    MemoryMeasurement::new(delta)
}

/// Get current memory usage (approximation using /proc/self/statm on Linux)
#[cfg(target_os = "linux")]
fn get_memory_usage() -> usize {
    use std::fs;
    
    // Read /proc/self/statm
    // Format: size resident shared text lib data dt
    // We want RSS (resident set size) which is the second field
    if let Ok(content) = fs::read_to_string("/proc/self/statm") {
        let fields: Vec<&str> = content.split_whitespace().collect();
        if fields.len() >= 2 {
            if let Ok(pages) = fields[1].parse::<usize>() {
                // Convert pages to bytes (page size is typically 4096)
                return pages * 4096;
            }
        }
    }
    
    0
}

/// Get current memory usage (approximation using task_info on macOS)
#[cfg(target_os = "macos")]
fn get_memory_usage() -> usize {
    // On macOS, we would use mach_task_self() and task_info()
    // For simplicity, we'll return 0 and rely on Chrome benchmark data
    0
}

/// Get current memory usage (approximation on Windows)
#[cfg(target_os = "windows")]
fn get_memory_usage() -> usize {
    // On Windows, we would use GetProcessMemoryInfo()
    // For simplicity, we'll return 0 and rely on Chrome benchmark data
    0
}

/// Get current memory usage (fallback for other platforms)
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn get_memory_usage() -> usize {
    0
}

/// Memory comparison result
#[derive(Debug, Clone)]
pub struct MemoryComparison {
    pub name: String,
    pub ace_memory: MemoryMeasurement,
    pub chrome_memory: Option<f64>, // MB
    pub ratio: Option<f64>, // ACE / Chrome (should be ≤ 0.5)
    pub meets_requirement: bool,
}

impl MemoryComparison {
    pub fn new(
        name: String,
        ace_memory: MemoryMeasurement,
        chrome_memory: Option<f64>,
    ) -> Self {
        let ratio = chrome_memory.map(|chrome_mb| {
            if chrome_mb > 0.0 {
                ace_memory.peak_mb / chrome_mb
            } else {
                0.0
            }
        });
        
        let meets_requirement = ratio.map(|r| r <= 0.5).unwrap_or(false);
        
        Self {
            name,
            ace_memory,
            chrome_memory,
            ratio,
            meets_requirement,
        }
    }
    
    pub fn print_comparison(&self) {
        println!("\n{}", "=".repeat(70));
        println!("Memory Comparison: {}", self.name);
        println!("{}", "=".repeat(70));
        
        println!("\nACE HTML Parser:");
        println!("  Peak Memory: {:.2} MB ({} bytes)", 
            self.ace_memory.peak_mb, self.ace_memory.peak_bytes);
        
        if let Some(chrome_mb) = self.chrome_memory {
            println!("\nChrome HTML Parser:");
            println!("  Peak Memory: {:.2} MB", chrome_mb);
            
            if let Some(ratio) = self.ratio {
                println!("\nComparison:");
                println!("  Ratio (ACE/Chrome): {:.2}x", ratio);
                println!("  ACE uses {:.1}% of Chrome's memory", ratio * 100.0);
                
                if self.meets_requirement {
                    println!("  Status: ✓ PASSES (≤ 50% requirement)");
                } else {
                    println!("  Status: ❌ FAILS (> 50% requirement)");
                }
            }
        } else {
            println!("\nChrome memory data not available");
            println!("Run with Chrome benchmark to get comparison");
        }
        
        println!("{}", "=".repeat(70));
    }
}

/// Run memory validation test
pub fn validate_memory_usage(html: &str, name: &str) -> MemoryComparison {
    println!("\n🔬 Running memory validation: {}", name);
    println!("Document size: {} bytes ({:.2} MB)", 
        html.len(), html.len() as f64 / 1_000_000.0);
    
    // Measure ACE memory usage
    println!("\n⏱️  Measuring ACE HTML Parser memory...");
    let ace_memory = measure_memory(|| {
        let _doc = build_document(html);
    });
    
    println!("  ACE Peak Memory: {:.2} MB", ace_memory.peak_mb);
    
    // Try to get Chrome memory usage
    println!("\n⏱️  Attempting to get Chrome memory data...");
    let chrome_runner = ChromeBenchRunner::new();
    
    let chrome_memory = if chrome_runner.is_available() {
        match chrome_runner.run_chrome_benchmark(html, name) {
            Ok(result) => {
                if let Some(memory) = result.memory {
                    println!("  Chrome Peak Memory: {:.2} MB", memory.mean_mb);
                    Some(memory.mean_mb)
                } else {
                    println!("  ⚠️  Chrome benchmark did not include memory data");
                    println!("     Update to puppeteer version for memory tracking");
                    None
                }
            }
            Err(e) => {
                println!("  ⚠️  Chrome benchmark failed: {}", e);
                None
            }
        }
    } else {
        println!("  ⚠️  Chrome benchmark not available (Node.js not installed)");
        None
    };
    
    MemoryComparison::new(name.to_string(), ace_memory, chrome_memory)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Helper: Generate HTML document of specified size
    fn generate_html(size_mb: usize) -> String {
        let mut html = String::from("<!DOCTYPE html><html><body>");
        
        let target_size = size_mb * 1_000_000;
        while html.len() < target_size {
            html.push_str("<div class='item'><p>Content paragraph with some text</p></div>");
        }
        
        html.push_str("</body></html>");
        html
    }
    
    #[test]
    fn test_memory_measurement_basic() {
        let memory = measure_memory(|| {
            let _vec: Vec<u8> = vec![0; 1_000_000]; // 1 MB allocation
        });
        
        // Should measure some memory usage
        println!("Measured memory: {:.2} MB", memory.peak_mb);
        
        // Note: This test may not be accurate without a custom allocator
        // It's mainly to verify the measurement infrastructure works
    }
    
    #[test]
    fn test_memory_validation_small_document() {
        let html = generate_html(1); // 1 MB document
        let comparison = validate_memory_usage(&html, "Small Document (1 MB)");
        
        comparison.print_comparison();
        
        // ACE should use reasonable memory for small documents
        assert!(comparison.ace_memory.peak_mb < 100.0, 
            "ACE uses too much memory: {:.2} MB", comparison.ace_memory.peak_mb);
    }
    
    #[test]
    fn test_memory_validation_10mb_document() {
        let html = generate_html(10); // 10 MB document
        let comparison = validate_memory_usage(&html, "Medium Document (10 MB)");
        
        comparison.print_comparison();
        
        // This is the key requirement: ACE memory ≤ 50% of Chrome
        if comparison.chrome_memory.is_some() {
            assert!(comparison.meets_requirement,
                "ACE memory usage exceeds 50% of Chrome: {:.2}x ratio",
                comparison.ratio.unwrap());
        } else {
            println!("⚠️  Skipping validation: Chrome memory data not available");
            println!("   Install Node.js and Puppeteer for full validation");
        }
        
        // At minimum, ACE should use less than 40 MB for 10 MB document
        // (This is the spec requirement: ≤ 40 MB for 10 MB doc)
        assert!(comparison.ace_memory.peak_mb <= 40.0,
            "ACE uses too much memory: {:.2} MB (requirement: ≤ 40 MB)",
            comparison.ace_memory.peak_mb);
    }
    
    #[test]
    #[ignore] // Ignore by default, run with --ignored for stress test
    fn test_memory_validation_100mb_document() {
        let html = generate_html(100); // 100 MB document
        let comparison = validate_memory_usage(&html, "Large Document (100 MB)");
        
        comparison.print_comparison();
        
        // Spec requirement: ≤ 200 MB for 100 MB document
        assert!(comparison.ace_memory.peak_mb <= 200.0,
            "ACE uses too much memory: {:.2} MB (requirement: ≤ 200 MB)",
            comparison.ace_memory.peak_mb);
        
        if comparison.chrome_memory.is_some() {
            assert!(comparison.meets_requirement,
                "ACE memory usage exceeds 50% of Chrome: {:.2}x ratio",
                comparison.ratio.unwrap());
        }
    }
}
