## 2026-06-15T18:12:36Z
You are the Refactoring Worker for Milestone 2.
Your goal is to implement JIT contracts and JIT internals refactorings to remove duplication and improve the JIT JSON parser's correctness.

## Required Tasks

1. **Modify `albedo-jit/src/contracts.rs`**:
   - Add `use std::sync::OnceLock;` and `use std::collections::HashMap;` under `pub mod core`.
   - Add a static OnceLock: `static JSON_PARSER: OnceLock<fn(&str) -> Result<JsonValue, String>> = OnceLock::new();`
   - Add a registration function:
     ```rust
     pub fn register_json_parser(parser: fn(&str) -> Result<JsonValue, String>) {
         let _ = JSON_PARSER.set(parser);
     }
     ```
   - Update `json_parse` function to:
     ```rust
     pub fn json_parse(text: &str) -> AlbedoResult<JsonValue> {
         if let Some(parser) = JSON_PARSER.get() {
             parser(text).map_err(|e| AlbedoError::Json(e))
         } else {
             Err(AlbedoError::Internal(
                 "json_parse do supervisor JIT ainda nao foi conectado ao parser oficial".to_string(),
             ))
         }
     }
     ```
   - Add helper methods `as_string`, `as_object`, `as_number`, and `get` directly onto `impl JsonValue` in `contracts.rs`.

2. **Modify `src/ace/json.rs`**:
   - Replace the local `pub enum JsonValue` definition and its implementation block with:
     ```rust
     pub use albedo_jit::contracts::core::JsonValue;
     ```
   - Define custom `from_serde` and `to_serde` mapping functions to convert between `serde_json::Value` and `JsonValue`.
   - Update `parse` and `stringify` functions to use `from_serde` and `to_serde`.

3. **Modify `src/main.rs`**:
   - In `main()`, after creating the window, register the parser:
     ```rust
     albedo_jit::contracts::core::register_json_parser(albedo::ace::json::parse);
     ```

4. **Modify `albedo-jit/src/runtime/js_value.rs`**:
   - Add `to_number(&self) -> f64` and `to_int32(&self) -> i32` methods on `JsValue` using the standard implementation logic.

5. **Modify JIT builtins & helper modules to use the new JsValue methods**:
   - **`albedo-jit/src/runtime/builtins.rs`**:
     - Remove local `to_number` and `to_int32` helper functions.
     - Update all internal calls to use `val.to_number()` and `val.to_int32()`.
     - Implement `from_u32(id: u32) -> Option<Self>` in `impl BuiltinId` using the matching logic.
   - **`albedo-jit/src/runtime/runtime_helpers.rs`**:
     - Remove local `to_number` and `builtin_from_id` helper functions.
     - Update all internal calls to use `val.to_number()` and `BuiltinId::from_u32(id)`.
   - **`albedo-jit/src/runtime/fast_builtins.rs`**:
     - Remove local `to_number` helper function.
     - Update internal calls to use `val.to_number()`.
   - **`albedo-jit/src/compiler/air_interpreter.rs`**:
     - Remove local `builtin_from_id` helper function.
     - Update internal calls to use `BuiltinId::from_u32(id)`.

## Verification & Testing
- Run `cargo check`, `cargo build`, and `cargo test` inside the workspace to verify compile success, no warning increases, and all tests passing.
- Write your handoff to `c:\Users\24802449\Documents\Github\Albedo-Browser\.agents\teamwork_preview_worker_m2\handoff.md`. Include verification command outputs (cargo test result) in the report.

## Integrity Warnings
MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
