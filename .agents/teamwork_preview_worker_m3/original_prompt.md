## 2026-06-15T21:36:19Z

You are the Refactoring Worker for Milestone 3.
Your goal is to implement the Core Engine & HTML Parser refactorings to eliminate code duplication and simplify redundant functions.

## Required Tasks

1. **Modify `src/ace/html/types.rs`**:
   - Add the following shared helper functions at the end of the file:
     ```rust
     pub fn is_void_element(tag: &str) -> bool {
         tag.eq_ignore_ascii_case("area")
             || tag.eq_ignore_ascii_case("base")
             || tag.eq_ignore_ascii_case("br")
             || tag.eq_ignore_ascii_case("col")
             || tag.eq_ignore_ascii_case("embed")
             || tag.eq_ignore_ascii_case("hr")
             || tag.eq_ignore_ascii_case("img")
             || tag.eq_ignore_ascii_case("input")
             || tag.eq_ignore_ascii_case("link")
             || tag.eq_ignore_ascii_case("meta")
             || tag.eq_ignore_ascii_case("param")
             || tag.eq_ignore_ascii_case("source")
             || tag.eq_ignore_ascii_case("track")
             || tag.eq_ignore_ascii_case("wbr")
     }

     pub fn parse_next_attribute(html: &str, idx: &mut usize) -> Option<(String, String)> {
         let bytes = html.as_bytes();
         while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
             *idx += 1;
         }
         if *idx >= bytes.len() {
             return None;
         }
         if bytes[*idx] == b'>' || bytes[*idx] == b'/' {
             return None;
         }

         let name_start = *idx;
         while *idx < bytes.len()
             && !bytes[*idx].is_ascii_whitespace()
             && bytes[*idx] != b'='
             && bytes[*idx] != b'>'
             && bytes[*idx] != b'/'
         {
             *idx += 1;
         }

         if *idx == name_start {
             return None;
         }

         let name = html[name_start..*idx].to_ascii_lowercase();

         while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
             *idx += 1;
         }

         let value = if *idx < bytes.len() && bytes[*idx] == b'=' {
             *idx += 1;
             while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
                 *idx += 1;
             }
             if *idx >= bytes.len() {
                 String::new()
             } else if bytes[*idx] == b'"' || bytes[*idx] == b'\'' {
                 let quote = bytes[*idx];
                 *idx += 1;
                 let start = *idx;
                 while *idx < bytes.len() && bytes[*idx] != quote {
                     *idx += 1;
                 }
                 let val = html[start..*idx].to_string();
                 if *idx < bytes.len() {
                     *idx += 1;
                 }
                 val
             } else {
                 let start = *idx;
                 while *idx < bytes.len()
                     && !bytes[*idx].is_ascii_whitespace()
                     && bytes[*idx] != b'>'
                     && bytes[*idx] != b'/'
                 {
                     *idx += 1;
                 }
                 html[start..*idx].to_string()
             }
         } else {
             String::new()
         };

         Some((name, value))
     }
     ```

2. **Modify `src/ace/html/fast_parse.rs`**:
   - Import `is_void_element` and `parse_next_attribute` from `super::types` by adding them to the `use super::types::{...}` block.
   - Delete the local definition of `fn is_void_element` at the bottom of the file.
   - Delete the local definition of `fn parse_fast_attr_value` as it's no longer used.
   - In `parse_fast_start_tag`, replace the attribute loop body where `Some(_)` is matched (approx lines 238-270) to use `parse_next_attribute`:
     ```rust
                 let (attr_name, value) = parse_next_attribute(html, &mut idx)?;
                 attributes.entry(attr_name).or_insert(value);
     ```

3. **Modify `src/ace/html/serializer.rs`**:
   - Import `is_void_element` from `super::types` by adding it to the `use super::types::{...}` block.
   - Delete the local definition of `fn is_void_element` (approx lines 129-134).

4. **Modify `src/ace/html/encoding.rs`**:
   - Add `use super::types::parse_next_attribute;` at the top.
   - Rewrite `parse_meta_attributes` to leverage `parse_next_attribute`:
     ```rust
     pub fn parse_meta_attributes(tag: &str) -> HashMap<String, String> {
         let bytes = tag.as_bytes();
         let mut idx = 0usize;
         let mut attrs = HashMap::new();

         while idx < bytes.len() && bytes[idx] != b' ' && bytes[idx] != b'>' {
             idx += 1;
         }

         while idx < bytes.len() {
             while idx < bytes.len()
                 && (bytes[idx].is_ascii_whitespace() || bytes[idx] == b'/' || bytes[idx] == b'>')
             {
                 idx += 1;
             }
             if idx >= bytes.len() {
                 break;
             }

             if let Some((name, value)) = parse_next_attribute(tag, &mut idx) {
                 attrs.entry(name).or_insert(value);
             } else {
                 break;
             }
         }

         attrs
     }
     ```

5. **Modify `src/ace/engine/dom/mod.rs`**:
   - In imports, add `is_void_element` to `use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode};` so it becomes `use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};`.
   - Delete the local `fn is_void_element(&self, tag: &str) -> bool` method.
   - Replace the call `self.is_void_element(&el.tag)` in `serialize_subtree_html` with `is_void_element(&el.tag)`.

6. **Modify `src/utils/time.rs`**:
   - Add the following time helpers:
     ```rust
     pub fn unix_timestamp_secs_f64() -> f64 {
         SystemTime::now()
             .duration_since(UNIX_EPOCH)
             .unwrap_or_default()
             .as_secs_f64()
     }

     pub fn unix_timestamp_nanos() -> u128 {
         SystemTime::now()
             .duration_since(UNIX_EPOCH)
             .unwrap_or_default()
             .as_nanos()
     }
     ```

7. **Modify `src/ace/engine/mod.rs`**:
   - Add `use crate::utils::time::unix_timestamp_secs_f64;` or use its full path.
   - Replace all `std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()` or similar inline expressions with a call to `unix_timestamp_secs_f64()`.

8. **Modify `src/ace/engine/dom/wpt_harness.rs`**:
   - Add `use crate::utils::time::unix_timestamp_nanos;` or use its full path in `TestTempDir::new()`.
   - Replace the inline `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()` calculation with `unix_timestamp_nanos()`.

9. **Modify `src/ace/json.rs`**:
   - Remove the unused import `use std::collections::HashMap;` from line 1.

## Verification & Testing
- Run target compilation and test suite specifically for `albedo-jit` (to avoid `rquickjs-sys` build script failures):
  ```powershell
  $env:Path = "C:\Program Files\CodeBlocks\MinGW\bin;" + $env:Path
  cargo test -p albedo-jit
  ```
- Make sure that `albedo-jit` compiles and passes all tests (except for the expected isolated test runner failure `test_builtins_math_array_string_json`).
- Write your handoff to `c:\Users\24802449\Documents\Github\Albedo-Browser\.agents\teamwork_preview_worker_m3\handoff.md`. Include verification command outputs in the report.
