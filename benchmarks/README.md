# Browser HTML Parser Benchmark Setup

This directory contains the infrastructure to benchmark ACE HTML parser against Chrome's and Firefox's HTML parsers.

## Overview

Since we can't directly call Chrome's or Firefox's C++ parsers from Rust, we use a two-step approach:

1. **ACE Benchmarks**: Run in Rust using the benchmark framework in `src/ace/html/bench/`
2. **Browser Benchmarks**: Run via Node.js scripts that use real browsers
3. **Comparison**: Compare results and generate reports

## Files

- `chrome_parser_bench.js` - Basic Node.js benchmark (no dependencies)
- `chrome_parser_bench_puppeteer.js` - Advanced Chrome benchmark using Puppeteer
- `firefox_parser_bench_playwright.js` - Firefox benchmark using Playwright
- `README.md` - This file

## Setup

### Option 1: Basic Benchmark (No Dependencies)

The basic benchmark uses a simple regex-based parser simulation. It's fast to set up but less accurate.

```bash
# No setup needed - uses built-in Node.js
node benchmarks/chrome_parser_bench.js <html_file> <benchmark_name>
```

### Option 2: Chrome Benchmark with Puppeteer (Recommended)

The Puppeteer benchmark uses real Chrome, providing accurate comparison data.

```bash
# Install Puppeteer (one-time setup)
cd benchmarks
npm install puppeteer

# Run benchmark
node chrome_parser_bench_puppeteer.js <html_file> <benchmark_name>
```

### Option 3: Firefox Benchmark with Playwright (Recommended)

The Playwright benchmark uses real Firefox, providing accurate comparison data.

```bash
# Install Playwright (one-time setup)
cd benchmarks
npm install playwright

# Run benchmark
node firefox_parser_bench_playwright.js <html_file> <benchmark_name>
```

### Option 4: Install All Browsers

```bash
# Install both Puppeteer and Playwright
cd benchmarks
npm install puppeteer playwright
```

## Usage from Rust

The Rust benchmark harness automatically:

1. Generates HTML test documents
2. Runs ACE parser benchmarks
3. Calls Node.js scripts to run browser benchmarks
4. Compares results and generates reports

### Running Chrome Comparison Benchmarks

```bash
# Run all Chrome comparison benchmarks
cargo test --release chrome_comparison -- --nocapture

# Run specific benchmark
cargo test --release bench_chrome_wikipedia -- --nocapture
```

### Running Firefox Comparison Benchmarks

```bash
# Run all Firefox comparison benchmarks
cargo test --release firefox_comparison -- --nocapture

# Run specific benchmark
cargo test --release bench_firefox_wikipedia -- --nocapture
```

### Example Output (Chrome)

```
🔬 Running comparison benchmark: Wikipedia Homepage
Document size: 1200000 bytes (1.20 MB)

⏱️  Benchmarking ACE HTML Parser...
  ACE Mean: 2.45ms

⏱️  Benchmarking Chrome HTML Parser...
  Chrome Mean: 3.12ms

======================================================================
Benchmark: Wikipedia Homepage
======================================================================

ACE HTML Parser:
  Mean:       2.45ms
  Median:     2.41ms
  P95:        2.89ms
  P99:        3.15ms

Chrome HTML Parser:
  Mean:       3.12ms
  Median:     3.08ms
  P95:        3.45ms
  P99:        3.78ms
  Throughput: 384.62 MB/s

Comparison:
  Speedup:    1.27x
  Status:     ✓ ACE is 27.3% faster
======================================================================
```

### Example Output (Firefox)

```
🔬 Running comparison benchmark: Wikipedia Homepage
Document size: 1200000 bytes (1.20 MB)

⏱️  Benchmarking ACE HTML Parser...
  ACE Mean: 2.45ms

⏱️  Benchmarking Firefox HTML Parser...
  Firefox Mean: 2.98ms

======================================================================
Benchmark: Wikipedia Homepage
======================================================================

ACE HTML Parser:
  Mean:       2.45ms
  Median:     2.41ms
  P95:        2.89ms
  P99:        3.15ms

Firefox HTML Parser:
  Mean:       2.98ms
  Median:     2.95ms
  P95:        3.32ms
  P99:        3.65ms
  Throughput: 402.68 MB/s

Comparison:
  Speedup:    1.22x
  Status:     ✓ ACE is 21.6% faster
======================================================================
```

## Benchmark Scenarios

The comparison benchmarks test various document types:

