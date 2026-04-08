//! Report Generation Demo
//! 
//! This example demonstrates the enhanced report generation capabilities:
//! - HTML reports with rich formatting
//! - JSON export for CI/CD integration
//! - Historical tracking
//! - Charts and visualizations

use crate::ace::html::{build_document, bench::{BenchRunner, BenchConfig, ReportFormat}};
use crate::ace::html::bench::report::{ReportGenerator, HistoricalData};
use crate::ace::html::tests::browser_comparison::{
    BrowserComparisonRunner, generate_html_report, generate_json_report
};

pub fn main() {
    println!("🚀 ACE HTML Parser - Report Generation Demo\n");
    
    // Demo 1: Basic HTML and JSON reports
    demo_basic_reports();
    
    // Demo 2: HTML reports with charts
    demo_charts_report();
    
    // Demo 3: Historical tracking
    demo_historical_tracking();
    
    // Demo 4: Browser comparison reports
    demo_browser_comparison();
    
    println!("\n✅ All reports generated successfully!");
    println!("Check the target/ directory for generated reports.");
}

fn demo_basic_reports() {
    println!("📊 Demo 1: Basic HTML and JSON Reports");
    println!("=" .repeat(50));
    
    let html_samples = vec![
        ("<div>Simple</div>", "simple"),
        ("<div class='test' id='main'><p>Nested</p></div>", "nested"),
        ("<table><tr><td>Table</td></tr></table>", "table"),
    ];
    
    let mut generator = ReportGenerator::new()
        .with_title("ACE HTML Parser - Basic Benchmarks");
    
    for (html, name) in html_samples {
        let config = BenchConfig::new(name)
            .with_warmup(3)
            .with_measurements(10);
        
        let runner = BenchRunner::new(config);
        let result = runner.run(|| {
            let _doc = build_document(html);
        });
        
        generator.add_result(result);
        println!("  ✓ Benchmarked: {}", name);
    }
    
    // Generate HTML report
    generator.generate("target/demo_basic_report.html", ReportFormat::Html)
        .expect("Failed to generate HTML report");
    println!("  📄 HTML report: target/demo_basic_report.html");
    
    // Generate JSON report
    generator.generate("target/demo_basic_report.json", ReportFormat::Json)
        .expect("Failed to generate JSON report");
    println!("  📄 JSON report: target/demo_basic_report.json");
    
    println!();
}

fn demo_charts_report() {
    println!("📈 Demo 2: HTML Reports with Charts");
    println!("=".repeat(50));
    
    let html_samples = vec![
        ("<div>Small</div>", "small_doc"),
        ("<div>".to_string() + &"<p>Medium</p>".repeat(10) + "</div>", "medium_doc"),
        ("<div>".to_string() + &"<p>Large</p>".repeat(100) + "</div>", "large_doc"),
    ];
    
    let mut generator = ReportGenerator::new()
        .with_title("ACE HTML Parser - Performance by Document Size");
    
    for (html, name) in html_samples {
        let config = BenchConfig::new(name)
            .with_warmup(3)
            .with_measurements(10);
        
        let runner = BenchRunner::new(config);
        let result = runner.run(|| {
            let _doc = build_document(&html);
        });
        
        generator.add_result(result);
        println!("  ✓ Benchmarked: {}", name);
    }
    
    // Generate HTML report with charts
    generator.generate("target/demo_charts_report.html", ReportFormat::HtmlWithCharts)
        .expect("Failed to generate HTML report with charts");
    println!("  📊 HTML report with charts: target/demo_charts_report.html");
    
    println!();
}

fn demo_historical_tracking() {
    println!("📅 Demo 3: Historical Tracking");
    println!("=".repeat(50));
    
    let html = "<div><p>Test document</p></div>";
    
    let mut generator = ReportGenerator::new()
        .with_title("ACE HTML Parser - Historical Tracking");
    
    // Simulate multiple benchmark runs over time
    for run in 1..=3 {
        let config = BenchConfig::new(&format!("run_{}", run))
            .with_warmup(2)
            .with_measurements(5);
        
        let runner = BenchRunner::new(config);
        let result = runner.run(|| {
            let _doc = build_document(html);
        });
        
        // Add to historical data
        let historical = HistoricalData::from_timestamp(
            1700000000 + (run * 86400), // Simulate daily runs
            vec![result.clone()]
        );
        generator.add_historical(historical);
        generator.add_result(result);
        
        println!("  ✓ Run {}: recorded", run);
    }
    
    // Save historical data
    generator.save_historical("target/demo_historical.json")
        .expect("Failed to save historical data");
    println!("  💾 Historical data: target/demo_historical.json");
    
    // Generate report with historical trends
    generator.generate("target/demo_historical_report.html", ReportFormat::Html)
        .expect("Failed to generate historical report");
    println!("  📄 Historical report: target/demo_historical_report.html");
    
    println!();
}

fn demo_browser_comparison() {
    println!("🌐 Demo 4: Browser Comparison Reports");
    println!("=".repeat(50));
    
    let runner = BrowserComparisonRunner::new();
    let availability = runner.check_availability();
    
    println!("  Browser availability:");
    println!("    Chrome:  {}", if availability.chrome { "✓" } else { "✗" });
    println!("    Firefox: {}", if availability.firefox { "✓" } else { "✗" });
    
    if !availability.any_available() {
        println!("  ⚠️  No browsers available for comparison");
        println!("  Install Node.js and run setup scripts in benchmarks/");
        println!();
        return;
    }
    
    let html = "<div><p>Browser comparison test</p></div>";
    let comparison = runner.compare_all_parsers(html, "browser_test");
    
    // Generate markdown report
    let markdown = crate::ace::html::tests::browser_comparison::generate_comprehensive_report(&[comparison.clone()]);
    std::fs::write("target/demo_browser_comparison.md", markdown)
        .expect("Failed to write markdown report");
    println!("  📄 Markdown report: target/demo_browser_comparison.md");
    
    // Generate HTML report
    generate_html_report(&[comparison.clone()], "target/demo_browser_comparison.html")
        .expect("Failed to generate HTML report");
    println!("  📄 HTML report: target/demo_browser_comparison.html");
    
    // Generate JSON report
    generate_json_report(&[comparison], "target/demo_browser_comparison.json")
        .expect("Failed to generate JSON report");
    println!("  📄 JSON report: target/demo_browser_comparison.json");
    
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_report_generation_demo() {
        // Just verify the demo runs without panicking
        demo_basic_reports();
        demo_charts_report();
        demo_historical_tracking();
        // Skip browser comparison in tests as it requires external dependencies
    }
}
