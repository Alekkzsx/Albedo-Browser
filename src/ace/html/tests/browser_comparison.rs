//! Side-by-Side Browser Comparison
//! 
//! This module provides infrastructure to benchmark ACE HTML parser against
//! both Chrome and Firefox parsers simultaneously, generating comprehensive
//! comparison reports.

use crate::ace::html::{build_document, bench::{BenchRunner, BenchConfig, BenchResult}};
use crate::ace::html::tests::chrome_bench::{ChromeBenchRunner, ChromeBenchResult};
use crate::ace::html::tests::firefox_bench::{FirefoxBenchRunner, FirefoxBenchResult};

/// Unified comparison result for ACE vs Chrome vs Firefox
#[derive(Debug, Clone)]
pub struct BrowserComparison {
    pub name: String,
    pub ace_result: BenchResult,
    pub chrome_result: Option<ChromeBenchResult>,
    pub firefox_result: Option<FirefoxBenchResult>,
    pub ace_vs_chrome_speedup: Option<f64>,
    pub ace_vs_firefox_speedup: Option<f64>,
}

impl BrowserComparison {
    pub fn new(
        name: String,
        ace_result: BenchResult,
        chrome_result: Option<ChromeBenchResult>,
        firefox_result: Option<FirefoxBenchResult>,
    ) -> Self {
        let ace_vs_chrome_speedup = chrome_result.as_ref().map(|chrome| {
            let ace_secs = ace_result.stats.mean.as_secs_f64();
            let chrome_secs = chrome.mean.as_secs_f64();
            if ace_secs > 0.0 {
                chrome_secs / ace_secs
            } else {
                1.0
            }
        });
        
        let ace_vs_firefox_speedup = firefox_result.as_ref().map(|firefox| {
            let ace_secs = ace_result.stats.mean.as_secs_f64();
            let firefox_secs = firefox.mean.as_secs_f64();
            if ace_secs > 0.0 {
                firefox_secs / ace_secs
            } else {
                1.0
            }
        });
        
        Self {
            name,
            ace_result,
            chrome_result,
            firefox_result,
            ace_vs_chrome_speedup,
            ace_vs_firefox_speedup,
        }
    }
    
    pub fn print_comparison(&self) {
        println!("\n{}", "=".repeat(80));
        println!("Benchmark: {}", self.name);
        println!("{}", "=".repeat(80));
        
        println!("\n📊 ACE HTML Parser:");
        println!("  Mean:       {:?}", self.ace_result.stats.mean);
        println!("  Median:     {:?}", self.ace_result.stats.median);
        println!("  P95:        {:?}", self.ace_result.stats.p95);
        println!("  P99:        {:?}", self.ace_result.stats.p99);
        println!("  CV:         {:.2}%", self.ace_result.stats.coefficient_of_variation());
        
        if let Some(chrome) = &self.chrome_result {
            println!("\n🌐 Chrome HTML Parser:");
            println!("  Mean:       {:?}", chrome.mean);
            println!("  Median:     {:?}", chrome.median);
            println!("  P95:        {:?}", chrome.p95);
            println!("  P99:        {:?}", chrome.p99);
            println!("  Throughput: {:.2} MB/s", chrome.throughput_mbps);
        } else {
            println!("\n🌐 Chrome: Not available");
        }
        
        if let Some(firefox) = &self.firefox_result {
            println!("\n🦊 Firefox HTML Parser:");
            println!("  Mean:       {:?}", firefox.mean);
            println!("  Median:     {:?}", firefox.median);
            println!("  P95:        {:?}", firefox.p95);
            println!("  P99:        {:?}", firefox.p99);
            println!("  Throughput: {:.2} MB/s", firefox.throughput_mbps);
        } else {
            println!("\n🦊 Firefox: Not available");
        }
        
        println!("\n📈 Comparison Summary:");
        
        if let Some(speedup) = self.ace_vs_chrome_speedup {
            if speedup > 1.0 {
                println!("  ACE vs Chrome:   ✓ {:.2}x faster ({:.1}% improvement)", 
                    speedup, (speedup - 1.0) * 100.0);
            } else {
                println!("  ACE vs Chrome:   ⚠ {:.2}x slower ({:.1}% behind)", 
                    1.0 / speedup, (1.0 - speedup) * 100.0);
            }
        }
        
        if let Some(speedup) = self.ace_vs_firefox_speedup {
            if speedup > 1.0 {
                println!("  ACE vs Firefox:  ✓ {:.2}x faster ({:.1}% improvement)", 
                    speedup, (speedup - 1.0) * 100.0);
            } else {
                println!("  ACE vs Firefox:  ⚠ {:.2}x slower ({:.1}% behind)", 
                    1.0 / speedup, (1.0 - speedup) * 100.0);
            }
        }
        
        println!("{}", "=".repeat(80));
    }
}

