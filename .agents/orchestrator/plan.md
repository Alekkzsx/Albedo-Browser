# Implementation Plan - DRY Refactoring of Albedo Browser

## Goal
Eliminate code duplication, reduce redundancies, and redesign repetitive functions across the Albedo Browser codebase following the DRY principle.

## Refactoring Milestones

### Milestone 1: Exploration & Identification of Duplicates (Completed)
- Analyzed the codebase using 3 Explorer subagents.
- Identified target areas: JIT/host contracts, JIT internals, main event setup, interaction event dispatch, void HTML elements, IndexedDB config paths, flat map JSON serialization, and epoch time helpers.

### Milestone 2: Refactor JIT Contracts & Internals
- **JIT/Host Contracts (`albedo-jit/src/contracts.rs` & `src/ace/json.rs`)**:
  - Re-export `albedo_jit::contracts::core::JsonValue` in `src/ace/json.rs` to unify types.
  - Implement helper methods (`as_string`, `as_object`, `as_number`, `get`) on `JsonValue` in `albedo-jit/src/contracts.rs`.
  - Add registration of host JSON parser callback via `std::sync::OnceLock` in `contracts.rs` and call it in `src/main.rs` to fix broken JIT JSON parsing.
- **JIT Internals (`albedo-jit/src/runtime/js_value.rs` & `builtins.rs`)**:
  - Implement `to_number` and `to_int32` as methods on `JsValue` in `js_value.rs` and replace duplicate functions in `builtins.rs`, `runtime_helpers.rs`, and `fast_builtins.rs`.
  - Implement `from_u32` on `BuiltinId` in `builtins.rs` and replace duplicate `builtin_from_id` functions in `runtime_helpers.rs` and `air_interpreter.rs`.

### Milestone 3: Refactor Core Engine & HTML Parser
- **Void Elements (`src/ace/html/fast_parse.rs` & `serializer.rs`)**:
  - Centralize `is_void_element` as a single shared function under `src/ace/html/` (e.g., in a new `types.rs` or `mod.rs`).
  - Eliminate heap-allocating `.to_lowercase()` call in `serializer.rs` by using case-insensitive check or lowercase tags.
- **Tag/Attribute parsing**:
  - Deduplicate attribute tokenization loops in `fast_parse.rs` and `encoding.rs`.
- **Time helper (`src/utils/time.rs`)**:
  - Centralize SystemTime epoch conversion logic in `src/utils/time.rs` and replace duplicate inline math in `src/ace/engine/mod.rs` and `wpt_harness.rs`.

### Milestone 4: Refactor UI Events, Tab Closing, & Path Resolving
- **UI Event Setup (`src/main.rs` & `src/browser/events.rs`)**:
  - Define `register_ui_callbacks` in `src/browser/events.rs` to consolidate the event wiring and remove cloning boilerplate from `main.rs`.
- **Interaction dispatch (`src/browser/events.rs`)**:
  - Define `dispatch_interaction` helper in `events.rs` to deduplicate pointer and scroll handlers.
- **Tab Closing Index (`src/browser/tabs/collection.rs`)**:
  - Correct the active tab calculation bug in `TabCollection::close` when `index <= curr`.
- **IndexedDB Path Resolving (`src/ace/runtime/bindings/webapi/indexeddb.rs`)**:
  - Fix double-joining of `"albedo"` path.

### Milestone 5: Refactor Network & Storage Serialization
- **Flat Map JSON Serialization (`cache.rs` & `storage.rs`)**:
  - Replace manual wrapper loops using `JsonValue` enum with direct `serde_json::to_string` and `serde_json::from_str` for `HashMap<String, String>`.
- **Network Clients (`FetchClient` & `ResourceManager`)**:
  - Consolidate common timeout, header and TLS configuration logic.

### Milestone 6: Verification & Forensic Audit
- Verify the entire workspace compiles successfully (`cargo check`, `cargo build`).
- Verify compiler warnings do not increase.
- Verify all unit and integration tests pass successfully (`cargo test`).
- Perform forensic integrity audit with Forensic Auditor.
