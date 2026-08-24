# Project: ace_core Refactoring, Security Hardening & Normative Alignment

## Architecture
`ace_core` is the core foundational engine of Albedo Browser. It provides primitives for memory management, collections, security tokens & origins, networking utilities, the WHATWG event loop, CSS math/color processing, and layout units.

Modules and boundaries:
- `ace_core/src/collections/`: Data structures (`InlineVec`, `TripleBuffer`, `RingBuffer`, etc.)
- `ace_core/src/arena/`: Generational arena allocation (`Arena<T>`, `SlotMap`)
- `ace_core/src/diagnostics/`: Telemetry & breadcrumbs (`BreadcrumbBuffer`)
- `ace_core/src/security/`: Origin, Referrer Policy, UnguessableToken, CSP domain matching
- `ace_core/src/net/`: MIME sniffing, percent decoding, URL helpers
- `ace_core/src/event_loop/`: WHATWG §8.1.6 compliant event loop, task queues, microtask drain, timers
- `ace_core/src/math/`: CSS Color 4/5, Bradford chromatic adaptation, `LayoutUnit` fixed-point math & snapping
- `ace_core/src/flags/`: Bitflags and style change hints to node flag mapping

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | `InlineVec` Memory Shift Fix | Fix inverted range `(*len..index).rev()` in `insert()` using `ptr::copy` | M1 | R2 Survey |
| 2 | `InlineVec` Double Free Fix | Fix `retain()` bit duplication & illegal drop using `RetainGuard` and `vec.retain` | M1 | R2 Survey |
| 3 | `TripleBuffer` Lock-Free Zero-Copy | Implement atomic 3-state transition (`AtomicU8`) & `UnsafeCell` zero-copy | M1 | R2 Survey |
| 4 | `Arena<T>` Deterministic `clear()` | Clean `free_list` and rebuild `(0..len).rev()` without slot duplication | M1 | R2 Survey |
| 5 | `BreadcrumbBuffer` $O(1)$ Deque & Zero Guard | Switch to `VecDeque` with `CAPACITY == 0` guard | M1 | R2 Survey |
| 6 | `UnguessableToken` CSPRNG | Use OS CSPRNG (`getrandom`) for 128-bit unguessable tokens (CWE-330) | M2 | R3 Survey |
| 7 | `compute_referrer` RFC 9110 / CWE-200 | Use `same_origin()` comparison and strip `userinfo` (credentials) | M2 | R3 Survey |
| 8 | `Origin` Serialization & File Isolation | Omit `:0` port on file/custom schemes per RFC 6454 | M2 | R3 Survey |
| 9 | `matches_domain_pattern` CSP3 §6.7.2 | Wildcard `*.domain.com` matches only subdomains, not apex domain | M2 | R3 Survey |
| 10 | `percent_decode` Byte Preservation | Preserve bytes on malformed `%` sequences (e.g. `"100%_concluido"`) | M2 | R3 Survey |
| 11 | `sniff_mime_type` UTF-8 Boundary | Handle multi-byte UTF-8 split at 512-byte boundary gracefully | M2 | R3 Survey |
| 12 | Event Loop Starvation Prevention | Fair queuing with starvation counters across all `TaskSource` queues | M3 | R4 Survey |
| 13 | Microtask Checkpoint Reentrancy Guard | Implement `performing_microtask_checkpoint` guard per WHATWG §8.1.6.3 | M3 | R4 Survey |
| 14 | Atomic Timer Macrotask Dispatch | Enqueue expired timers as individual `TaskSource::Timer` macrotasks | M3 | R4 Survey |
| 15 | Dynamic Timer Nesting Clamping | Propagate nesting depth via TLS and enforce 4ms minimum clamp for depth >= 5 | M3 | R4 Survey |
| 16 | Bradford Chromatic Adaptation | Implement D65 <-> D50 adaptation matrices for sRGB <-> Lab/Lch | M4 | R5 Survey |
| 17 | CSS Color 4 Polar Interpolation | Implement 4 hue methods, powerless component handling & alpha premul | M4 | R5 Survey |
| 18 | `LayoutUnit` Box Snapping | Box snapping with zero pixel cracking ($\text{right}_1 \equiv \text{left}_2$) | M4 | R5 Survey |
| 19 | `style_hint_to_node_flags` Mapping | Map `StyleChangeHint::SUBTREE_RECALC` -> `NodeFlags::SUBTREE_DIRTY` | M4 | R5 Survey |
| 20 | Full Workspace Verification & E2E Validation | Run all 80+ unit/integration tests, zero clippy warnings, memory integrity | M5 | Acceptance Criteria |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Memory Soundness & Collections (R2) | `collections/inline_vec.rs`, `triple_buffer.rs`, `arena/slab.rs`, `diagnostics/breadcrumbs.rs` | none | IN_PROGRESS |
| M2 | Web Security & Net Hardening (R3) | `Cargo.toml`, `security/token.rs`, `referrer.rs`, `origin.rs`, `utils.rs`, `net/utils.rs`, `net/mime.rs` | none | PLANNED |
| M3 | WHATWG Event Loop Alignment (R4) | `event_loop/mod.rs`, `event_loop/source.rs` | none | PLANNED |
| M4 | W3C Math, Color & LayoutUnit (R5) | `math/color.rs`, `math/layout_unit.rs`, `flags/utils.rs` | none | PLANNED |
| M5 | Comprehensive Verification & Hardening | Full workspace tests, clippy check, regression tests, adversarial audit | M1, M2, M3, M4 | PLANNED |

## Interface Contracts
### `TripleBuffer` Consumer Contract
- Producer: `write(&mut self, value: T)` writes to back buffer and atomically updates shared state.
- Consumer: `consume(&mut self) -> Option<&T>` or `read(&self) -> &T` returns zero-copy reference without heap allocations.

### `LayoutUnit` Box Snapping Contract
- `snap_box(origin: LayoutUnit, size: LayoutUnit) -> (i32, i32)`
- Invariant: `origin.snap_box(s1).0 + origin.snap_box(s1).1 == (origin + s1).snap_box(s2).0`.

### `UnguessableToken` Contract
- `UnguessableToken::new() -> Self` must use OS CSPRNG (`getrandom::getrandom`) and never return all-zeros.

## Code Layout
- `ace_core/Cargo.toml`: crate dependencies (`getrandom = "0.2"`)
- `ace_core/src/collections/inline_vec.rs`: `InlineVec<T, N>`
- `ace_core/src/collections/triple_buffer.rs`: `TripleBuffer<T>`
- `ace_core/src/arena/slab.rs`: `Arena<T>`
- `ace_core/src/diagnostics/breadcrumbs.rs`: `BreadcrumbBuffer`
- `ace_core/src/security/token.rs`: `UnguessableToken`
- `ace_core/src/security/referrer.rs`: `compute_referrer`
- `ace_core/src/security/origin.rs`: `Origin`
- `ace_core/src/security/utils.rs`: `matches_domain_pattern`
- `ace_core/src/net/utils.rs`: `percent_decode`
- `ace_core/src/net/mime.rs`: `sniff_mime_type`
- `ace_core/src/event_loop/mod.rs`: `EventLoop`, `TaskSourceQueues`, `drain_microtasks`
- `ace_core/src/event_loop/source.rs`: `TaskSource`
- `ace_core/src/math/color.rs`: `Color`, `ColorSpace`, Bradford D65<->D50, polar interpolation
- `ace_core/src/math/layout_unit.rs`: `LayoutUnit::snap_box`
- `ace_core/src/flags/utils.rs`: `style_hint_to_node_flags`
