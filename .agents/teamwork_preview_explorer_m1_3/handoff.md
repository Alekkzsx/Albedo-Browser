# Handoff Report — JIT & Cross-Crate Duplication Analysis

## 1. Observation
Direct observations of file paths, line numbers, and contents:

- **`JsonValue` enum**:
  - `albedo-jit/src/contracts.rs`, line 11-18:
    ```rust
    pub enum JsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<JsonValue>),
        Object(HashMap<String, JsonValue>),
    }
    ```
  - `src/ace/json.rs`, line 4-11:
    ```rust
    pub enum JsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<JsonValue>),
        Object(HashMap<String, JsonValue>),
    }
    ```
- **Dummy `json_parse` in JIT**:
  - `albedo-jit/src/contracts.rs`, line 37-41:
    ```rust
    pub fn json_parse(_text: &str) -> AlbedoResult<JsonValue> {
        Err(AlbedoError::Internal(
            "json_parse do supervisor JIT ainda nao foi conectado ao parser oficial".to_string(),
        ))
    }
    ```
- **Real `parse` in Albedo**:
  - `src/ace/json.rs`, line 86-89:
    ```rust
    pub fn parse(s: &str) -> Result<JsonValue, String> {
        let parsed: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        Ok(JsonValue::from(parsed))
    }
    ```
- **`Url` newtype placeholder in JIT**:
  - `albedo-jit/src/contracts.rs`, line 21:
    ```rust
    pub struct Url(pub String);
    ```
- **Real `Url` struct in Albedo**:
  - `src/ace/url/types.rs`, line 52-61:
    ```rust
    pub struct Url {
        pub scheme: String,
        // ... and 7 other fields ...
    }
    ```
- **`AlbedoError` and `AlbedoResult`**:
  - `albedo-jit/src/contracts.rs`, line 24-32.
- **`to_number` function duplicates**:
  - `albedo-jit/src/runtime/builtins.rs`, line 241-257
  - `albedo-jit/src/runtime/runtime_helpers.rs`, line 487-503
  - `albedo-jit/src/runtime/fast_builtins.rs`, line 319-327
- **`builtin_from_id` function duplicates**:
  - `albedo-jit/src/runtime/runtime_helpers.rs`, line 301-345
  - `albedo-jit/src/compiler/air_interpreter.rs`, line 242-286
- **`StringInterner` duplicates**:
  - `albedo-jit/src/runtime/object_model.rs`, line 47-63
  - `src/ace/engine/dom/string_intern.rs`, line 20-86

---

## 2. Logic Chain

1. **Acyclic Dependency Constraint**: The JIT crate `albedo-jit` is a dependency of the main browser crate `albedo`, and therefore `albedo-jit` cannot directly depend on `albedo` without introducing cyclic dependencies.
2. **Ad-hoc Contracts**: To support contracts (like passing deserialized JSON data or URLs) between the browser and JIT runtimes, mock/placeholder versions of `JsonValue`, `Url`, and `AlbedoError` were introduced in `albedo-jit/src/contracts.rs`.
3. **Broken Runtime Behavior**: The mock `json_parse` in `albedo-jit/src/contracts.rs` always returns an internal error. Since the JIT's `JSON.parse` builtins call this method (via `object_model::json_parse`), native JIT executions of JSON string parsing fail silently or return `undefined`, despite a fully functioning `serde_json` parser existing in `src/ace/json.rs`.
4. **Redundant Logic Copy-Pasting**:
   - `to_number` was copy-pasted across three modules inside `albedo-jit` instead of being consolidated as a method on `JsValue` or a common helper.
   - `builtin_from_id` was copy-pasted between two separate modules instead of being consolidated under `BuiltinId` itself.
   - Separate string interners with different internal layouts exist in JIT (`object_model.rs`) and DOM (`string_intern.rs`).

---

## 3. Caveats
- No code modification was made during this investigation (read-only constraint).
- Build compilation was verified via local files, but execution testing (`cargo test`) was not completed due to timing out on system permissions during command execution.
- We assume that the JIT compiler requires its own minimal object model representations for fast native execution (e.g. Nan-boxed `JsValue` and pointer-based `JsObject` shapes), which makes complete unification of JIT/QuickJS/DOM object models unfeasible without complete redesign.

---

## 4. Conclusion
The duplication of `JsonValue`, `Url`, and `AlbedoError` is caused by JIT/host decoupling and results in a broken native JSON parser builtin. Furthermore, internal code copy-pasting of runtime conversions (`to_number`, `builtin_from_id`) leads to maintenance overhead. 

The duplicates can be solved cleanly by:
1. Moving all contract types and parsers to a common/shared crate (e.g. `albedo-core`), or using runtime dependency injection (passing parser callbacks from `albedo` into `albedo-jit`'s engine).
2. Implementing the duplicate helper functions as methods directly on `JsValue` and `BuiltinId` inside the `albedo-jit` crate.

---

## 5. Verification Method

- **Files to Inspect**:
  - `albedo-jit/src/contracts.rs` compared to `src/ace/json.rs` and `src/ace/url/types.rs`.
  - `albedo-jit/src/runtime/runtime_helpers.rs` compared to `albedo-jit/src/runtime/builtins.rs` (for `to_number`) and `albedo-jit/src/compiler/air_interpreter.rs` (for `builtin_from_id`).
- **Test Command**:
  Run `cargo test` in `c:\Users\24802449\Documents\Github\Albedo-Browser` once code modifications/refactors are made to ensure no breaking changes are introduced.
