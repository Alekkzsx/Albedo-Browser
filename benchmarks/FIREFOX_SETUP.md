# Firefox Benchmark Setup Guide

This document describes the Firefox benchmark infrastructure for comparing ACE HTML parser performance against Firefox's HTML parser.

## Overview

The Firefox benchmark setup mirrors the Chrome benchmark infrastructure, using Playwright instead of Puppeteer to drive Firefox's real HTML parser.

## Files Created

### 1. JavaScript Benchmark Script
**File**: `benchmarks/firefox_parser_bench_playwright.js`

This Node.js script uses Playwright to:
- Launch headless Firefox
- Run HTML parsing benchmarks using Firefox's real parser
- Measure parsing performance with warmup and measurement phases
- Output JSON statistics (mean, median, p95, p99, throughput)

**Usage**:
```bash
node benchmarks/firefox_parser_bench_playwright.js <html_file> <benchmark_name>
```

### 2. Rust Benchmark Module
**File**: `src/ace/html/tests/firefox_bench.rs`

This Rust module provides:
- `FirefoxBenchRunner`: Orchestrates Firefox benchmarks via Node.js
- `FirefoxBenchResult`: Stores Firefox benchmark results
- `ParserComparison`: Compares ACE vs Firefox performance
- `generate_comparison_report()`: Generates markdown reports

**Key Features**:
- Automatic Node.js script invocation
- JSON parsing (zero dependencies)
- Statistical comparison
- Report generation

### 3. Rust Comparison Tests
**File**: `src/ace/html/tests/firefox_comparison_tests.rs`

Test suite that:
- Compares ACE vs Firefox on various document sizes
- Generates comparison reports
- Validates performance targets
- Provides examples for custom benchmarks

**Test Cases**:
- Small documents (10 KB)
- Medium documents (500 KB)
- Large documents (1.2 MB Wikipedia-like)
- Stress tests (5 MB)

### 4. Updated Configuration Files

**`benchmarks/package.json`**:
- Added Playwright as optional dependency
- Added Firefox benchmark scripts
- Updated description to include Firefox

**`src/ace/html/tests/mod.rs`**:
- Added `firefox_bench` module
- Added `firefox_comparison_tests` module

**`benchmarks/setup.sh` and `benchmarks/setup.ps1`**:
- Updated to support both Chrome and Firefox
- Interactive menu for choosing which browsers to install
- Automatic Playwright installation

**`benchmarks/README.md`**:
- Comprehensive documentation for both Chrome and Firefox benchmarks
- Setup instructions for Playwright
- Usage examples
- Troubleshooting guide

## Installation

### Quick Start

```bash
# Navigate to benchmarks directory
cd benchmarks

# Run setup script (interactive)
./setup.sh  # Linux/macOS
# or
./setup.ps1  # Windows

# Choose option 2 or 3 to install Playwright
```

### Manual Installation

```bash
cd benchmarks
npm install playwright
npx playwright install firefox
```

## Usage

### Running Firefox Benchmarks from Rust

```bash
# Run all Firefox comparison tests
cargo test --release firefox_comparison -- --nocapture

# Run specific test
cargo test --release test_firefox_comparison_wikipedia -- --nocapture

# Generate full comparison report
cargo test --release test_generate_full_comparison_report -- --nocapture
```

### Running Firefox Benchmark Script Directly

```bash
# Create a test HTML file
echo "<div>test</div>" > test.html

# Run benchmark
node benchmarks/firefox_parser_bench_playwright.js test.html "Test Benchmark"
```

### Example Output

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

## Architecture

### Workflow

1. **Rust Test** calls `FirefoxBenchRunner::compare_parsers()`
2. **ACE Benchmark** runs in Rust using `BenchRunner`
3. **Firefox Benchmark** runs via Node.js:
   - Writes HTML to temp file
   - Spawns `node firefox_parser_bench_playwright.js`
   - Playwright launches Firefox
   - Measures parsing time using Performance API
   - Returns JSON statistics
4. **Comparison** calculates speedup and generates report

### Data Flow

```
Rust Test
    ↓
FirefoxBenchRunner
    ↓
    ├─→ ACE Benchmark (Rust)
    │       ↓
    │   BenchResult
    │
    └─→ Node.js Script
            ↓
        Playwright
            ↓
        Firefox Parser
            ↓
        JSON Output
            ↓
    FirefoxBenchResult
            ↓
    ParserComparison
            ↓
    Report (Markdown)
```

## Comparison with Chrome Benchmark

| Aspect | Chrome | Firefox |
|--------|--------|---------|
| Tool | Puppeteer | Playwright |
| Script | `chrome_parser_bench_puppeteer.js` | `firefox_parser_bench_playwright.js` |
| Rust Module | `chrome_bench.rs` | `firefox_bench.rs` |
| Tests | `chrome_comparison_tests.rs` | `firefox_comparison_tests.rs` |
| Browser Size | ~170 MB | ~80 MB |
| API | Similar | Similar |

Both implementations follow the same pattern for consistency.

## Performance Targets

According to the spec requirements:

- **Minimum**: 300 MB/s throughput (baseline)
- **Target**: 500 MB/s throughput (browser-level)
- **Stretch**: 800 MB/s throughput (superior to browsers)

### Latency Targets

- **16KB chunk**: p50 < 0.5ms, p99 < 1ms
- **64KB chunk**: p50 < 2ms, p99 < 5ms

## Troubleshooting

### Playwright Not Found

```bash
cd benchmarks
npm install playwright
npx playwright install firefox
```

### Firefox Download Fails

```bash
# Manually install Firefox browser
npx playwright install firefox --force
```

### Node.js Not Found

Install Node.js from https://nodejs.org/ or via package manager:
- Ubuntu/Debian: `sudo apt install nodejs npm`
- macOS: `brew install node`
- Windows: Download installer

### Permission Issues (Linux/macOS)

```bash
chmod +x benchmarks/setup.sh
./benchmarks/setup.sh
```

## CI Integration

Example GitHub Actions workflow:

```yaml
- name: Setup Node.js
  uses: actions/setup-node@v3
  with:
    node-version: '18'

- name: Install Playwright
  run: |
    cd benchmarks
    npm install playwright
    npx playwright install firefox

- name: Run Firefox Benchmarks
  run: cargo test --release firefox_comparison -- --nocapture
```

## Future Enhancements

Potential improvements:
- Add Safari benchmark using Playwright WebKit
- Add Edge benchmark using Playwright Chromium
- Parallel benchmark execution
- Historical performance tracking
- Automated regression detection
- Performance visualization (charts/graphs)

## References

- [Playwright Documentation](https://playwright.dev/)
- [Firefox HTML Parser (Gecko)](https://searchfox.org/mozilla-central/source/parser/html)
- [ACE HTML Parser Design](../.kiro/specs/ace-html-chrome-level/design.md)
- [ACE HTML Parser Requirements](../.kiro/specs/ace-html-chrome-level/requirements.md)
- [WHATWG HTML Parsing Spec](https://html.spec.whatwg.org/multipage/parsing.html)

## License

MIT License - Same as the Albedo Browser project

---

**Created**: 2024
**Version**: 1.0
**Status**: Ready for use
