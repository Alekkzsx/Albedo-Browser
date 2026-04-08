# Browser Comparison - Side-by-Side Benchmarking

This module provides infrastructure for comprehensive side-by-side comparison of the ACE HTML parser against both Chrome and Firefox parsers.

## Overview

The browser comparison module builds on top of the existing Chrome and Firefox benchmark modules to provide:

- **Unified Comparison Interface**: Run ACE, Chrome, and Firefox benchmarks simultaneously
- **Comprehensive Reports**: Generate detailed comparison reports with statistics
- **Performance Insights**: Identify performance gaps and strengths across all three parsers
- **Flexible Testing**: Support for various document types and sizes

## Architecture

```
BrowserComparisonRunner
    ├── ChromeBenchRunner (from chrome_bench.rs)
    ├── FirefoxBenchRunner (from firefox_bench.rs)
    └── ACE Benchmark (using bench::BenchRunner)
```

## Prerequisites

To use the browser comparison functionality, you need:

1. **Node.js** installed on your system
2. **Chrome benchmark setup** (see `benchmarks/README.md`)
   - Puppeteer installed: `npm install puppeteer`
3. **Firefox benchmark setup** (see `benchmarks/FIREFOX_SETUP.md`)
   - Playwright installed: `npm install playwright`

The comparison will work with any available browsers - if only Chrome or Firefox is available, it will compare against that browser only.

## Usage

### Basic Usage

```rust
use albedo::ace::html::tests::browser_comparison::{
    BrowserComparisonRunner,
    generate_comprehensive_report,
};

// Create the comparison runner
let runner = BrowserComparisonRunner::new();

// Check which browsers are available
let availability = runner.check_availability();
println!("Chrome available: {}", availability.chrome);
println!("Firefox available: {}", availability.firefox);

// Run a comparison
let html = "<html><body><div>Hello World</div></body></html>";
let comparison = runner.compare_all_parsers(html, "Simple Document");

// Print results
comparison.print_comparison();
```

### Running Multiple Benchmarks

```rust
let runner = BrowserComparisonRunner::new();
let mut comparisons = Vec::new();

// Benchmark 1: Simple document
let simple = "<div>test</div>";
comparisons.push(runner.compare_all_parsers(simple, "Simple"));

// Benchmark 2: Complex document
let complex = r#"
    <html>
        <body>
            <div class="container">
                <h1>Title</h1>
                <p>Content</p>
            </div>
        </body>
    </html>
"#;
comparisons.push(runner.compare_all_parsers(complex, "Complex"));

// Generate comprehensive report
let report = generate_comprehensive_report(&comparisons);
std::fs::write("comparison_report.md", report).unwrap();
```

### Running Tests

```bash
# Run all browser comparison tests
cargo test --lib browser_comparison

# Run specific test
cargo test --lib test_simple_document_comparison

# Run with output
cargo test --lib browser_comparison -- --nocapture
```

### Running the Demo

```bash
# Run the browser comparison demo
cargo run --example browser_comparison_demo
```

## Output Format

### Console Output

The comparison prints detailed statistics for each parser:

```
================================================================================
Benchmark: Simple Document
================================================================================

📊 ACE HTML Parser:
  Mean:       125.3µs
  Median:     123.1µs
  P95:        145.2µs
  P99:        156.8µs
  CV:         8.5%

🌐 Chrome HTML Parser:
  Mean:       98.2µs
  Median:     96.5µs
  P95:        112.3µs
  P99:        125.6µs
  Throughput: 450.2 MB/s

🦊 Firefox HTML Parser:
  Mean:       105.6µs
  Median:     103.2µs
  P95:        120.1µs
  P99:        132.4µs
  Throughput: 420.8 MB/s

📈 Comparison Summary:
  ACE vs Chrome:   ⚠ 0.78x slower (27.6% behind)
  ACE vs Firefox:  ⚠ 0.84x slower (18.7% behind)
================================================================================
```

### Markdown Report

The `generate_comprehensive_report()` function creates a detailed markdown report:

```markdown
# ACE HTML Parser - Comprehensive Browser Comparison

## Performance Comparison Summary

| Benchmark | ACE Mean | Chrome Mean | Firefox Mean | vs Chrome | vs Firefox |
|-----------|----------|-------------|--------------|-----------|------------|
| Simple    | 125.3µs  | 98.2µs      | 105.6µs      | ⚠ 0.78x   | ⚠ 0.84x    |
| Complex   | 456.7µs  | 389.2µs     | 412.3µs      | ⚠ 0.85x   | ⚠ 0.90x    |

## Detailed Statistics

### Simple
**ACE HTML Parser:**
- Mean: 125.3µs
- Median: 123.1µs
- P95: 145.2µs
- P99: 156.8µs
- CV: 8.5%

**Chrome HTML Parser:**
- Mean: 98.2µs
- Median: 96.5µs
- P95: 112.3µs
- P99: 125.6µs
- Throughput: 450.2 MB/s

**ACE vs Chrome:** 0.78x (ACE is 27.6% slower)
**ACE vs Firefox:** 0.84x (ACE is 18.7% slower)

## Performance Insights
- **Average ACE vs Chrome:** 0.82x
- **Average ACE vs Firefox:** 0.87x
```

