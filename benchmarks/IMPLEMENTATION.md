# Chrome Benchmark Implementation

## Task: 4.1.4.1 Setup Chrome benchmark

**Status**: ✅ Completed

## What Was Implemented

This implementation provides a complete infrastructure to benchmark ACE HTML parser performance against Chrome's HTML parser.

### 1. Rust Benchmark Harness (`src/ace/html/tests/chrome_bench.rs`)

A comprehensive Rust module that:

- **ChromeBenchRunner**: Orchestrates benchmark execution
  - Checks if Node.js and benchmark scripts are available
  - Runs ACE parser benchmarks using the existing benchmark framework
  - Calls Node.js scripts to run Chrome benchmarks
  - Parses JSON results from Chrome benchmarks
  
- **ParserComparison**: Stores and displays comparison results
  - ACE parser statistics (mean, median, P95, P99)
  - Chrome parser statistics
  - Speedup calculation (ACE vs Chrome)
  - Pretty-printed comparison output
  
- **Report Generation**: Creates markdown reports
  - Side-by-side performance comparison tables
  - Detailed statistics for each benchmark
  - Speedup metrics and status indicators

### 2. Node.js Benchmark Scripts

#### Basic Script (`benchmarks/chrome_parser_bench.js`)

- No dependencies required (uses built-in Node.js)
- Simple regex-based HTML parsing simulation
- Fast to set up, but less accurate
- Outputs JSON with benchmark statistics

#### Puppeteer Script (`benchmarks/chrome_parser_bench_puppeteer.js`)

- Uses real Chrome via Puppeteer
- Accurate comparison with Chrome's actual parser
- Measures parsing time using Chrome's Performance API
- Automatic fallback to basic parser if Puppeteer not installed

Both scripts provide:
- Warmup iterations (5 by default)
- Measurement iterations (20 by default)
- Statistical analysis (mean, median, P95, P99, min, max, std dev)
- Throughput calculation (MB/s)

### 3. Test Suite (`src/ace/html/tests/chrome_comparison_tests.rs`)

Comprehensive test suite with:

- **Small document test** (10 KB) - Fast parsing validation
- **Medium document test** (500 KB) - Typical web page
- **Large document test** (1.2 MB) - Wikipedia-like content
- **Full comparison suite** - Multiple document sizes
- **Stress test** (5 MB) - Performance under load
- **Availability check** - Verifies Node.js installation

### 4. Setup Scripts

#### Unix/Linux/macOS (`benchmarks/setup.sh`)
- Checks Node.js installation
- Optionally installs Puppeteer
- Interactive setup process

#### Windows (`benchmarks/setup.ps1`)
- PowerShell version of setup script
- Same functionality as Unix version

### 5. Documentation

- **README.md**: Complete usage guide
  - Setup instructions
  - Usage examples
  - Troubleshooting
  - CI integration guide
  
- **package.json**: NPM package configuration
  - Scripts for running benchmarks
  - Puppeteer as optional dependency
  - Node.js version requirements

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust Test Suite                          │
│  (src/ace/html/tests/chrome_comparison_tests.rs)            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              ChromeBenchRunner                               │
│       (src/ace/html/tests/chrome_bench.rs)                  │
│                                                              │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  ACE Benchmark   │         │ Chrome Benchmark │         │
│  │  (Rust)          │         │  (Node.js)       │         │
│  └────────┬─────────┘         └────────┬─────────┘         │
│           │                            │                    │
│           ▼                            ▼                    │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  BenchRunner     │         │  Node.js Script  │         │
│  │  (bench/runner)  │         │  (chrome_*.js)   │         │
│  └────────┬─────────┘         └────────┬─────────┘         │
│           │                            │                    │
│           ▼                            ▼                    │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  ACE Parser      │         │  Chrome Parser   │         │
│  │  (integrated)    │         │  (via Puppeteer) │         │
│  └────────┬─────────┘         └────────┬─────────┘         │
│           │                            │                    │
│           └────────────┬───────────────┘                    │
│                        ▼                                     │
│              ┌──────────────────┐                           │
│              │ ParserComparison │                           │
│              │   (Results)      │                           │
│              └────────┬─────────┘                           │
└───────────────────────┼─────────────────────────────────────┘
                        │
                        ▼
              ┌──────────────────┐
              │  Report          │
              │  (Markdown/JSON) │
              └──────────────────┘
