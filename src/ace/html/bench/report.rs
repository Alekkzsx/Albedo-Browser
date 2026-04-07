//! Report generation for benchmark results (HTML and JSON)

use super::runner::BenchResult;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Html,
    Json,
}

/// Report generator
pub struct ReportGenerator {
    results: Vec<BenchResult>,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
    
    pub fn add_result(&mut self, result: BenchResult) {
        self.results.push(result);
    }
    
    /// Generate report to file
    pub fn generate(&self, path: impl AsRef<Path>, format: ReportFormat) -> std::io::Result<()> {
        match format {
            ReportFormat::Html => self.generate_html(path),
            ReportFormat::Json => self.generate_json(path),
        }
    }
    
    fn generate_html(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        writeln!(file, "<!DOCTYPE html>")?;
        writeln!(file, "<html>")?;
        writeln!(file, "<head>")?;
        writeln!(file, "  <meta charset=\"UTF-8\">")?;
        writeln!(file, "  <title>ACE HTML Parser Benchmarks</title>")?;
        writeln!(file, "  <style>")?;
        writeln!(file, "{}", Self::html_css())?;
        writeln!(file, "  </style>")?;
        writeln!(file, "</head>")?;
        writeln!(file, "<body>")?;
        writeln!(file, "  <h1>ACE HTML Parser Benchmarks</h1>")?;
        writeln!(file, "  <p>Generated: {}</p>", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs())?;
        
        for result in &self.results {
            self.write_html_result(&mut file, result)?;
        }
        
        writeln!(file, "</body>")?;
        writeln!(file, "</html>")?;
        
        Ok(())
    }
    
    fn write_html_result(&self, file: &mut File, result: &BenchResult) -> std::io::Result<()> {
        writeln!(file, "  <div class=\"benchmark\">")?;
        writeln!(file, "    <h2>{}</h2>", result.name)?;
        writeln!(file, "    <table>")?;
        writeln!(file, "      <tr><th>Metric</th><th>Value</th></tr>")?;
        writeln!(file, "      <tr><td>Mean</td><td>{:?}</td></tr>", result.stats.mean)?;
        writeln!(file, "      <tr><td>Median</td><td>{:?}</td></tr>", result.stats.median)?;
        writeln!(file, "      <tr><td>Std Dev</td><td>{:?}</td></tr>", result.stats.std_dev)?;
        writeln!(file, "      <tr><td>P95</td><td>{:?}</td></tr>", result.stats.p95)?;
        writeln!(file, "      <tr><td>P99</td><td>{:?}</td></tr>", result.stats.p99)?;
        writeln!(file, "      <tr><td>Min</td><td>{:?}</td></tr>", result.stats.min)?;
        writeln!(file, "      <tr><td>Max</td><td>{:?}</td></tr>", result.stats.max)?;
        writeln!(file, "      <tr><td>CV</td><td>{:.2}%</td></tr>", result.stats.coefficient_of_variation())?;
        writeln!(file, "      <tr><td>Outliers</td><td>{} ({:.1}%)</td></tr>", 
            result.stats.outliers.len(),
            result.stats.outlier_percentage()
        )?;
        
        if let Some(comp) = &result.comparison {
            writeln!(file, "      <tr class=\"comparison\"><td colspan=\"2\"><strong>Baseline Comparison</strong></td></tr>")?;
            writeln!(file, "      <tr><td>Baseline</td><td>{:?}</td></tr>", comp.baseline_mean)?;
            writeln!(file, "      <tr><td>Current</td><td>{:?}</td></tr>", comp.current_mean)?;
            writeln!(file, "      <tr><td>Speedup</td><td>{:.2}x</td></tr>", comp.speedup)?;
            writeln!(file, "      <tr><td>Change</td><td>{:+.2}%</td></tr>", comp.percentage_change())?;
            
            let status_class = if comp.is_regression { "regression" } else { "ok" };
            let status_text = if comp.is_regression { "REGRESSION" } else { "OK" };
            writeln!(file, "      <tr class=\"{}\"><td>Status</td><td>{}</td></tr>", status_class, status_text)?;
        }
        
        writeln!(file, "    </table>")?;
        writeln!(file, "  </div>")?;
        
        Ok(())
    }
    
    fn html_css() -> &'static str {
        r#"
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    background: #f5f5f5;
}
h1 {
    color: #333;
    border-bottom: 3px solid #007bff;
    padding-bottom: 10px;
}
.benchmark {
    background: white;
    border-radius: 8px;
    padding: 20px;
    margin: 20px 0;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}
h2 {
    color: #007bff;
    margin-top: 0;
}
table {
    width: 100%;
    border-collapse: collapse;
}
th, td {
    padding: 10px;
    text-align: left;
    border-bottom: 1px solid #eee;
}
th {
    background: #f8f9fa;
    font-weight: 600;
}
.comparison {
    background: #e7f3ff;
}
.regression {
    background: #ffe7e7;
    color: #d32f2f;
    font-weight: bold;
}
.ok {
    background: #e7ffe7;
    color: #2e7d32;
}
        "#
    }
    
    fn generate_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        writeln!(file, "{{")?;
        writeln!(file, "  \"benchmarks\": [")?;
        
        for (i, result) in self.results.iter().enumerate() {
            writeln!(file, "    {{")?;
            writeln!(file, "      \"name\": \"{}\",", result.name)?;
            writeln!(file, "      \"mean_ns\": {},", result.stats.mean.as_nanos())?;
            writeln!(file, "      \"median_ns\": {},", result.stats.median.as_nanos())?;
            writeln!(file, "      \"std_dev_ns\": {},", result.stats.std_dev.as_nanos())?;
            writeln!(file, "      \"p95_ns\": {},", result.stats.p95.as_nanos())?;
            writeln!(file, "      \"p99_ns\": {},", result.stats.p99.as_nanos())?;
            writeln!(file, "      \"min_ns\": {},", result.stats.min.as_nanos())?;
            writeln!(file, "      \"max_ns\": {},", result.stats.max.as_nanos())?;
            writeln!(file, "      \"cv_percent\": {},", result.stats.coefficient_of_variation())?;
            writeln!(file, "      \"outlier_count\": {},", result.stats.outliers.len())?;
            writeln!(file, "      \"outlier_percent\": {}", result.stats.outlier_percentage())?;
            
            if let Some(comp) = &result.comparison {
                writeln!(file, "      ,\"comparison\": {{")?;
                writeln!(file, "        \"baseline_mean_ns\": {},", comp.baseline_mean.as_nanos())?;
                writeln!(file, "        \"current_mean_ns\": {},", comp.current_mean.as_nanos())?;
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
        writeln!(file, "}}")?;
        
        Ok(())
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
    }
}
