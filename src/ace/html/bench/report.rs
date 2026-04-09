//! Report generation for benchmark results (HTML and JSON)
//! 
//! This module provides comprehensive report generation for benchmarks including:
//! - HTML reports with rich formatting and visualization
//! - JSON export for programmatic access and CI/CD integration
//! - Historical tracking and comparison
//! - Optional charts and graphs

use super::runner::BenchResult;
use std::fs::File;
use std::io::{Write, Read};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Html,
    Json,
    HtmlWithCharts, // Enhanced HTML with embedded charts
}

/// Historical benchmark data for tracking over time
#[derive(Debug, Clone)]
pub struct HistoricalData {
    pub timestamp: u64,
    pub results: Vec<BenchResult>,
}

impl HistoricalData {
    pub fn new(results: Vec<BenchResult>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self { timestamp, results }
    }
    
    pub fn from_timestamp(timestamp: u64, results: Vec<BenchResult>) -> Self {
        Self { timestamp, results }
    }
}

/// Report generator with historical tracking
pub struct ReportGenerator {
    results: Vec<BenchResult>,
    historical: Vec<HistoricalData>,
    title: String,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            historical: Vec::new(),
            title: "ACE HTML Parser Benchmarks".to_string(),
        }
    }
    
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    
    pub fn add_result(&mut self, result: BenchResult) {
        self.results.push(result);
    }
    
    pub fn add_historical(&mut self, data: HistoricalData) {
        self.historical.push(data);
    }
    
    /// Load historical data from JSON file
    pub fn load_historical(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        // Parse historical data from JSON
        if let Ok(data) = self.parse_historical_json(&contents) {
            self.historical = data;
        }
        
        Ok(())
    }
    
    /// Save current results as historical data
    pub fn save_historical(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut all_historical = self.historical.clone();
        all_historical.push(HistoricalData::new(self.results.clone()));
        
        let json = self.generate_historical_json(&all_historical);
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        
        Ok(())
    }
    
    /// Generate report to file
    pub fn generate(&self, path: impl AsRef<Path>, format: ReportFormat) -> std::io::Result<()> {
        match format {
            ReportFormat::Html => self.generate_html(path),
            ReportFormat::Json => self.generate_json(path),
            ReportFormat::HtmlWithCharts => self.generate_html_with_charts(path),
        }
    }
    
    fn generate_html(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        writeln!(file, "<!DOCTYPE html>")?;
        writeln!(file, "<html lang=\"en\">")?;
        writeln!(file, "<head>")?;
        writeln!(file, "  <meta charset=\"UTF-8\">")?;
        writeln!(file, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
        writeln!(file, "  <title>{}</title>", self.title)?;
        writeln!(file, "  <style>")?;
        writeln!(file, "{}", Self::html_css())?;
        writeln!(file, "  </style>")?;
        writeln!(file, "</head>")?;
        writeln!(file, "<body>")?;
        writeln!(file, "  <div class=\"container\">")?;
        writeln!(file, "    <h1>{}</h1>", self.title)?;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        writeln!(file, "    <p class=\"timestamp\">Generated: {}</p>", Self::format_timestamp(timestamp))?;
        
        // Summary section
        if !self.results.is_empty() {
            writeln!(file, "    <div class=\"summary\">")?;
            writeln!(file, "      <h2>Summary</h2>")?;
            writeln!(file, "      <div class=\"summary-grid\">")?;
            writeln!(file, "        <div class=\"summary-card\">")?;
            writeln!(file, "          <div class=\"summary-value\">{}</div>", self.results.len())?;
            writeln!(file, "          <div class=\"summary-label\">Total Benchmarks</div>")?;
            writeln!(file, "        </div>")?;
            
            let avg_mean = self.results.iter()
                .map(|r| r.stats.mean.as_secs_f64())
                .sum::<f64>() / self.results.len() as f64;
            writeln!(file, "        <div class=\"summary-card\">")?;
            writeln!(file, "          <div class=\"summary-value\">{:.2}ms</div>", avg_mean * 1000.0)?;
            writeln!(file, "          <div class=\"summary-label\">Average Mean</div>")?;
            writeln!(file, "        </div>")?;
            
            let regressions = self.results.iter()
                .filter(|r| r.comparison.as_ref().map_or(false, |c| c.is_regression))
                .count();
            writeln!(file, "        <div class=\"summary-card\">")?;
            writeln!(file, "          <div class=\"summary-value\">{}</div>", regressions)?;
            writeln!(file, "          <div class=\"summary-label\">Regressions</div>")?;
            writeln!(file, "        </div>")?;
            writeln!(file, "      </div>")?;
            writeln!(file, "    </div>")?;
        }
        
        // Results table
        writeln!(file, "    <div class=\"results\">")?;
        writeln!(file, "      <h2>Benchmark Results</h2>")?;
        
        for result in &self.results {
            self.write_html_result(&mut file, result)?;
        }
        
        writeln!(file, "    </div>")?;
        
        // Historical trends
        if !self.historical.is_empty() {
            writeln!(file, "    <div class=\"historical\">")?;
            writeln!(file, "      <h2>Historical Trends</h2>")?;
            self.write_historical_section(&mut file)?;
            writeln!(file, "    </div>")?;
        }
        
        writeln!(file, "  </div>")?;
        writeln!(file, "</body>")?;
        writeln!(file, "</html>")?;
        
        Ok(())
    }
    
    fn write_html_result(&self, file: &mut File, result: &BenchResult) -> std::io::Result<()> {
        writeln!(file, "      <div class=\"benchmark\">")?;
        writeln!(file, "        <h3>{}</h3>", result.name)?;
        writeln!(file, "        <div class=\"metrics-grid\">")?;
        
        // Main metrics
        writeln!(file, "          <div class=\"metric\">")?;
        writeln!(file, "            <div class=\"metric-label\">Mean</div>")?;
        writeln!(file, "            <div class=\"metric-value\">{:.3}ms</div>", result.stats.mean.as_secs_f64() * 1000.0)?;
        writeln!(file, "          </div>")?;
        
        writeln!(file, "          <div class=\"metric\">")?;
        writeln!(file, "            <div class=\"metric-label\">Median</div>")?;
        writeln!(file, "            <div class=\"metric-value\">{:.3}ms</div>", result.stats.median.as_secs_f64() * 1000.0)?;
        writeln!(file, "          </div>")?;
        
        writeln!(file, "          <div class=\"metric\">")?;
        writeln!(file, "            <div class=\"metric-label\">P95</div>")?;
        writeln!(file, "            <div class=\"metric-value\">{:.3}ms</div>", result.stats.p95.as_secs_f64() * 1000.0)?;
        writeln!(file, "          </div>")?;
        
        writeln!(file, "          <div class=\"metric\">")?;
        writeln!(file, "            <div class=\"metric-label\">P99</div>")?;
        writeln!(file, "            <div class=\"metric-value\">{:.3}ms</div>", result.stats.p99.as_secs_f64() * 1000.0)?;
        writeln!(file, "          </div>")?;
        
        writeln!(file, "        </div>")?;
        
        // Detailed table
        writeln!(file, "        <table class=\"details\">")?;
        writeln!(file, "          <tr><th>Metric</th><th>Value</th></tr>")?;
        writeln!(file, "          <tr><td>Std Dev</td><td>{:.3}ms</td></tr>", result.stats.std_dev.as_secs_f64() * 1000.0)?;
        writeln!(file, "          <tr><td>Min</td><td>{:.3}ms</td></tr>", result.stats.min.as_secs_f64() * 1000.0)?;
        writeln!(file, "          <tr><td>Max</td><td>{:.3}ms</td></tr>", result.stats.max.as_secs_f64() * 1000.0)?;
        writeln!(file, "          <tr><td>CV</td><td>{:.2}%</td></tr>", result.stats.coefficient_of_variation())?;
        writeln!(file, "          <tr><td>Outliers</td><td>{} ({:.1}%)</td></tr>", 
            result.stats.outliers.len(),
            result.stats.outlier_percentage()
        )?;
        
        if let Some(comp) = &result.comparison {
            writeln!(file, "          <tr class=\"comparison-header\"><td colspan=\"2\"><strong>Baseline Comparison</strong></td></tr>")?;
            writeln!(file, "          <tr><td>Baseline</td><td>{:.3}ms</td></tr>", comp.baseline_mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "          <tr><td>Current</td><td>{:.3}ms</td></tr>", comp.current_mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "          <tr><td>Speedup</td><td>{:.2}x</td></tr>", comp.speedup)?;
            writeln!(file, "          <tr><td>Change</td><td>{:+.2}%</td></tr>", comp.percentage_change())?;
            
            let status_class = if comp.is_regression { "regression" } else { "ok" };
            let status_text = if comp.is_regression { "⚠ REGRESSION" } else { "✓ OK" };
            writeln!(file, "          <tr class=\"{}\"><td>Status</td><td>{}</td></tr>", status_class, status_text)?;
        }
        
        writeln!(file, "        </table>")?;
        writeln!(file, "      </div>")?;
        
        Ok(())
    }
    
    fn html_css() -> &'static str {
        r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
    padding: 20px;
    color: #333;
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

.timestamp {
    color: rgba(255,255,255,0.9);
    font-size: 0.9em;
    margin-bottom: 30px;
}

.summary {
    background: white;
    border-radius: 12px;
    padding: 30px;
    margin-bottom: 30px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.2);
}