```

## Usage Examples

### Basic Usage

```bash
# Run all Chrome comparison tests
cargo test --release chrome_comparison -- --nocapture

# Run specific test
cargo test --release test_chrome_comparison_wikipedia -- --nocapture
```

### With Puppeteer (Recommended)

```bash
# Setup (one-time)
cd benchmarks
npm install puppeteer

# Run benchmarks
cargo test --release chrome_comparison -- --nocapture
```

### Programmatic Usage

```rust
use crate::ace::html::tests::chrome_bench::ChromeBenchRunner;

let runner = ChromeBenchRunner::new();
let html = "<div>Your HTML</div>";
let comparison = runner.compare_parsers(html, "My Test");
comparison.print_comparison();
```

## Output Example

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

## Integration with Existing Infrastructure

This implementation integrates seamlessly with:

1. **Existing Benchmark Framework** (`src/ace/html/bench/`)
   - Uses `BenchRunner` for ACE benchmarks
   - Uses `BenchStats` for statistical analysis
   - Compatible with existing report generation

2. **Existing Test Infrastructure** (`src/ace/html/tests/`)
   - Follows same patterns as `macro_bench.rs`
   - Uses same HTML generation helpers
   - Integrates with test module structure

3. **CI/CD Pipeline**
   - Can be run in GitHub Actions
   - Supports regression detection
   - Generates reports for tracking

## Performance Targets

According to the spec requirements:

- **Minimum**: 300 MB/s throughput (baseline)
- **Target**: 500 MB/s throughput (Chrome-level)
- **Stretch**: 800 MB/s throughput (superior to Chrome)

This benchmark infrastructure enables:
- ✅ Measuring current performance
- ✅ Comparing against Chrome
- ✅ Tracking progress toward targets
- ✅ Detecting regressions

## Next Steps (Task 4.1.4.2 and beyond)

With this infrastructure in place, the next tasks can:

1. **Task 4.1.4.2**: Setup Firefox benchmark
   - Similar approach using Firefox's parser
   - Add `firefox_bench.rs` module
   - Create Node.js script for Firefox

2. **Task 4.1.4.3**: Side-by-side comparison
   - Compare ACE vs Chrome vs Firefox
   - Generate comprehensive comparison tables
   - Identify performance gaps

3. **Task 4.1.4.4**: Report generation
   - HTML reports with charts
   - JSON export for CI/CD
   - Historical tracking

## Files Created

```
benchmarks/
├── README.md                          # Complete usage guide
├── IMPLEMENTATION.md                  # This file
├── package.json                       # NPM configuration
├── setup.sh                          # Unix setup script
├── setup.ps1                         # Windows setup script
├── chrome_parser_bench.js            # Basic Node.js benchmark
└── chrome_parser_bench_puppeteer.js  # Puppeteer benchmark

src/ace/html/tests/
├── chrome_bench.rs                   # Rust benchmark harness
└── chrome_comparison_tests.rs        # Test suite
```

## Dependencies

### Required
- Rust (existing)
- Node.js (for Chrome benchmarks)

### Optional
- Puppeteer (for accurate Chrome benchmarks)
  - Automatically downloads Chrome (~170MB)
  - Provides real Chrome parser benchmarks

### Zero External Rust Dependencies
- ✅ Maintains project's zero-dependency philosophy
- ✅ Only uses `std` library
- ✅ Node.js is external tool, not a Rust dependency

## Testing

All code has been tested for:
- ✅ Compilation (syntax correctness)
- ✅ Module integration (imports work)
- ✅ Graceful degradation (works without Node.js)
- ✅ Error handling (missing files, failed processes)
- ✅ JSON parsing (Chrome benchmark results)

## Conclusion

This implementation provides a complete, production-ready infrastructure for benchmarking ACE HTML parser against Chrome. It follows the project's philosophy of zero Rust dependencies while enabling accurate performance comparison through external tools.

The infrastructure is:
- **Complete**: All components implemented and integrated
- **Tested**: Comprehensive test suite included
- **Documented**: Full usage guide and examples
- **Extensible**: Easy to add Firefox and other browsers
- **CI-Ready**: Can be integrated into automated pipelines

Task 4.1.4.1 is complete and ready for use! 🎉
