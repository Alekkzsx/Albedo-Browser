# Project: Albedo Browser — ace_dom Subsystem Audit & Master Plan

## Architecture
`ace_dom` is the foundational DOM and HTML5 parsing subsystem of the Albedo Browser engine.
- **Memory & Storage**: Generational slotmap arena (`Arena<NodeData>`) backed by `ace_core`, using compact 64-bit generational `NodeId` references (0 cyclic leaks, $O(1)$ amortized teardown, 100% Safe Rust).
- **HTML5 Parser**: WHATWG §12 streaming tokenizer with SIMD fast paths (`memchr3`), state machine, Tree Builder with 16-step Adoption Agency Algorithm (AAA) + Noah's Ark clause, Declarative Shadow DOM (DSD), and Foreign Content (SVG/MathML) fixup tables.
- **CSS4 Selector Engine**: Right-to-Left (RTL) compound selector matcher, 64-bucket Counting Ancestor Bloom Filter (`AncestorFilter`), and rule-partitioned `RuleBucketIndex`.
- **DOM Features**: MutationObserver, Live Range boundary auto-adjustment, Form Validity (10-flag `ValidityState`), HTML Sanitizer, and unified GC Tracing (`GcTracer` / `Traceable`).

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Tokenizer & FSM Compliance | WHATWG §12.2 Tokenizer FSM, entity decoding, ambiguous ampersand in attributes, script escaping states, doctype identifiers | M1 | Survey (explorer_1) |
| 2 | Tree Builder & 16-Step AAA | Full 16-step Adoption Agency Algorithm, bookmark shifting fix, Noah's ark limit, DSD (<template shadowrootmode>), Foreign Content (SVG/MathML) | M1 | Survey (explorer_1) |
| 3 | Form Validity & Associations | 10-flag ValidityState, form attribute association, checkValidity/reportValidity | M1 | Survey (explorer_1) |
| 4 | Memory Density & Struct Layout | NodeData (144B -> 88B via boxed Doctype/Document data), ElementData (inline attributes/classes), TextData (SmolStr 24B), exact byte breakdown | M2 | Survey (explorer_2) |
| 5 | GC Tracing & Soundness | Generational arena lifecycle, non-owning NodeId handles, GcTracer trait, cycle-freedom, 0 unsafe blocks, Send+Sync | M2 | Survey (explorer_2) |
| 6 | Engine Comparative Matrix | Detailed architectural comparison vs Blink (Oilpan/Compact DOM), WebKit (JSC GC), Gecko (Stylo/nsINode), Ladybird (LibWeb GC), Servo (DomRef) | M2 | Survey (explorer_2) |
| 7 | CSS4 Selectors & Bloom Filter | RTL selector matching, :is(), :where(), :has(), :not(), Ancestor Bloom Filter fast rejection pipeline | M3 | Survey (explorer_3) |
| 8 | MutationObserver & Live Ranges | Mutation records, subtree observation, microtask delivery, LiveRangeRegistry boundary points auto-adjustment, LCA-based comparison | M3 | Survey (explorer_3) |
| 9 | HTML Sanitizer | Configurable element/attribute allowlists/denylists, event handler purging, javascript: URI filtering, node unwrapping | M3 | Survey (explorer_3) |
| 10 | Asymptotic Complexity Optimizations | O(1) getElementById via ElementIndex, O(depth) LCA position comparison, O(1) Bloom filter rejection | M3 | Survey (explorer_3) |
| 11 | Clippy & Test Suite Hardening | Fix 2 clippy warnings in hardening_phase5_master_test.rs, ensure cargo clippy --workspace --all-targets --all-features -- -D warnings = 0 warnings, 100% tests pass | M4 | Survey (all explorers) |
| 12 | Definitive Master Plan Consolidation | Deliver complete Master Plan with Quick Wins, Architectural Refactorings, SIMD/Zero-Copy, WPT Hardening, and Technical Audit Report | M5 | Synthesis & Final Gate |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | WHATWG HTML §12 & Tree Construction Audit | Tokenizer states, entity decoding, AAA step 3.17 bookmark fix, DSD, SVG/MathML | None | DONE (Explored) |
| M2 | Memory Soundness & Competitive SOTA Analysis | Exact byte footprint per node, GC cycle-freedom, 100% Safe Rust invariants, Comparative Matrix | None | DONE (Explored) |
| M3 | Query, Observer, Range & Asymptotic Complexity | CSS4 RTL engine, Ancestor Bloom Filter, MutationObserver hooks, Live Range LCA, Sanitizer | None | DONE (Explored) |
| M4 | Clippy & Test Suite Remediation | Fix clippy warnings in `tests/hardening_phase5_master_test.rs`, run full workspace clippy & test validation | M1, M2, M3 | DONE (0 warnings, 100% pass) |
| M5 | Master Plan & Technical Report Consolidation | Deliver `ACE_DOM_MASTER_PLAN.md`, verify via Reviewer, Challenger, and Auditor | M4 | DONE (Pass Gate) |

## Interface Contracts
### `ace_dom` ↔ `ace_core`
- `Arena<NodeData>`: Generational slotmap for node allocation and indexing. `NodeId` = 64-bit `(version: u32, index: u32)`.
- `InlineVec<T, N>`: Small-vector optimization (0 heap allocations for $\le N$ elements).
- `Atom`: Fast interned atomic strings for tag names and attribute names.

### `ace_dom` ↔ `ace_css` & `ace_style`
- `ComplexSelector`: AST for CSS selectors compiled from selector strings.
- `AncestorFilter`: 64-bucket counting Bloom filter pushed/popped during DOM tree traversal for $O(1)$ fast rejection.
- `RuleBucketIndex`: Rule index partitioned by ID, Class, Tag, and Universal selectors.

### `ace_dom` ↔ `ace_js`
- `GcTracer`: Tri-color GC visitor trait for marking reachable DOM nodes from JS root objects without cyclic memory retention.

## Code Layout
- `Albedo_Core_Engine/ace_dom/src/lib.rs`: Subsystem root and public exports.
- `Albedo_Core_Engine/ace_dom/src/node/`: `NodeData`, `ElementData`, `TextData`, `CommentData`, `DocumentData`, `DoctypeData`.
- `Albedo_Core_Engine/ace_dom/src/tree/`: Tree mutations (`append_child`, `insert_before`, `remove_child`, `replace_child`), iterators (`descendants`, `ancestors`, `children`).
- `Albedo_Core_Engine/ace_dom/src/tokenizer/`: HTML5 tokenizer, state machine, entity decoder, SIMD fast-path.
- `Albedo_Core_Engine/ace_dom/src/tree_builder/`: HTML5 tree construction, insertion modes, AAA, DSD, foreign content.
- `Albedo_Core_Engine/ace_dom/src/query/`: CSS selectors, Ancestor Bloom Filter, `ElementIndex`, `RuleBucketIndex`.
- `Albedo_Core_Engine/ace_dom/src/observer/`: MutationObserver and mutation records.
- `Albedo_Core_Engine/ace_dom/src/range/`: Live Range, boundary points, `LiveRangeRegistry`.
- `Albedo_Core_Engine/ace_dom/src/form/`: Form validity and association algorithms.
- `Albedo_Core_Engine/ace_dom/src/sanitizer/`: HTML Sanitizer API.
- `Albedo_Core_Engine/ace_dom/src/gc.rs`: Tracing GC integration (`GcTracer`, `Traceable`).
- `Albedo_Core_Engine/ace_dom/tests/`: Integration test suites and hardening tests.