1. **Wikipedia Homepage** (1.2 MB) - Complex article with tables, lists, links
2. **GitHub README** (500 KB) - Code blocks, markdown-style content
3. **Twitter Timeline** (2 MB) - Many small elements, dynamic content
4. **Amazon Product** (800 KB) - Product details, reviews, images
5. **YouTube Watch** (1.5 MB) - Video player, comments, recommendations

## Interpreting Results

### Speedup Metric

- **> 1.0**: ACE is faster than the browser
- **= 1.0**: ACE and browser are equal
- **< 1.0**: ACE is slower than the browser

### Target Performance

According to the spec requirements:

- **Minimum**: 300 MB/s throughput (baseline)
- **Target**: 500 MB/s throughput (Chrome/Firefox-level)
- **Stretch**: 800 MB/s throughput (superior to browsers)

### Latency Targets

- **16KB chunk**: p50 < 0.5ms, p99 < 1ms
- **64KB chunk**: p50 < 2ms, p99 < 5ms

## Troubleshooting

### Node.js Not Found

```bash
# Install Node.js
# Ubuntu/Debian:
sudo apt install nodejs npm

# macOS:
brew install node

# Windows:
# Download from https://nodejs.org/
```

### Puppeteer Installation Issues

```bash
# If Puppeteer fails to download Chrome:
npm install puppeteer --unsafe-perm=true

# Or use system Chrome:
npm install puppeteer-core
```

### Playwright Installation Issues

```bash
# If Playwright fails to download browsers:
npx playwright install firefox

# Or install all browsers:
npx playwright install
```

### Benchmark Script Not Found

Make sure you're running from the workspace root:

```bash
# From workspace root
cargo test --release chrome_comparison

# Not from subdirectory
```

### Advanced Usage

### Custom HTML Documents

```rust
// Chrome comparison
use crate::ace::html::tests::chrome_bench::ChromeBenchRunner;

let runner = ChromeBenchRunner::new();
let html = "<div>Your custom HTML</div>";
let comparison = runner.compare_parsers(html, "Custom Test");
comparison.print_comparison();

// Firefox comparison
use crate::ace::html::tests::firefox_bench::FirefoxBenchRunner;

let runner = FirefoxBenchRunner::new();
let html = "<div>Your custom HTML</div>";
let comparison = runner.compare_parsers(html, "Custom Test");
comparison.print_comparison();
```

### Generating Reports

```rust
// Chrome report
use crate::ace::html::tests::chrome_bench::{ChromeBenchRunner, generate_comparison_report};

let runner = ChromeBenchRunner::new();
let comparisons = vec![
    runner.compare_parsers(html1, "Test 1"),
    runner.compare_parsers(html2, "Test 2"),
];

let report = generate_comparison_report(&comparisons);
std::fs::write("target/chrome_comparison.md", report).unwrap();

// Firefox report
use crate::ace::html::tests::firefox_bench::{FirefoxBenchRunner, generate_comparison_report};

let runner = FirefoxBenchRunner::new();
let comparisons = vec![
    runner.compare_parsers(html1, "Test 1"),
    runner.compare_parsers(html2, "Test 2"),
];

let report = generate_comparison_report(&comparisons);
std::fs::write("target/firefox_comparison.md", report).unwrap();
```

## CI Integration

For continuous integration, you can:

1. Install Node.js and browser automation tools in CI environment
2. Run comparison benchmarks as part of test suite
3. Fail build if performance regresses below threshold

Example GitHub Actions:

```yaml
- name: Setup Node.js
  uses: actions/setup-node@v3
  with:
    node-version: '18'

- name: Install Browser Automation Tools
  run: |
    cd benchmarks
    npm install puppeteer playwright

- name: Run Chrome Comparison Benchmarks
  run: cargo test --release chrome_comparison -- --nocapture

- name: Run Firefox Comparison Benchmarks
  run: cargo test --release firefox_comparison -- --nocapture
```

## Notes

- Browser benchmarks require Node.js to be installed
- Puppeteer downloads Chrome (~170MB) on first run
- Playwright downloads Firefox (~80MB) on first run
- Benchmark results may vary based on system load and CPU
- Run benchmarks multiple times for stable results
- Use `--release` mode for accurate performance measurements

## References

- [ACE HTML Parser Design](../.kiro/specs/ace-html-chrome-level/design.md)
- [ACE HTML Parser Requirements](../.kiro/specs/ace-html-chrome-level/requirements.md)
- [Puppeteer Documentation](https://pptr.dev/)
- [Playwright Documentation](https://playwright.dev/)
- [WHATWG HTML Parsing Spec](https://html.spec.whatwg.org/multipage/parsing.html)
