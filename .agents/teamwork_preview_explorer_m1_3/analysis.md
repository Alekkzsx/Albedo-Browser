# Codebase Duplication Analysis Report

## Summary of Findings
This analysis focuses on the `albedo-jit` sub-crate and its interaction with the `albedo` crate to identify code duplication, redundant definitions, and placeholder implementation side-effects. Several key instances of duplicated logic and types were found:
1. **Cross-Crate Contract Duplication (`JsonValue`, `Url`, `AlbedoError`)**: Duplicated or dummy definitions exist in `albedo-jit/src/contracts.rs` to break cyclic dependencies, causing JIT's `JSON.parse` builtin to be completely disconnected from the actual parser.
2. **JIT Internal Duplication (`to_number`, `builtin_from_id`)**: Identical utility functions are repeated across multiple runtime and compiler modules within JIT.
3. **Conceptual Redundancies (`StringInterner` and Object Models)**: Both crates implement separate interner systems and distinct object models.

---

## 1. Cross-Crate Contract Duplication

Because the main package `albedo` depends on `albedo-jit`, the JIT crate cannot depend back on `albedo`. To circumvent this, the authors defined a "contract supervisor" module in `albedo-jit/src/contracts.rs`. However, this has led to copy-pasted structures and dummy, broken implementations.

### 1.1 `JsonValue`
- **Location 1**: `albedo-jit/src/contracts.rs` (lines 11-18)
- **Location 2**: `src/ace/json.rs` (lines 4-11)

#### Structural Comparison:
Both crates declare `JsonValue` as:
```rust
// JIT contracts version (has PartialEq derive)
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

// Albedo main package version
#[derive(Clone, Debug)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}
```

#### The Dummy Parser Issue:
In `albedo-jit/src/contracts.rs` (lines 37-41):
```rust
pub fn json_parse(_text: &str) -> AlbedoResult<JsonValue> {
    Err(AlbedoError::Internal(
        "json_parse do supervisor JIT ainda nao foi conectado ao parser oficial".to_string(),
    ))
}
```
In JIT's `object_model.rs` (lines 447-461), `json_parse` calls `contracts::core::json_parse`, meaning JIT's native execution of `JSON.parse` is broken and always evaluates to `JsValue::undefined()`:
```rust
pub fn json_parse(s: JsValue) -> JsValue {
    // ...
    match crate::contracts::core::json_parse(&text) {
        Ok(v) => json_to_jsvalue(&v),
        Err(_) => JsValue::undefined(),
    }
}
```
Meanwhile, `src/ace/json.rs` contains a real parser using `serde_json`:
```rust
pub fn parse(s: &str) -> Result<JsonValue, String> {
    let parsed: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
    Ok(JsonValue::from(parsed))
}
```

### 1.2 `Url`
- **Location 1**: `albedo-jit/src/contracts.rs` (line 21)
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct Url(pub String);
  ```
- **Location 2**: `src/ace/url/types.rs` (lines 52-61)
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct Url {
      pub scheme: String,
      pub username: String,
      pub password: Option<String>,
      pub host: Option<Host>,
      pub port: Option<u16>,
      pub path: Vec<String>,
      pub query: Option<String>,
      pub fragment: Option<String>,
  }
  ```
- **Analysis**: The JIT's version of `Url` is a newtype wrapper around a `String` designed to resemble a contract type, but JIT does not actually use this `Url` structure anywhere in its codebase.

