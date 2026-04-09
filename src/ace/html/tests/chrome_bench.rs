//! Chrome HTML Parser Benchmark Comparison
//! 
//! This module provides infrastructure to benchmark ACE HTML parser against
//! Chrome's HTML parser using Node.js with Puppeteer or similar tools.
//! 
//! Since we can't directly call Chrome's C++ parser from Rust, we:
//! 1. Generate benchmark HTML documents
//! 2. Run ACE parser benchmarks in Rust
//! 3. Run Chrome parser benchmarks via Node.js script
//! 4. Compare and generate reports

use std::time::Duration;
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::ace::html::{build_document, bench::{BenchRunner, BenchConfig, BenchResult}};

/// Chrome benchmark result from Node.js
#[derive(Debug, Clone)]
pub struct ChromeBenchResult {
    pub name: String,
    pub mean: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub min: Duration,
    pub max: Duration,
    pub throughput_mbps: f64,
    pub memory: Option<MemoryStats>,
}

/// Memory statistics from Chrome benchmark
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub mean_bytes: f64,
    pub median_bytes: f64,
    pub p95_bytes: f64,
    pub p99_bytes: f64,
    pub min_bytes: f64,
    pub max_bytes: f64,
    pub mean_mb: f64,
    pub median_mb: f64,
    pub p95_mb: f64,
    pub p99_mb: f64,
}

/// Comparison between ACE and Chrome
#[derive(Debug, Clone)]
pub struct ParserComparison {
    pub name: String,
    pub ace_result: BenchResult,
    pub chrome_result: Option<ChromeBenchResult>,
    pub speedup: Option<f64>, // ACE vs Chrome (> 1.0 means ACE is faster)
}

impl ParserComparison {
    pub fn new(name: String, ace_result: BenchResult, chrome_result: Option<ChromeBenchResult>) -> Self {
        let speedup = chrome_result.as_ref().map(|chrome| {
            let ace_secs = ace_result.stats.mean.as_secs_f64();
            let chrome_secs = chrome.mean.as_secs_f64();
            if ace_secs > 0.0 {
                chrome_secs / ace_secs
            } else {
                1.0
            }
        });
        
        Self {
            name,
            ace_result,
            chrome_result,
            speedup,
        }
    }
    
    pub fn print_comparison(&self) {
        println!("\n{}", "=".repeat(70));
        println!("Benchmark: {}", self.name);
        println!("{}", "=".repeat(70));
        
        println!("\nACE HTML Parser:");
        println!("  Mean:       {:?}", self.ace_result.stats.mean);
        println!("  Median:     {:?}", self.ace_result.stats.median);
        println!("  P95:        {:?}", self.ace_result.stats.p95);
        println!("  P99:        {:?}", self.ace_result.stats.p99);
        
        if let Some(chrome) = &self.chrome_result {
            println!("\nChrome HTML Parser:");
            println!("  Mean:       {:?}", chrome.mean);
            println!("  Median:     {:?}", chrome.median);
            println!("  P95:        {:?}", chrome.p95);
            println!("  P99:        {:?}", chrome.p99);
            println!("  Throughput: {:.2} MB/s", chrome.throughput_mbps);
            
            if let Some(speedup) = self.speedup {
                println!("\nComparison:");
                println!("  Speedup:    {:.2}x", speedup);
                
                if speedup > 1.0 {
                    println!("  Status:     ✓ ACE is {:.1}% faster", (speedup - 1.0) * 100.0);
                } else {
                    println!("  Status:     ⚠ ACE is {:.1}% slower", (1.0 - speedup) * 100.0);
                }
            }
        } else {
            println!("\nChrome benchmark not available (Node.js script not run)");
        }
        
        println!("{}", "=".repeat(70));
    }
}

/// Chrome benchmark runner
pub struct ChromeBenchRunner {
    node_script_path: String,
}

impl ChromeBenchRunner {
    pub fn new() -> Self {
        Self {
            node_script_path: "benchmarks/chrome_parser_bench.js".to_string(),
        }
    }
    
    /// Check if Node.js and the benchmark script are available
    pub fn is_available(&self) -> bool {
        // Check if Node.js is installed
        let node_check = Command::new("node")
            .arg("--version")
            .output();
        
        if node_check.is_err() {
            return false;
        }
        
        // Check if benchmark script exists
        Path::new(&self.node_script_path).exists()
    }
    