.summary h2 {
    color: #667eea;
    margin-bottom: 20px;
    font-size: 1.8em;
}

.summary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 20px;
}

.summary-card {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 8px;
    padding: 20px;
    text-align: center;
    color: white;
}

.summary-value {
    font-size: 2.5em;
    font-weight: bold;
    margin-bottom: 5px;
}

.summary-label {
    font-size: 0.9em;
    opacity: 0.9;
}

.results h2 {
    color: white;
    margin-bottom: 20px;
    font-size: 1.8em;
}

.benchmark {
    background: white;
    border-radius: 12px;
    padding: 30px;
    margin-bottom: 20px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.2);
    transition: transform 0.2s, box-shadow 0.2s;
}

.benchmark:hover {
    transform: translateY(-2px);
    box-shadow: 0 15px 40px rgba(0,0,0,0.3);
}

.benchmark h3 {
    color: #667eea;
    margin-bottom: 20px;
    font-size: 1.5em;
    border-bottom: 2px solid #667eea;
    padding-bottom: 10px;
}

.metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 15px;
    margin-bottom: 20px;
}

.metric {
    background: #f8f9fa;
    border-radius: 8px;
    padding: 15px;
    text-align: center;
    border-left: 4px solid #667eea;
}

.metric-label {
    font-size: 0.85em;
    color: #666;
    margin-bottom: 5px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.metric-value {
    font-size: 1.5em;
    font-weight: bold;
    color: #333;
}

.details {
    width: 100%;
    border-collapse: collapse;
    margin-top: 20px;
}

.details th,
.details td {
    padding: 12px;
    text-align: left;
    border-bottom: 1px solid #eee;
}

.details th {
    background: #f8f9fa;
    font-weight: 600;
    color: #667eea;
    text-transform: uppercase;
    font-size: 0.85em;
    letter-spacing: 0.5px;
}

.details tr:hover {
    background: #f8f9fa;
}

.comparison-header {
    background: #e7f3ff !important;
    font-weight: bold;
}

.regression {
    background: #ffe7e7 !important;
    color: #d32f2f;
    font-weight: bold;
}

.ok {
    background: #e7ffe7 !important;
    color: #2e7d32;
    font-weight: bold;
}

.historical {
    background: white;
    border-radius: 12px;
    padding: 30px;
    margin-top: 30px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.2);
}

