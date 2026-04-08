# Enhanced Report Generation

This document describes the enhanced report generation capabilities for ACE HTML Parser benchmarks.

## Features

### 1. Multiple Report Formats

#### HTML Reports
- **Basic HTML**: Clean, professional reports with modern styling
- **HTML with Charts**: Enhanced reports with visual bar charts for performance comparison
- Features:
  - Responsive design
  - Summary cards with key metrics
  - Detailed statistics tables
  - Gradient backgrounds and modern UI
  - Mobile-friendly layout

#### JSON Reports
- Machine-readable format for CI/CD integration
- Includes:
  - Timestamp and metadata
  - Complete statistics (mean, median, p95, p99, etc.)
  - Baseline comparisons
  - Historical data (if available)
- Perfect for:
  - Automated regression detection
  - Performance tracking systems
  - Data analysis pipelines

### 2. Historical Tracking

Track benchmark performance over time:
- Save benchmark results with timestamps
- Load historical data from previous runs
- Compare current performance against historical trends
- Visualize performance changes over time

```rust
use crate::ace::html::bench::report::{ReportGenerator, HistoricalData};

let mut generator = ReportGenerator::new();

// Add current results
generator.add_result(current_result);

// Load historical data
generator.load_historical("benchmarks/history.json")?;

// Save updated history
generator.save_historical("benchmarks/history.json")?;
```

### 3. Browser Comparison Reports

Compare ACE HTML Parser against Chrome and Firefox:
- Side-by-side performance comparison
- Speedup calculations
- Multiple output formats (Markdown, HTML, JSON)
- Visual indicators for faster/slower performance

```rust
use crate::ace::html::tests::browser_comparison::{
    BrowserComparisonRunner,
    generate_html_report,
    generate_json_report
};

let runner = BrowserComparisonRunner::new();
let comparison = runner.compare_all_parsers(html, "test_name");

// Generate reports
generate_html_report(&[comparison.clone()], "report.html")?;
generate_json_report(&[comparison], "report.json")?;
```

## Usage Examples

### Basic Report Generation

```rust
use crate::ace::html::bench::{BenchRunner, BenchConfig, ReportFormat};
use crate::ace::html::bench::report::ReportGenerator;

// Run benchmarks
let config = BenchConfig::new("my_benchmark")
    .with_warmup(5)
    .with_measurements(20);

let runner = BenchRunner::new(config);
let result = runner.run(|| {
    // Your code to benchmark
});

// Generate reports
let mut generator = ReportGenerator::new()
    .with_title("My Custom Benchmark Report");

generator.add_result(result);

// HTML report
generator.generate("report.html", ReportFormat::Html)?;

// JSON report
generator.generate("report.json", ReportFormat::Json)?;

// HTML with charts
generator.generate("report_charts.html", ReportFormat::HtmlWithCharts)?;
```

### Historical Tracking

```rust
use crate::ace::html::bench::report::{ReportGenerator, HistoricalData};

let mut generator = ReportGenerator::new();

// Load previous results
if let Ok(_) = generator.load_historical("history.json") {
    println!("Loaded historical data");
}

// Add new results
generator.add_result(new_result);

// Save updated history
generator.save_historical("history.json")?;

// Generate report with trends
generator.generate("report.html", ReportFormat::Html)?;
```

### Browser Comparison

```rust
use crate::ace::html::tests::browser_comparison::{
    BrowserComparisonRunner,
    generate_comprehensive_report,
    generate_html_report,
    generate_json_report
};

let runner = BrowserComparisonRunner::new();

// Check browser availability
let availability = runner.check_availability();
if !availability.any_available() {
    println!("No browsers available for comparison");
    return;
}

// Run comparison
let html = "<div>Test document</div>";
let comparison = runner.compare_all_parsers(html, "test");

// Generate markdown report
let markdown = generate_comprehensive_report(&[comparison.clone()]);
std::fs::write("comparison.md", markdown)?;

// Generate HTML report
generate_html_report(&[comparison.clone()], "comparison.html")?;

// Generate JSON report
generate_json_report(&[comparison], "comparison.json")?;
```

## Report Formats

### HTML Report Structure