/// Unified browser comparison runner
pub struct BrowserComparisonRunner {
    chrome_runner: ChromeBenchRunner,
    firefox_runner: FirefoxBenchRunner,
}

impl BrowserComparisonRunner {
    pub fn new() -> Self {
        Self {
            chrome_runner: ChromeBenchRunner::new(),
            firefox_runner: FirefoxBenchRunner::new(),
        }
    }
    
    /// Check which browsers are available for benchmarking
    pub fn check_availability(&self) -> BrowserAvailability {
        BrowserAvailability {
            chrome: self.chrome_runner.is_available(),
            firefox: self.firefox_runner.is_available(),
        }
    }
    
    /// Run comprehensive comparison: ACE vs Chrome vs Firefox
    pub fn compare_all_parsers(&self, html: &str, name: &str) -> BrowserComparison {
        println!("\n🔬 Running comprehensive browser comparison: {}", name);
        println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
        
        let availability = self.check_availability();
        println!("\n📋 Browser availability:");
        println!("  Chrome:  {}", if availability.chrome { "✓" } else { "✗" });
        println!("  Firefox: {}", if availability.firefox { "✓" } else { "✗" });
        
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
        let chrome_result = if availability.chrome {
            println!("\n⏱️  Benchmarking Chrome HTML Parser...");
            match self.chrome_runner.run_chrome_benchmark(html, name) {
                Ok(result) => {
                    println!("  Chrome Mean: {:?}", result.mean);
                    Some(result)
                }
                Err(e) => {
                    println!("  ⚠️  Chrome benchmark failed: {}", e);
                    None
                }
            }
        } else {
            println!("\n⏱️  Chrome benchmark skipped (not available)");
            None
        };
        
        // Run Firefox benchmark
        let firefox_result = if availability.firefox {
            println!("\n⏱️  Benchmarking Firefox HTML Parser...");
            match self.firefox_runner.run_firefox_benchmark(html, name) {
                Ok(result) => {
                    println!("  Firefox Mean: {:?}", result.mean);
                    Some(result)
                }
                Err(e) => {
                    println!("  ⚠️  Firefox benchmark failed: {}", e);
                    None
                }
            }
        } else {
            println!("\n⏱️  Firefox benchmark skipped (not available)");
            None
        };
        
        BrowserComparison::new(name.to_string(), ace_result, chrome_result, firefox_result)
    }
}

impl Default for BrowserComparisonRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Browser availability status
#[derive(Debug, Clone, Copy)]
pub struct BrowserAvailability {
    pub chrome: bool,
    pub firefox: bool,
}

impl BrowserAvailability {
    pub fn any_available(&self) -> bool {
        self.chrome || self.firefox
    }
    
    pub fn all_available(&self) -> bool {
        self.chrome && self.firefox
    }
}