## API Reference

### `BrowserComparisonRunner`

Main struct for running browser comparisons.

**Methods:**
- `new()` - Create a new comparison runner
- `check_availability()` - Check which browsers are available
- `compare_all_parsers(html: &str, name: &str)` - Run comparison across all available parsers

### `BrowserComparison`

Result of a single comparison benchmark.

**Fields:**
- `name: String` - Benchmark name
- `ace_result: BenchResult` - ACE parser results
- `chrome_result: Option<ChromeBenchResult>` - Chrome parser results (if available)
- `firefox_result: Option<FirefoxBenchResult>` - Firefox parser results (if available)
- `ace_vs_chrome_speedup: Option<f64>` - Speedup factor vs Chrome
- `ace_vs_firefox_speedup: Option<f64>` - Speedup factor vs Firefox

**Methods:**
- `print_comparison()` - Print formatted comparison to console

### `generate_comprehensive_report(comparisons: &[BrowserComparison]) -> String`

Generate a comprehensive markdown report from multiple comparisons.

## Benchmark Types

The comparison supports various benchmark types:

1. **Micro Benchmarks**: Small, focused HTML snippets
   - Simple tags: `<div>test</div>`
   - Attributes: `<div class="x" id="y">test</div>`
   - Entities: `&nbsp; &lt; &gt;`

2. **Small Documents**: Realistic small pages (< 10KB)
   - Basic structure with header, main, footer
   - Navigation menus
   - Simple forms

3. **Medium Documents**: Typical web pages (10-100KB)
   - Multiple sections
   - Tables
   - Lists

4. **Large Documents**: Complex pages (> 100KB)
   - Many elements (100+ divs)
   - Deep nesting
   - Attribute-heavy

## Performance Metrics

The comparison tracks:

- **Mean**: Average parsing time
- **Median**: Middle value (50th percentile)
- **P95**: 95th percentile (worst 5% excluded)
- **P99**: 99th percentile (worst 1% excluded)
- **CV**: Coefficient of variation (consistency measure)
- **Throughput**: MB/s (for browser parsers)
- **Speedup**: Relative performance (ACE vs browser)

## Interpreting Results

### Speedup Factor

- `> 1.0`: ACE is faster than the browser
- `= 1.0`: ACE matches the browser
- `< 1.0`: ACE is slower than the browser

### Status Indicators

- `✓`: ACE is faster (good)
- `⚠`: ACE is slower (needs optimization)

### Typical Results

Based on current implementation:

- **Simple documents**: ACE is typically 0.7-0.9x browser speed
- **Complex documents**: ACE is typically 0.8-0.95x browser speed
- **Large documents**: ACE may match or exceed browser speed with speculative parsing

## Troubleshooting

### "No browsers available"

**Problem**: Neither Chrome nor Firefox benchmarks are available.

**Solution**:
1. Install Node.js
2. Run setup scripts in `benchmarks/` directory
3. Verify with `node --version`

### "Chrome benchmark failed"

**Problem**: Chrome benchmark script fails to run.

**Solution**:
1. Check if Puppeteer is installed: `npm list puppeteer`
2. Reinstall if needed: `npm install puppeteer`
3. Check script exists: `benchmarks/chrome_parser_bench.js`

### "Firefox benchmark failed"

**Problem**: Firefox benchmark script fails to run.

**Solution**:
1. Check if Playwright is installed: `npm list playwright`
2. Reinstall if needed: `npm install playwright`
3. Check script exists: `benchmarks/firefox_parser_bench_playwright.js`

## Future Enhancements

Planned improvements:

1. **Safari Support**: Add WebKit/Safari comparison
2. **Memory Comparison**: Compare memory usage across parsers
3. **Streaming Comparison**: Compare incremental parsing performance
4. **Visual Reports**: Generate HTML reports with charts
5. **CI Integration**: Automated regression detection

## Related Modules

- `chrome_bench.rs` - Chrome-specific benchmarking
- `firefox_bench.rs` - Firefox-specific benchmarking
- `bench/mod.rs` - Core benchmarking framework
- `macro_bench.rs` - ACE macro benchmarks
- `micro_bench.rs` - ACE micro benchmarks

## Contributing

When adding new comparison features:

1. Maintain compatibility with existing Chrome/Firefox modules
2. Add tests in `browser_comparison_tests.rs`
3. Update this documentation
4. Ensure graceful degradation when browsers are unavailable

## License

Part of the Albedo Browser project. See LICENSE file for details.