```
┌─────────────────────────────────────┐
│  Title                              │
│  Generated: timestamp               │
├─────────────────────────────────────┤
│  Summary Cards                      │
│  ┌──────┐ ┌──────┐ ┌──────┐       │
│  │Total │ │ Avg  │ │Regr. │       │
│  └──────┘ └──────┘ └──────┘       │
├─────────────────────────────────────┤
│  Benchmark Results                  │
│  ┌─────────────────────────────┐   │
│  │ Benchmark Name              │   │
│  │ ┌────┐ ┌────┐ ┌────┐ ┌────┐│   │
│  │ │Mean│ │P95 │ │P99 │ │ CV ││   │
│  │ └────┘ └────┘ └────┘ └────┘│   │
│  │ Detailed Statistics Table   │   │
│  └─────────────────────────────┘   │
├─────────────────────────────────────┤
│  Historical Trends (if available)   │
│  Table with timestamp, metrics,     │
│  and trend indicators               │
└─────────────────────────────────────┘
```

### JSON Report Structure

```json
{
  "title": "Report Title",
  "timestamp": 1234567890,
  "benchmarks": [
    {
      "name": "benchmark_name",
      "statistics": {
        "mean_ns": 1000000,
        "mean_ms": 1.0,
        "median_ns": 950000,
        "median_ms": 0.95,
        "p95_ns": 1200000,
        "p95_ms": 1.2,
        "p99_ns": 1500000,
        "p99_ms": 1.5,
        "cv_percent": 5.2,
        "outlier_count": 2,
        "outlier_percent": 2.0
      },
      "comparison": {
        "baseline_mean_ns": 1200000,
        "baseline_mean_ms": 1.2,
        "speedup": 1.2,
        "change_percent": 20.0,
        "is_regression": false
      }
    }
  ],
  "historical": [
    {
      "timestamp": 1234567890,
      "results": [
        {
          "name": "benchmark_name",
          "mean_ms": 1.0
        }
      ]
    }
  ]
}
```

## CI/CD Integration

### Example GitHub Actions Workflow

```yaml
name: Performance Benchmarks

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run benchmarks
        run: cargo run --example report_generation_demo
      
      - name: Check for regressions
        run: |
          # Parse JSON report and check for regressions
          python scripts/check_regressions.py target/demo_basic_report.json
      
      - name: Upload reports
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-reports
          path: |
            target/*.html
            target/*.json
            target/*.md
```

### Example Regression Detection Script

```python
import json
import sys

def check_regressions(report_path):
    with open(report_path) as f:
        data = json.load(f)
    
    regressions = []
    for benchmark in data['benchmarks']:
        if 'comparison' in benchmark:
            comp = benchmark['comparison']
            if comp['is_regression']:
                regressions.append({
                    'name': benchmark['name'],
                    'change': comp['change_percent']
                })
    
    if regressions:
        print("⚠️  Performance regressions detected:")
        for reg in regressions:
            print(f"  - {reg['name']}: {reg['change']:.1f}% slower")
        sys.exit(1)
    else:
        print("✅ No performance regressions detected")

if __name__ == '__main__':
    check_regressions(sys.argv[1])
```

## Best Practices

1. **Regular Benchmarking**: Run benchmarks regularly to track performance trends
2. **Historical Data**: Keep historical data to detect gradual performance degradation
3. **Multiple Formats**: Generate both HTML (for humans) and JSON (for automation)
4. **Browser Comparison**: Periodically compare against Chrome/Firefox to ensure competitiveness
5. **CI Integration**: Automate benchmark runs and regression detection in CI/CD pipelines

## Troubleshooting

### Browser Comparison Not Working

If browser comparison fails:
1. Ensure Node.js is installed: `node --version`
2. Run setup scripts in `benchmarks/` directory
3. Check that benchmark scripts exist:
   - `benchmarks/chrome_parser_bench.js`
   - `benchmarks/firefox_parser_bench_playwright.js`

### Historical Data Not Loading

If historical data fails to load:
1. Check file exists and is readable
2. Verify JSON format is valid
3. Ensure file permissions are correct

### Reports Not Generating

If report generation fails:
1. Check target directory exists and is writable
2. Verify sufficient disk space
3. Check file paths are correct

## Future Enhancements

Potential future improvements:
- Interactive JavaScript charts (using Chart.js or similar)
- PDF report generation
- Email notifications for regressions
- Slack/Discord integration
- Automated performance analysis and recommendations
- Comparison against multiple baseline versions
- Performance budgets and alerts