/// Generate comprehensive comparison report
pub fn generate_comprehensive_report(comparisons: &[BrowserComparison]) -> String {
    let mut report = String::from("# ACE HTML Parser - Comprehensive Browser Comparison\n\n");
    
    // Summary table
    report.push_str("## Performance Comparison Summary\n\n");
    report.push_str("| Benchmark | ACE Mean | Chrome Mean | Firefox Mean | vs Chrome | vs Firefox |\n");
    report.push_str("|-----------|----------|-------------|--------------|-----------|------------|\n");
    
    for comp in comparisons {
        let ace_ms = comp.ace_result.stats.mean.as_secs_f64() * 1000.0;
        
        let chrome_str = comp.chrome_result.as_ref()
            .map(|c| format!("{:.2}ms", c.mean.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "N/A".to_string());
        
        let firefox_str = comp.firefox_result.as_ref()
            .map(|f| format!("{:.2}ms", f.mean.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "N/A".to_string());
        
        let vs_chrome = comp.ace_vs_chrome_speedup
            .map(|s| {
                if s > 1.0 {
                    format!("✓ {:.2}x", s)
                } else {
                    format!("⚠ {:.2}x", s)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());
        
        let vs_firefox = comp.ace_vs_firefox_speedup
            .map(|s| {
                if s > 1.0 {
                    format!("✓ {:.2}x", s)
                } else {
                    format!("⚠ {:.2}x", s)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());
        
        report.push_str(&format!(
            "| {} | {:.2}ms | {} | {} | {} | {} |\n",
            comp.name, ace_ms, chrome_str, firefox_str, vs_chrome, vs_firefox
        ));
    }
    
    // Detailed statistics
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
        
        if let Some(firefox) = &comp.firefox_result {
            report.push_str("\n**Firefox HTML Parser:**\n");
            report.push_str(&format!("- Mean: {:?}\n", firefox.mean));
            report.push_str(&format!("- Median: {:?}\n", firefox.median));
            report.push_str(&format!("- P95: {:?}\n", firefox.p95));
            report.push_str(&format!("- P99: {:?}\n", firefox.p99));
            report.push_str(&format!("- Throughput: {:.2} MB/s\n", firefox.throughput_mbps));
        }
        
        if let Some(speedup) = comp.ace_vs_chrome_speedup {
            report.push_str(&format!("\n**ACE vs Chrome:** {:.2}x ", speedup));
            if speedup > 1.0 {
                report.push_str(&format!("(ACE is {:.1}% faster)\n", (speedup - 1.0) * 100.0));
            } else {
                report.push_str(&format!("(ACE is {:.1}% slower)\n", (1.0 - speedup) * 100.0));
            }
        }
        
        if let Some(speedup) = comp.ace_vs_firefox_speedup {
            report.push_str(&format!("**ACE vs Firefox:** {:.2}x ", speedup));
            if speedup > 1.0 {
                report.push_str(&format!("(ACE is {:.1}% faster)\n", (speedup - 1.0) * 100.0));
            } else {
                report.push_str(&format!("(ACE is {:.1}% slower)\n", (1.0 - speedup) * 100.0));
            }
        }
        
        report.push_str("\n");
    }
    
    // Performance insights
    report.push_str("## Performance Insights\n\n");
    
    let avg_chrome_speedup = comparisons.iter()
        .filter_map(|c| c.ace_vs_chrome_speedup)
        .sum::<f64>() / comparisons.iter().filter(|c| c.ace_vs_chrome_speedup.is_some()).count() as f64;
    
    let avg_firefox_speedup = comparisons.iter()
        .filter_map(|c| c.ace_vs_firefox_speedup)
        .sum::<f64>() / comparisons.iter().filter(|c| c.ace_vs_firefox_speedup.is_some()).count() as f64;
    
    if avg_chrome_speedup.is_finite() {
        report.push_str(&format!("- **Average ACE vs Chrome:** {:.2}x\n", avg_chrome_speedup));
    }
    
    if avg_firefox_speedup.is_finite() {
        report.push_str(&format!("- **Average ACE vs Firefox:** {:.2}x\n", avg_firefox_speedup));
    }
    
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_browser_comparison_runner_creation() {
        let runner = BrowserComparisonRunner::new();
        let availability = runner.check_availability();
        
        // Just check that we can create the runner
        println!("Chrome available: {}", availability.chrome);
        println!("Firefox available: {}", availability.firefox);
    }
    
    #[test]
    fn test_browser_comparison_simple() {
        let runner = BrowserComparisonRunner::new();
        let html = "<div>test</div>";
        
        let comparison = runner.compare_all_parsers(html, "simple_test");
        
        assert_eq!(comparison.name, "simple_test");
        assert!(comparison.ace_result.stats.mean.as_secs_f64() > 0.0);
    }
    
    #[test]
    fn test_comprehensive_report_generation() {
        let runner = BrowserComparisonRunner::new();
        let html = "<div>test</div>";
        
        let comparison = runner.compare_all_parsers(html, "test");
        let report = generate_comprehensive_report(&[comparison]);
        
        assert!(report.contains("# ACE HTML Parser - Comprehensive Browser Comparison"));
        assert!(report.contains("test"));
    }
}


/// Generate HTML report for browser comparison
pub fn generate_html_report(comparisons: &[BrowserComparison], path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create(path)?;
    
    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html lang=\"en\">")?;
    writeln!(file, "<head>")?;
    writeln!(file, "  <meta charset=\"UTF-8\">")?;
    writeln!(file, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
    writeln!(file, "  <title>ACE HTML Parser - Browser Comparison</title>")?;
    writeln!(file, "  <style>{}</style>", browser_comparison_css())?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;
    writeln!(file, "  <div class=\"container\">")?;
    writeln!(file, "    <h1>🚀 ACE HTML Parser - Browser Comparison</h1>")?;
    writeln!(file, "    <p class=\"subtitle\">Performance comparison against Chrome and Firefox parsers</p>")?;
    
    // Summary cards
    writeln!(file, "    <div class=\"summary-grid\">")?;
    
    let total = comparisons.len();
    writeln!(file, "      <div class=\"summary-card\">")?;
    writeln!(file, "        <div class=\"card-value\">{}</div>", total)?;
    writeln!(file, "        <div class=\"card-label\">Total Benchmarks</div>")?;
    writeln!(file, "      </div>")?;
    
    let chrome_count = comparisons.iter().filter(|c| c.chrome_result.is_some()).count();
    if chrome_count > 0 {
        let avg_speedup = comparisons.iter()
            .filter_map(|c| c.ace_vs_chrome_speedup)
            .sum::<f64>() / chrome_count as f64;
        
        writeln!(file, "      <div class=\"summary-card chrome\">")?;
        writeln!(file, "        <div class=\"card-value\">{:.2}x</div>", avg_speedup)?;
        writeln!(file, "        <div class=\"card-label\">vs Chrome (avg)</div>")?;
        writeln!(file, "      </div>")?;
    }
    
    let firefox_count = comparisons.iter().filter(|c| c.firefox_result.is_some()).count();
    if firefox_count > 0 {
        let avg_speedup = comparisons.iter()
            .filter_map(|c| c.ace_vs_firefox_speedup)
            .sum::<f64>() / firefox_count as f64;
        
        writeln!(file, "      <div class=\"summary-card firefox\">")?;
        writeln!(file, "        <div class=\"card-value\">{:.2}x</div>", avg_speedup)?;
        writeln!(file, "        <div class=\"card-label\">vs Firefox (avg)</div>")?;
        writeln!(file, "      </div>")?;
    }
    
    writeln!(file, "    </div>")?;
    
    // Detailed results
    writeln!(file, "    <div class=\"results\">")?;
    
    for comp in comparisons {
        writeln!(file, "      <div class=\"benchmark-card\">")?;
        writeln!(file, "        <h2>{}</h2>", comp.name)?;
        
        writeln!(file, "        <div class=\"parser-grid\">")?;
        
        // ACE results
        writeln!(file, "          <div class=\"parser-result ace\">")?;
        writeln!(file, "            <h3>ACE HTML Parser</h3>")?;
        writeln!(file, "            <div class=\"metric-row\">")?;
        writeln!(file, "              <span class=\"metric-label\">Mean:</span>")?;
        writeln!(file, "              <span class=\"metric-value\">{:.3}ms</span>", comp.ace_result.stats.mean.as_secs_f64() * 1000.0)?;
        writeln!(file, "            </div>")?;
        writeln!(file, "            <div class=\"metric-row\">")?;
        writeln!(file, "              <span class=\"metric-label\">P95:</span>")?;
        writeln!(file, "              <span class=\"metric-value\">{:.3}ms</span>", comp.ace_result.stats.p95.as_secs_f64() * 1000.0)?;
        writeln!(file, "            </div>")?;
        writeln!(file, "          </div>")?;
        
        // Chrome results
        if let Some(chrome) = &comp.chrome_result {
            writeln!(file, "          <div class=\"parser-result chrome\">")?;
            writeln!(file, "            <h3>Chrome Parser</h3>")?;
            writeln!(file, "            <div class=\"metric-row\">")?;
            writeln!(file, "              <span class=\"metric-label\">Mean:</span>")?;
            writeln!(file, "              <span class=\"metric-value\">{:.3}ms</span>", chrome.mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "            </div>")?;
            writeln!(file, "            <div class=\"metric-row\">")?;
            writeln!(file, "              <span class=\"metric-label\">P95:</span>")?;
            writeln!(file, "              <span class=\"metric-value\">{:.3}ms</span>", chrome.p95.as_secs_f64() * 1000.0)?;
            writeln!(file, "            </div>")?;
            
            if let Some(speedup) = comp.ace_vs_chrome_speedup {
                let status_class = if speedup > 1.0 { "faster" } else { "slower" };
                writeln!(file, "            <div class=\"speedup {}\">{:.2}x</div>", status_class, speedup)?;
            }
            
            writeln!(file, "          </div>")?;
        }
        
        // Firefox results
        if let Some(firefox) = &comp.firefox_result {
            writeln!(file, "          <div class=\"parser-result firefox\">")?;
            writeln!(file, "            <h3>Firefox Parser</h3>")?;
            writeln!(file, "            <div class=\"metric-row\">")?;
            writeln!(file, "              <span class=\"metric-label\">Mean:</span>")?;
            writeln!(file, "              <span class=\"metric-value\">{:.3}ms</span>", firefox.mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "            </div>")?;
            writeln!(file, "            <div class=\"metric-row\">")?;
            writeln!(file, "              <span class=\"metric-label\">P95:</span>")?;
            writeln!(file, "              <span class=\"metric-value\">{:.3}ms</span>", firefox.p95.as_secs_f64() * 1000.0)?;
            writeln!(file, "            </div>")?;
            
            if let Some(speedup) = comp.ace_vs_firefox_speedup {
                let status_class = if speedup > 1.0 { "faster" } else { "slower" };
                writeln!(file, "            <div class=\"speedup {}\">{:.2}x</div>", status_class, speedup)?;
            }
            
            writeln!(file, "          </div>")?;
        }
        
        writeln!(file, "        </div>")?;
        writeln!(file, "      </div>")?;
    }
    
    writeln!(file, "    </div>")?;
    writeln!(file, "  </div>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    
    Ok(())
}

fn browser_comparison_css() -> &'static str {
    r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
    padding: 20px;
}

.container {
    max-width: 1400px;
    margin: 0 auto;
}

h1 {
    color: white;
    font-size: 2.5em;
    margin-bottom: 10px;
    text-shadow: 2px 2px 4px rgba(0,0,0,0.2);
}

.subtitle {
    color: rgba(255,255,255,0.9);
    font-size: 1.1em;
    margin-bottom: 30px;
}

.summary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 20px;
    margin-bottom: 30px;
}

.summary-card {
    background: white;
    border-radius: 12px;
    padding: 30px;
    text-align: center;
    box-shadow: 0 10px 30px rgba(0,0,0,0.2);
}

.summary-card.chrome {
    border-top: 4px solid #4285f4;
}

.summary-card.firefox {
    border-top: 4px solid #ff7139;
}

.card-value {
    font-size: 3em;
    font-weight: bold;
    color: #333;
    margin-bottom: 10px;
}

.card-label {
    font-size: 1em;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 1px;
}

.results {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.benchmark-card {
    background: white;
    border-radius: 12px;
    padding: 30px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.2);
}

.benchmark-card h2 {
    color: #667eea;
    margin-bottom: 20px;
    font-size: 1.5em;
    border-bottom: 2px solid #667eea;
    padding-bottom: 10px;
}

.parser-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 20px;
}

.parser-result {
    background: #f8f9fa;
    border-radius: 8px;
    padding: 20px;
    position: relative;
}

.parser-result.ace {
    border-left: 4px solid #667eea;
}

.parser-result.chrome {
    border-left: 4px solid #4285f4;
}

.parser-result.firefox {
    border-left: 4px solid #ff7139;
}

.parser-result h3 {
    font-size: 1.1em;
    margin-bottom: 15px;
    color: #333;
}

.metric-row {
    display: flex;
    justify-content: space-between;
    padding: 8px 0;
    border-bottom: 1px solid #e0e0e0;
}

.metric-label {
    color: #666;
    font-size: 0.9em;
}

.metric-value {
    font-weight: bold;
    color: #333;
}

.speedup {
    margin-top: 15px;
    padding: 10px;
    border-radius: 6px;
    text-align: center;
    font-size: 1.3em;
    font-weight: bold;
}

.speedup.faster {
    background: #e7ffe7;
    color: #2e7d32;
}

.speedup.slower {
    background: #ffe7e7;
    color: #d32f2f;
}

@media (max-width: 768px) {
    .parser-grid {
        grid-template-columns: 1fr;
    }
}
    "#
}

/// Generate JSON report for browser comparison
pub fn generate_json_report(comparisons: &[BrowserComparison], path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let mut file = File::create(path)?;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    writeln!(file, "{{")?;
    writeln!(file, "  \"title\": \"ACE HTML Parser - Browser Comparison\",")?;
    writeln!(file, "  \"timestamp\": {},", timestamp)?;
    writeln!(file, "  \"comparisons\": [")?;
    
    for (i, comp) in comparisons.iter().enumerate() {
        writeln!(file, "    {{")?;
        writeln!(file, "      \"name\": \"{}\",", comp.name)?;
        
        // ACE results
        writeln!(file, "      \"ace\": {{")?;
        writeln!(file, "        \"mean_ms\": {},", comp.ace_result.stats.mean.as_secs_f64() * 1000.0)?;
        writeln!(file, "        \"median_ms\": {},", comp.ace_result.stats.median.as_secs_f64() * 1000.0)?;
        writeln!(file, "        \"p95_ms\": {},", comp.ace_result.stats.p95.as_secs_f64() * 1000.0)?;
        writeln!(file, "        \"p99_ms\": {}", comp.ace_result.stats.p99.as_secs_f64() * 1000.0)?;
        writeln!(file, "      }},")?;
        
        // Chrome results
        if let Some(chrome) = &comp.chrome_result {
            writeln!(file, "      \"chrome\": {{")?;
            writeln!(file, "        \"mean_ms\": {},", chrome.mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"median_ms\": {},", chrome.median.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"p95_ms\": {},", chrome.p95.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"p99_ms\": {},", chrome.p99.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"throughput_mbps\": {}", chrome.throughput_mbps)?;
            writeln!(file, "      }},")?;
        }
        
        // Firefox results
        if let Some(firefox) = &comp.firefox_result {
            writeln!(file, "      \"firefox\": {{")?;
            writeln!(file, "        \"mean_ms\": {},", firefox.mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"median_ms\": {},", firefox.median.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"p95_ms\": {},", firefox.p95.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"p99_ms\": {},", firefox.p99.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"throughput_mbps\": {}", firefox.throughput_mbps)?;
            writeln!(file, "      }},")?;
        }
        
        // Speedup comparisons
        writeln!(file, "      \"speedup\": {{")?;
        if let Some(speedup) = comp.ace_vs_chrome_speedup {
            writeln!(file, "        \"vs_chrome\": {},", speedup)?;
        }
        if let Some(speedup) = comp.ace_vs_firefox_speedup {
            writeln!(file, "        \"vs_firefox\": {}", speedup)?;
        }
        writeln!(file, "      }}")?;
        
        if i < comparisons.len() - 1 {
            writeln!(file, "    }},")?;
        } else {
            writeln!(file, "    }}")?;
        }
    }
    
    writeln!(file, "  ]")?;
    writeln!(file, "}}")?;
    
    Ok(())
}