.historical h2 {
    color: #667eea;
    margin-bottom: 20px;
    font-size: 1.8em;
}

.historical-table {
    width: 100%;
    border-collapse: collapse;
}

.historical-table th,
.historical-table td {
    padding: 12px;
    text-align: left;
    border-bottom: 1px solid #eee;
}

.historical-table th {
    background: #f8f9fa;
    font-weight: 600;
    color: #667eea;
}

.trend-up {
    color: #2e7d32;
}

.trend-down {
    color: #d32f2f;
}

@media (max-width: 768px) {
    .metrics-grid {
        grid-template-columns: 1fr;
    }
    
    .summary-grid {
        grid-template-columns: 1fr;
    }
}
        "#
    }
    
    fn write_historical_section(&self, file: &mut File) -> std::io::Result<()> {
        if self.historical.is_empty() {
            return Ok(());
        }
        
        writeln!(file, "        <table class=\"historical-table\">")?;
        writeln!(file, "          <tr>")?;
        writeln!(file, "            <th>Timestamp</th>")?;
        writeln!(file, "            <th>Benchmark</th>")?;
        writeln!(file, "            <th>Mean</th>")?;
        writeln!(file, "            <th>Trend</th>")?;
        writeln!(file, "          </tr>")?;
        
        for data in &self.historical {
            for result in &data.results {
                writeln!(file, "          <tr>")?;
                writeln!(file, "            <td>{}</td>", Self::format_timestamp(data.timestamp))?;
                writeln!(file, "            <td>{}</td>", result.name)?;
                writeln!(file, "            <td>{:.3}ms</td>", result.stats.mean.as_secs_f64() * 1000.0)?;
                
                // Calculate trend if we have comparison data
                if let Some(comp) = &result.comparison {
                    let trend_class = if comp.speedup > 1.0 { "trend-up" } else { "trend-down" };
                    let trend_symbol = if comp.speedup > 1.0 { "↑" } else { "↓" };
                    writeln!(file, "            <td class=\"{}\">{}  {:.1}%</td>", 
                        trend_class, trend_symbol, (comp.speedup - 1.0).abs() * 100.0)?;
                } else {
                    writeln!(file, "            <td>-</td>")?;
                }
                
                writeln!(file, "          </tr>")?;
            }
        }
        
        writeln!(file, "        </table>")?;
        
        Ok(())
    }
    
    fn format_timestamp(timestamp: u64) -> String {
        // Simple timestamp formatting (Unix timestamp to readable format)
        // In a real implementation, you'd use chrono or similar, but we're avoiding dependencies
        let days = timestamp / 86400;
        let hours = (timestamp % 86400) / 3600;
        let minutes = (timestamp % 3600) / 60;
        let seconds = timestamp % 60;
        
        format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    }
    
    fn generate_html_with_charts(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        // Start with regular HTML
        writeln!(file, "<!DOCTYPE html>")?;
        writeln!(file, "<html lang=\"en\">")?;
        writeln!(file, "<head>")?;
        writeln!(file, "  <meta charset=\"UTF-8\">")?;
        writeln!(file, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
        writeln!(file, "  <title>{}</title>", self.title)?;
        writeln!(file, "  <style>")?;
        writeln!(file, "{}", Self::html_css())?;
        writeln!(file, "{}", Self::chart_css())?;
        writeln!(file, "  </style>")?;
        writeln!(file, "</head>")?;
        writeln!(file, "<body>")?;
        writeln!(file, "  <div class=\"container\">")?;
        writeln!(file, "    <h1>{}</h1>", self.title)?;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        writeln!(file, "    <p class=\"timestamp\">Generated: {}</p>", Self::format_timestamp(timestamp))?;
        
        // Add charts section
        if !self.results.is_empty() {
            writeln!(file, "    <div class=\"charts\">")?;
            writeln!(file, "      <h2>Performance Visualization</h2>")?;
            self.write_charts(&mut file)?;
            writeln!(file, "    </div>")?;
        }
        
        // Regular results section
        writeln!(file, "    <div class=\"results\">")?;
        writeln!(file, "      <h2>Benchmark Results</h2>")?;
        
        for result in &self.results {
            self.write_html_result(&mut file, result)?;
        }
        
        writeln!(file, "    </div>")?;
        writeln!(file, "  </div>")?;
        writeln!(file, "</body>")?;
        writeln!(file, "</html>")?;
        
        Ok(())
    }
    
    fn chart_css() -> &'static str {
        r#"
.charts {
    background: white;
    border-radius: 12px;
    padding: 30px;
    margin-bottom: 30px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.2);
}

