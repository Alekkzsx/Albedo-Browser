# ACE HTML Parser - Contributing Guide

Thank you for contributing to ACE-html.

This guide defines local setup, coding conventions, testing expectations, and PR flow for `src/ace/html`.

## 1) Setup Instructions

### Requirements

- Rust stable (`rustup` + `cargo`)
- Git
- Node.js (optional, needed for browser comparison scripts)
- `patch` binary (required by `rquickjs-sys` build script on Windows)

### Windows note for `patch`

Install:

```powershell
winget install --id GnuWin32.Patch -e --accept-source-agreements --accept-package-agreements
```

If current shell does not see `patch`, restart terminal or prepend this folder to `PATH` during commands:

`%LOCALAPPDATA%\Microsoft\WinGet\Packages\GnuWin32.Patch_Microsoft.Winget.Source_8wekyb3d8bbwe\bin`

### Build

```bash
cargo check
```

## 2) Code Style

- Follow existing module layout and naming patterns.
- Keep hot-path code explicit and allocation-aware.
- Prefer small focused functions for slow/error paths.
- Public APIs should include Rustdoc comments and examples when practical.
- Do not introduce third-party parser dependencies into ACE-html core.

### Performance-sensitive changes

When touching lexer/tokenizer/tree-builder hot paths:

- document why the change should be faster
- include before/after benchmark data when possible
- avoid hidden allocations in tight loops

## 3) Testing Guidelines

Run the minimum fast checks before a PR:

```bash
cargo check
cargo test --release ace::html::bench::tests
cargo test --release ace::html::tests::tokenizer_tests
```

Recommended broader validation:

```bash
cargo test --release ace::html::tests::micro_bench -- --nocapture
cargo test --release ace::html::tests::macro_bench -- --nocapture
cargo test --release ace::html::tests::browser_comparison_tests -- --nocapture
```

### Test expectations

- Add tests for bug fixes and edge cases.
- Keep tests deterministic (no flaky time-sensitive assertions).
- For benchmark-oriented tests, validate thresholds conservatively.

## 4) Pull Request Process

1. Create a focused branch (single feature/fix scope).
2. Keep commits small and descriptive.
3. Include summary of behavior and performance impact.
4. Link relevant task IDs from `.kiro/specs/ace-html-chrome-level/tasks.md`.
5. Include test evidence (commands + key outputs).
6. Request review after local checks pass.

### PR checklist

- [ ] Code compiles (`cargo check`)
- [ ] Relevant tests pass
- [ ] Docs updated for user-visible or API changes
- [ ] No unrelated file churn
- [ ] Performance impact described (if hot path changed)

## 5) Scope Boundaries

This guide is ACE-html specific. Changes outside `src/ace/html` may require additional reviewers and subsystem-specific validation.

## Related Docs

- `ARCHITECTURE.md`
- `DATAFLOW.md`
- `PERFORMANCE_GUIDE.md`
- `USAGE_GUIDE.md`