### 1.3 `AlbedoError` and `AlbedoResult`
- **Location 1**: `albedo-jit/src/contracts.rs` (lines 24-32)
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub enum AlbedoError {
      Json(String),
      Url(String),
      Network(String),
      Jit(String),
      Internal(String),
  }
  ```
- **Analysis**: This error type only exists as a JIT contract placeholder. The main package `albedo` does not define or import it (it relies on `rquickjs::Error` or standard errors).

---

## 2. JIT Internal Duplication

Within `albedo-jit`, several helper functions are copy-pasted across different modules.

### 2.1 `to_number` Helper Function
The conversion function from `JsValue` to `f64` is declared three times:

- **Module 1**: `albedo-jit/src/runtime/builtins.rs` (lines 241-257)
- **Module 2**: `albedo-jit/src/runtime/runtime_helpers.rs` (lines 487-503)
  *Implementation for Modules 1 & 2 is identical:*
  ```rust
  fn to_number(v: JsValue) -> f64 {
      if v.is_int32() {
          v.as_int32() as f64
      } else if v.is_float64() {
          v.as_float64()
      } else if v.is_bool() {
          if v.as_bool() { 1.0 } else { 0.0 }
      } else if v.is_null() {
          0.0
      } else {
          f64::NAN
      }
  }
  ```
- **Module 3**: `albedo-jit/src/runtime/fast_builtins.rs` (lines 319-327)
  *Implementation in Module 3 is a simplified subset:*
  ```rust
  fn to_number(v: JsValue) -> f64 {
      if v.is_int32() {
          v.as_int32() as f64
      } else if v.is_float64() {
          v.as_float64()
      } else {
          f64::NAN
      }
  }
  ```

### 2.2 `builtin_from_id` Helper Function
The conversion function mapping raw `u32` IDs to `BuiltinId` enums is copy-pasted verbatim between two modules:

- **Module 1**: `albedo-jit/src/runtime/runtime_helpers.rs` (lines 301-345)
- **Module 2**: `albedo-jit/src/compiler/air_interpreter.rs` (lines 242-286)

Both modules duplicate a 45-line matching pattern:
```rust
fn builtin_from_id(id: u32) -> Option<BuiltinId> {
    use BuiltinId::*;
    Some(match id {
        x if x == MathAbs as u32 => MathAbs,
        // ... all 39 math/array/string builtins repeated ...
        _ => return None,
    })
}
```

---

## 3. Conceptual & Systemic Redundancies

### 3.1 `StringInterner`
- **JIT Version**: `albedo-jit/src/runtime/object_model.rs` (lines 47-63)
  Uses a `HashMap<String, u32>` and a global static `RwLock<StringInterner>` to map strings to numerical IDs for Nan-boxed representation.
- **Main Package Version**: `src/ace/engine/dom/string_intern.rs` (lines 20-86)
  Uses `HashMap<Arc<str>, usize, FxBuildHasher>` and a global static `Mutex<StringInterner>` for fast DOM string de-duplication.
- **Analysis**: While the two serve slightly different roles (JIT needs `u32` for `JsValue` payload encoding, DOM uses `Arc<str>` for tag names/attrs), they represent two distinct implementations of string interning in the same application.

### 3.2 Dual Object Models
- **JIT Version**: Defines `JsObject` in `albedo-jit/src/runtime/object_model.rs` with shapes/hidden-classes and custom property vectors.
- **QuickJS / DOM Version**: QuickJS (via `rquickjs`) has its own object heap and property layout, and `src/ace/engine/dom/` defines a complete DOM hierarchy (with elements, styles, etc.).
- **Analysis**: The JIT runtime operates on its own raw pointer-based `JsObject` layout, which requires complex serialization and mapping at boundary crossings (e.g. OSR spill buffers).

---

## Proposed Remediation Paths

To address this duplication without introducing circular crate dependencies:

1. **Shared Core Crate (`albedo-core` or `albedo-common`)**:
   Extract all contract types (`JsonValue`, `Url`, `AlbedoError`), the thread-safe `StringInterner`, and common JSON/URL parsers into a new minimal dependency crate. Both `albedo-jit` and `albedo` can then import this shared library.
2. **Dependency Injection / Function Pointers**:
   Introduce a contract registration interface in JIT (`albedo-jit` exposing a function/struct registry). The main package `albedo` can inject the official `json_parse` and other contract helpers during JIT initialization.
3. **Consolidate JIT Helper Logic**:
   - Refactor `to_number` as a public method directly on `JsValue` (in `js_value.rs`).
   - Implement `TryFrom<u32>` or a `from_id` method directly on `BuiltinId` (in `builtins.rs`) to replace the duplicate `builtin_from_id` functions.