    /// Run Chrome benchmark via Node.js
    pub fn run_chrome_benchmark(&self, html: &str, name: &str) -> Result<ChromeBenchResult, String> {
        if !self.is_available() {
            return Err("Node.js or benchmark script not available".to_string());
        }
        
        // Write HTML to temporary file
        let temp_path = format!("target/bench_temp_{}.html", name.replace(" ", "_"));
        fs::write(&temp_path, html)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        // Run Node.js benchmark script
        let output = Command::new("node")
            .arg(&self.node_script_path)
            .arg(&temp_path)
            .arg(name)
            .output()
            .map_err(|e| format!("Failed to run Node.js: {}", e))?;
        
        // Clean up temp file
        let _ = fs::remove_file(&temp_path);
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Node.js script failed: {}", stderr));
        }
        
        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_chrome_result(&stdout, name)
    }
    
    /// Parse Chrome benchmark result from JSON
    fn parse_chrome_result(&self, json: &str, name: &str) -> Result<ChromeBenchResult, String> {
        // Simple JSON parsing (we avoid serde to maintain zero dependencies)
        // Expected format: {"mean":1.5,"median":1.4,"p95":2.0,"p99":2.5,"min":1.0,"max":3.0,"throughput":500.0,"memory":{...}}
        
        let parse_field = |field: &str| -> Result<f64, String> {
            json.split(&format!("\"{}\":", field))
                .nth(1)
                .and_then(|s| s.split(',').next().or_else(|| s.split('}').next()))
                .and_then(|s| s.trim().parse::<f64>().ok())
                .ok_or_else(|| format!("Failed to parse field: {}", field))
        };
        
        // Parse memory stats if available
        let memory = if json.contains("\"memory\"") {
            let parse_memory_field = |field: &str| -> Option<f64> {
                json.split(&format!("\"{}\":", field))
                    .nth(1)
                    .and_then(|s| s.split(',').next().or_else(|| s.split('}').next()))
                    .and_then(|s| s.trim().parse::<f64>().ok())
            };
            
            Some(MemoryStats {
                mean_bytes: parse_memory_field("mean_bytes").unwrap_or(0.0),
                median_bytes: parse_memory_field("median_bytes").unwrap_or(0.0),
                p95_bytes: parse_memory_field("p95_bytes").unwrap_or(0.0),
                p99_bytes: parse_memory_field("p99_bytes").unwrap_or(0.0),
                min_bytes: parse_memory_field("min_bytes").unwrap_or(0.0),
                max_bytes: parse_memory_field("max_bytes").unwrap_or(0.0),
                mean_mb: parse_memory_field("mean_mb").unwrap_or(0.0),
                median_mb: parse_memory_field("median_mb").unwrap_or(0.0),
                p95_mb: parse_memory_field("p95_mb").unwrap_or(0.0),
                p99_mb: parse_memory_field("p99_mb").unwrap_or(0.0),
            })
        } else {
            None
        };
        
        Ok(ChromeBenchResult {
            name: name.to_string(),
            mean: Duration::from_secs_f64(parse_field("mean")? / 1000.0),
            median: Duration::from_secs_f64(parse_field("median")? / 1000.0),
            p95: Duration::from_secs_f64(parse_field("p95")? / 1000.0),
            p99: Duration::from_secs_f64(parse_field("p99")? / 1000.0),
            min: Duration::from_secs_f64(parse_field("min")? / 1000.0),
            max: Duration::from_secs_f64(parse_field("max")? / 1000.0),
            throughput_mbps: parse_field("throughput")?,
            memory,
        })
    }
    
    /// Run comparison benchmark: ACE vs Chrome
    pub fn compare_parsers(&self, html: &str, name: &str) -> ParserComparison {
        println!("\n🔬 Running comparison benchmark: {}", name);
        println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
        
        // Run ACE benchmark
        println!("\n⏱️  Benchmarking ACE HTML Parser...");
        let config = BenchConfig::new(name)
            .with_warmup(5)
            .with_measurements(20);
        
        let runner = BenchRunner::new(config);
        let ace_result = runner.run(|| {
            let _doc = build_document(html);
        });
        
        println!("  ACE Mean: {:?}", ace_result.stats.mean);
        
        // Run Chrome benchmark
        println!("\n⏱️  Benchmarking Chrome HTML Parser...");
        let chrome_result = match self.run_chrome_benchmark(html, name) {
            Ok(result) => {
                println!("  Chrome Mean: {:?}", result.mean);
                Some(result)
            }
            Err(e) => {
                println!("  ⚠️  Chrome benchmark failed: {}", e);
                None
            }
        };
        
        ParserComparison::new(name.to_string(), ace_result, chrome_result)
    }
}

