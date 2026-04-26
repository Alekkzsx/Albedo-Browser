# ACE-HTML Superiority Roadmap

## Objective
Turn ACE-HTML into a parser that is stronger in three dimensions at the same time:
- specification conformance,
- sustained performance stability,
- regression resistance in CI.

## Current Gates
- Required conformance (`tests/ace_html_conformance/required.json`) must stay at 100%.
- Hard-cases dataset (`tests/ace_html_hard_cases.json`) must stay green.
- Property-based parser equivalence (`tests/ace_html_proptests.rs`) must stay green.
- Quality thresholds (`tools/ace_html_quality_thresholds.json`) block regressions in:
  - required tree pass rate,
  - html5lib tree pass rate,
  - tokenizer pass rate,
  - throughput,
  - memory per case.

## KPIs
- Required tree pass rate: `>= 99.0%` (target 100%).
- html5lib tree full pass rate: `>= 95.0%` (raise gradually to 99%+).
- Tokenizer subset pass rate: `>= 95.0%` (raise gradually to 99%+).
- Throughput (ACE native benchmark): `>= 5 MB/s` minimum gate.
- Mean parse time (ACE): `<= 500 ms` for configured perf payload.
- Memory cost: `avgAllocatedBytesPerCase <= 2_500_000`.

## Verification Commands
- Full ACE benchmark:
```bash
cargo run --release --bin ace_html_super_benchmark > ace_html_super_benchmark_results.json
```

- Memory profile:
```bash
cargo run --release --bin ace_html_memory_profile > ace_html_memory_profile_results.json
```

- Browser comparison (Linux script):
```bash
cd .codex-bench
npm run benchmark
```

- Regression gate:
```bash
python3 tools/ace_html_regression_check.py
```

## Fuzzing and Differential Checks
- Fuzz target location: `fuzz/fuzz_targets/ace_html_diff.rs`
- Run locally:
```bash
cargo fuzz run ace_html_diff
```

## CI Integration
- Workflow: `.github/workflows/ace-html-superiority.yml`
- Pipeline runs:
  1. required conformance,
  2. hard-cases guard,
  3. proptests,
  4. super benchmark,
  5. memory profile,
  6. regression threshold gate.

## Next Technical Focus
1. Raise html5lib full pass-rate threshold in small increments.
2. Expand tokenizer corpus beyond subset and wire into gate.
3. Add browser-native fallback harness for Firefox on environments where system binary is incompatible with Playwright.
4. Keep publishing benchmark artifacts for trend analysis.