.charts h2 {
    color: #667eea;
    margin-bottom: 20px;
    font-size: 1.8em;
}

.chart-container {
    margin: 20px 0;
    padding: 20px;
    background: #f8f9fa;
    border-radius: 8px;
}

.chart-title {
    font-weight: bold;
    margin-bottom: 15px;
    color: #333;
}

.bar-chart {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.bar-item {
    display: flex;
    align-items: center;
    gap: 10px;
}

.bar-label {
    min-width: 150px;
    font-size: 0.9em;
    color: #666;
}

.bar-container {
    flex: 1;
    height: 30px;
    background: #e0e0e0;
    border-radius: 4px;
    position: relative;
    overflow: hidden;
}

.bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #667eea 0%, #764ba2 100%);
    border-radius: 4px;
    transition: width 0.3s ease;
}

.bar-value {
    min-width: 80px;
    text-align: right;
    font-size: 0.9em;
    font-weight: bold;
    color: #333;
}
        "#
    }
    
    fn write_charts(&self, file: &mut File) -> std::io::Result<()> {
        // Find max mean for scaling
        let max_mean = self.results.iter()
            .map(|r| r.stats.mean.as_secs_f64())
            .fold(0.0f64, |a, b| a.max(b));
        
        if max_mean == 0.0 {
            return Ok(());
        }
        
        writeln!(file, "      <div class=\"chart-container\">")?;
        writeln!(file, "        <div class=\"chart-title\">Mean Execution Time Comparison</div>")?;
        writeln!(file, "        <div class=\"bar-chart\">")?;
        
        for result in &self.results {
            let mean_ms = result.stats.mean.as_secs_f64() * 1000.0;
            let percentage = (result.stats.mean.as_secs_f64() / max_mean * 100.0).min(100.0);
            
            writeln!(file, "          <div class=\"bar-item\">")?;
            writeln!(file, "            <div class=\"bar-label\">{}</div>", result.name)?;
            writeln!(file, "            <div class=\"bar-container\">")?;
            writeln!(file, "              <div class=\"bar-fill\" style=\"width: {:.1}%\"></div>", percentage)?;
            writeln!(file, "            </div>")?;
            writeln!(file, "            <div class=\"bar-value\">{:.3}ms</div>", mean_ms)?;
            writeln!(file, "          </div>")?;
        }
        
        writeln!(file, "        </div>")?;
        writeln!(file, "      </div>")?;
        
        Ok(())
    }
    
    fn generate_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        writeln!(file, "{{")?;
        writeln!(file, "  \"title\": \"{}\",", self.title)?;
        writeln!(file, "  \"timestamp\": {},", timestamp)?;
        writeln!(file, "  \"benchmarks\": [")?;
        
        for (i, result) in self.results.iter().enumerate() {
            writeln!(file, "    {{")?;
            writeln!(file, "      \"name\": \"{}\",", result.name)?;
            writeln!(file, "      \"statistics\": {{")?;
            writeln!(file, "        \"mean_ns\": {},", result.stats.mean.as_nanos())?;
            writeln!(file, "        \"mean_ms\": {},", result.stats.mean.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"median_ns\": {},", result.stats.median.as_nanos())?;
            writeln!(file, "        \"median_ms\": {},", result.stats.median.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"std_dev_ns\": {},", result.stats.std_dev.as_nanos())?;
            writeln!(file, "        \"std_dev_ms\": {},", result.stats.std_dev.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"p95_ns\": {},", result.stats.p95.as_nanos())?;
            writeln!(file, "        \"p95_ms\": {},", result.stats.p95.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"p99_ns\": {},", result.stats.p99.as_nanos())?;
            writeln!(file, "        \"p99_ms\": {},", result.stats.p99.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"min_ns\": {},", result.stats.min.as_nanos())?;
            writeln!(file, "        \"min_ms\": {},", result.stats.min.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"max_ns\": {},", result.stats.max.as_nanos())?;
            writeln!(file, "        \"max_ms\": {},", result.stats.max.as_secs_f64() * 1000.0)?;
            writeln!(file, "        \"cv_percent\": {},", result.stats.coefficient_of_variation())?;
            writeln!(file, "        \"outlier_count\": {},", result.stats.outliers.len())?;
            writeln!(file, "        \"outlier_percent\": {}", result.stats.outlier_percentage())?;
            writeln!(file, "      }}")?;
            
            if let Some(comp) = &result.comparison {
                writeln!(file, "      ,\"comparison\": {{")?;
                writeln!(file, "        \"baseline_mean_ns\": {},", comp.baseline_mean.as_nanos())?;
                writeln!(file, "        \"baseline_mean_ms\": {},", comp.baseline_mean.as_secs_f64() * 1000.0)?;
                writeln!(file, "        \"current_mean_ns\": {},", comp.current_mean.as_nanos())?;
                writeln!(file, "        \"current_mean_ms\": {},", comp.current_mean.as_secs_f64() * 1000.0)?;
                writeln!(file, "        \"speedup\": {},", comp.speedup)?;
                writeln!(file, "        \"change_percent\": {},", comp.percentage_change())?;
                writeln!(file, "        \"is_regression\": {}", comp.is_regression)?;
                writeln!(file, "      }}")?;
            }
            
            if i < self.results.len() - 1 {
                writeln!(file, "    }},")?;
            } else {
                writeln!(file, "    }}")?;
            }
        }
        
        writeln!(file, "  ]")?;
        
        // Add historical data if available
        if !self.historical.is_empty() {
            writeln!(file, "  ,\"historical\": [")?;
            
            for (i, data) in self.historical.iter().enumerate() {
                writeln!(file, "    {{")?;
                writeln!(file, "      \"timestamp\": {},", data.timestamp)?;
                writeln!(file, "      \"results\": [")?;
                
                for (j, result) in data.results.iter().enumerate() {
                    writeln!(file, "        {{")?;
                    writeln!(file, "          \"name\": \"{}\",", result.name)?;
                    writeln!(file, "          \"mean_ms\": {}", result.stats.mean.as_secs_f64() * 1000.0)?;
                    
                    if j < data.results.len() - 1 {
                        writeln!(file, "        }},")?;
                    } else {
                        writeln!(file, "        }}")?;
                    }
                }
                
                writeln!(file, "      ]")?;
                
                if i < self.historical.len() - 1 {
                    writeln!(file, "    }},")?;
                } else {
                    writeln!(file, "    }}")?;
                }
            }
            
            writeln!(file, "  ]")?;
        }
        
        writeln!(file, "}}")?;
        
        Ok(())
    }
    
    fn generate_historical_json(&self, historical: &[HistoricalData]) -> String {
        let mut json = String::from("[\n");
        
        for (i, data) in historical.iter().enumerate() {
            json.push_str("  {\n");
            json.push_str(&format!("    \"timestamp\": {},\n", data.timestamp));
            json.push_str("    \"results\": [\n");
            
            for (j, result) in data.results.iter().enumerate() {
                json.push_str("      {\n");
                json.push_str(&format!("        \"name\": \"{}\",\n", result.name));
                json.push_str(&format!("        \"mean_ms\": {}\n", result.stats.mean.as_secs_f64() * 1000.0));
                
                if j < data.results.len() - 1 {
                    json.push_str("      },\n");
                } else {
                    json.push_str("      }\n");
                }
            }
            
            json.push_str("    ]\n");
            
            if i < historical.len() - 1 {
                json.push_str("  },\n");
            } else {
                json.push_str("  }\n");
            }
        }
        
        json.push_str("]\n");
        json
    }
    
    fn parse_historical_json(&self, _json: &str) -> Result<Vec<HistoricalData>, String> {
        // Simple JSON parsing for historical data
        // In a production system, you'd want more robust parsing
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::stats::BenchStats;
    use std::time::Duration;
    
    #[test]
    fn test_report_generation() {
        let samples = vec![Duration::from_millis(10); 10];
        let stats = BenchStats::from_samples(samples);
        
        let result = BenchResult {
            name: "test_benchmark".to_string(),
            stats,
            comparison: None,
        };
        
        let mut generator = ReportGenerator::new();
        generator.add_result(result);
        
        // Test JSON generation
        let json_path = "target/test_report.json";
        generator.generate(json_path, ReportFormat::Json).unwrap();
        assert!(Path::new(json_path).exists());
        
        // Test HTML generation
        let html_path = "target/test_report.html";
        generator.generate(html_path, ReportFormat::Html).unwrap();
        assert!(Path::new(html_path).exists());
        
        // Test HTML with charts generation
        let html_charts_path = "target/test_report_charts.html";
        generator.generate(html_charts_path, ReportFormat::HtmlWithCharts).unwrap();
        assert!(Path::new(html_charts_path).exists());
    }
    
    #[test]
    fn test_historical_tracking() {
        let samples = vec![Duration::from_millis(10); 10];
        let stats = BenchStats::from_samples(samples);
        
        let result = BenchResult {
            name: "test_benchmark".to_string(),
            stats,
            comparison: None,
        };
        
        let mut generator = ReportGenerator::new();
        generator.add_result(result.clone());
        
        // Add historical data
        let historical = HistoricalData::new(vec![result]);
        generator.add_historical(historical);
        
        // Test saving historical data
        let historical_path = "target/test_historical.json";
        generator.save_historical(historical_path).unwrap();
        assert!(Path::new(historical_path).exists());
    }
    
    #[test]
    fn test_custom_title() {
        let generator = ReportGenerator::new()
            .with_title("Custom Benchmark Report");
        
        assert_eq!(generator.title, "Custom Benchmark Report");
    }
}