impl Default for ChromeBenchRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate comparison report
pub fn generate_comparison_report(comparisons: &[ParserComparison]) -> String {
    let mut report = String::from("# ACE HTML Parser vs Chrome Comparison\n\n");
    report.push_str("## Performance Comparison\n\n");
    report.push_str("| Benchmark | ACE Mean | Chrome Mean | Speedup | Status |\n");
    report.push_str("|-----------|----------|-------------|---------|--------|\n");
    
    for comp in comparisons {
        let ace_ms = comp.ace_result.stats.mean.as_secs_f64() * 1000.0;
        
        if let Some(chrome) = &comp.chrome_result {
            let chrome_ms = chrome.mean.as_secs_f64() * 1000.0;
            let speedup = comp.speedup.unwrap_or(1.0);
            let status = if speedup > 1.0 {
                format!("✓ {:.1}% faster", (speedup - 1.0) * 100.0)
            } else {
                format!("⚠ {:.1}% slower", (1.0 - speedup) * 100.0)
            };
            
            report.push_str(&format!(
                "| {} | {:.2}ms | {:.2}ms | {:.2}x | {} |\n",
                comp.name, ace_ms, chrome_ms, speedup, status
            ));
        } else {
            report.push_str(&format!(
                "| {} | {:.2}ms | N/A | N/A | Chrome data unavailable |\n",
                comp.name, ace_ms
            ));
        }
    }
    
    report.push_str("\n## Detailed Statistics\n\n");
    
    for comp in comparisons {
        report.push_str(&format!("### {}\n\n", comp.name));
        report.push_str("**ACE HTML Parser:**\n");
        report.push_str(&format!("- Mean: {:?}\n", comp.ace_result.stats.mean));
        report.push_str(&format!("- Median: {:?}\n", comp.ace_result.stats.median));
        report.push_str(&format!("- P95: {:?}\n", comp.ace_result.stats.p95));
        report.push_str(&format!("- P99: {:?}\n", comp.ace_result.stats.p99));
        report.push_str(&format!("- CV: {:.2}%\n", comp.ace_result.stats.coefficient_of_variation()));
        
        if let Some(chrome) = &comp.chrome_result {
            report.push_str("\n**Chrome HTML Parser:**\n");
            report.push_str(&format!("- Mean: {:?}\n", chrome.mean));
            report.push_str(&format!("- Median: {:?}\n", chrome.median));
            report.push_str(&format!("- P95: {:?}\n", chrome.p95));
            report.push_str(&format!("- P99: {:?}\n", chrome.p99));
            report.push_str(&format!("- Throughput: {:.2} MB/s\n", chrome.throughput_mbps));
        }
        
        report.push_str("\n");
    }
    
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chrome_bench_runner_creation() {
        let runner = ChromeBenchRunner::new();
        assert_eq!(runner.node_script_path, "benchmarks/chrome_parser_bench.js");
    }
    
    #[test]
    fn test_parser_comparison_creation() {
        let html = "<div>test</div>";
        let config = BenchConfig::new("test").with_warmup(1).with_measurements(5);
        let runner = BenchRunner::new(config);
        let ace_result = runner.run(|| {
            let _doc = build_document(html);
        });
        
        let comparison = ParserComparison::new("test".to_string(), ace_result, None);
        assert_eq!(comparison.name, "test");
        assert!(comparison.speedup.is_none());
    }
    
    #[test]
    fn test_comparison_report_generation() {
        let html = "<div>test</div>";
        let config = BenchConfig::new("test").with_warmup(1).with_measurements(5);
        let runner = BenchRunner::new(config);
        let ace_result = runner.run(|| {
            let _doc = build_document(html);
        });
        
        let comparison = ParserComparison::new("test".to_string(), ace_result, None);
        let report = generate_comparison_report(&[comparison]);
        
        assert!(report.contains("# ACE HTML Parser vs Chrome Comparison"));
        assert!(report.contains("test"));
    }
}
