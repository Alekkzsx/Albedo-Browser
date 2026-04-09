# ACE HTML Parser - Performance Guide

This guide focuses on practical performance work for ACE-html: measuring, profiling, and tuning.

## Goals and Targets

- Throughput target: >= 500 MB/s for large HTML workloads.
- Streaming target: p50 < 0.5 ms and p99 < 1 ms for 16 KB chunks.
- Memory target: <= 50% of Chrome for comparable documents.

## 1) Optimization Techniques

### SIMD paths

ACE-html uses runtime SIMD detection in `simd.rs`.

- AVX-512: fastest path for bulk scans.
- AVX2: main accelerated fallback.
- SSE2/scalar: compatibility fallback.

Where to focus:

- `normalize_whitespace_simd`
- `fast_entity_lookup`
- tag boundary scans in lexer/tokenizer hot paths

### Zero-copy and memory locality

- Use `NodeArena` to avoid per-node heap allocation.
- Use `StringInterner` to deduplicate tag and attribute strings.
- Keep hot structs compact and contiguous where possible.

### Branch behavior

- Keep hot-path conditions simple and predictable.
- Avoid rare-case branching inside tight loops; move slow paths to helper functions.

### Parallel parsing features

- `speculative.rs` for tokenizer/tree-builder overlap.
- `preload_scanner.rs` for parallel resource discovery.
- Bounded channels for backpressure and stable memory behavior.

## 2) Profiling Guide

### Build profile

Use release mode for any meaningful metric:

```bash
cargo build --release
```

### Baseline benchmark runs

Run ACE micro/macro benches:

```bash
cargo test --release ace::html::tests::micro_bench -- --nocapture
cargo test --release ace::html::tests::macro_bench -- --nocapture
```

Generate benchmark reports:

```bash
cargo run --release --example report_generation_demo
```

### Browser comparison runs

Requires Node.js scripts under `benchmarks/`:

```bash
cargo test --release ace::html::tests::browser_comparison_tests -- --nocapture
```

### CPU profiler workflow (Linux)

```bash
cargo build --release
perf record --call-graph=dwarf ./target/release/<your_binary>
perf report
```

### CPU profiler workflow (Windows)

Recommended tools:

- Windows Performance Recorder / Analyzer (WPR/WPA)
- Visual Studio Profiler
- `cargo flamegraph` on supported setups

### What to look for

- High self-time in lexer/tokenizer loops.
- Excess string allocations indicating poor interner hit rate.
- Channel contention in speculative mode for medium-size documents.

## 3) Tuning Parameters

### Document-size thresholds

- Small docs: prefer single-thread parser path.
- Large docs: enable speculative tokenizer path.

Tune thresholds in speculative entry points if workload differs.

### Arena sizing

Defaults are conservative for broad workloads. Tune chunk size when:

- many small docs: smaller chunks reduce transient footprint
- very large docs: bigger chunks reduce chunk churn

### Channel capacity

Speculative parsing uses bounded channels. Increase carefully only if:

- tokenizer is consistently blocked on full channel
- memory headroom is available

### Preload scanner chunking

Parallel preload scanning chunk size affects throughput and overhead.

- smaller chunk: better balancing, more scheduler overhead
- bigger chunk: lower overhead, weaker balancing

### Benchmark configuration knobs

Use `BenchConfig` for:

- warmup iterations
- measured iterations
- regression thresholds

## 4) Regression Workflow

1. Run baseline benchmark suite.
2. Change one optimization at a time.
3. Re-run same suite with same environment.
4. Generate HTML/JSON report and compare.
5. If regression appears, inspect p95/p99 and not only mean.

## 5) Practical Checklist

- Build in release mode.
- Confirm SIMD level detected at runtime.
- Check interner hit rate and arena stats.
- Compare p95/p99 in addition to average.
- Validate on representative real HTML, not only synthetic data.

## Related Docs

- `ARCHITECTURE.md`
- `DATAFLOW.md`
- `USAGE_GUIDE.md`
- `bench/REPORT_GENERATION.md`
