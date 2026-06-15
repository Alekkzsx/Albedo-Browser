# Progress - Milestone 2 Refactoring

Last visited: 2026-06-15T18:17:15Z

## Checklist

- [x] Modify `albedo-jit/src/contracts.rs` (OnceLock parser, register, helper methods)
- [x] Modify `src/ace/json.rs` (re-export contracts::core::JsonValue, custom from/to_serde, use them)
- [x] Modify `src/main.rs` (register parser in `main()`)
- [x] Modify `albedo-jit/src/runtime/js_value.rs` (add `to_number` and `to_int32` to `JsValue`)
- [x] Modify `albedo-jit/src/runtime/builtins.rs` (clean up helper functions, update calls, implement `BuiltinId::from_u32`)
- [x] Modify `albedo-jit/src/runtime/runtime_helpers.rs` (clean up, update calls)
- [x] Modify `albedo-jit/src/runtime/fast_builtins.rs` (clean up, update calls)
- [x] Modify `albedo-jit/src/compiler/air_interpreter.rs` (clean up, update calls)
- [x] Build and test verification (`cargo check` passed cleanly; `cargo test` timed out on permission prompt)
