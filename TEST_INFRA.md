# E2E Test Infra: ace_core Engine

## Test Philosophy
- Opaque-box, requirement-driven testing based on `ORIGINAL_REQUEST.md`, W3C/WHATWG/RFC standards, and `PROJECT.md`.
- No reliance on internal non-public implementation details; exercises public types and functions exported by `ace_core`.
- 4-Tier Test Architecture:
  - **Tier 1: Feature Coverage (>=5 test cases per feature)**: Basic isolated functionality for all 19 features.
  - **Tier 2: Boundary & Corner Cases (>=5 test cases per feature)**: Extreme values, empty buffers, limits, malformed inputs, reentrancy.
  - **Tier 3: Cross-Feature Combinations (pairwise interactions)**: Multi-module integration (e.g. Origin + Referrer + Security Token, Event Loop + Timers + Microtasks, Color interpolation + LayoutUnit snapping).
  - **Tier 4: Real-World Application Scenarios (>=10 realistic workloads)**: Complex browser pipelines (navigation requests, full event loop load, animation triple buffering, CSS styling and node invalidation).

## Feature Inventory & Test Matrix
| # | Feature | Source | Tier 1 (Req >=5) | Tier 2 (Req >=5) | Tier 3 (Pairwise) | Tier 4 (Workloads) |
|---|---------|--------|:----------------:|:----------------:|:-----------------:|:------------------:|
| 1 | `InlineVec` Memory Shift Fix | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 2 | `InlineVec` Double Free Fix | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 3 | `TripleBuffer` Lock-Free Zero-Copy | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 4 | `Arena<T>` Deterministic `clear()` | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 5 | `BreadcrumbBuffer` O(1) Deque & Zero Guard | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 6 | `UnguessableToken` CSPRNG | ORIGINAL_REQUEST §R3 | 5 | 5 | ✓ | ✓ |
| 7 | `compute_referrer` RFC 9110 / CWE-200 | ORIGINAL_REQUEST §R3 | 5 | 5 | ✓ | ✓ |
| 8 | `Origin` Serialization & File Isolation | ORIGINAL_REQUEST §R3 | 5 | 5 | ✓ | ✓ |
| 9 | `matches_domain_pattern` CSP3 §6.7.2 | ORIGINAL_REQUEST §R3 | 5 | 5 | ✓ | ✓ |
| 10 | `percent_decode` Byte Preservation | ORIGINAL_REQUEST §R3 | 5 | 5 | ✓ | ✓ |
| 11 | `sniff_mime_type` UTF-8 Boundary | ORIGINAL_REQUEST §R3 | 5 | 5 | ✓ | ✓ |
| 12 | Event Loop Starvation Prevention | ORIGINAL_REQUEST §R4 | 5 | 5 | ✓ | ✓ |
| 13 | Microtask Checkpoint Reentrancy Guard | ORIGINAL_REQUEST §R4 | 5 | 5 | ✓ | ✓ |
| 14 | Atomic Timer Macrotask Dispatch | ORIGINAL_REQUEST §R4 | 5 | 5 | ✓ | ✓ |
| 15 | Dynamic Timer Nesting Clamping | ORIGINAL_REQUEST §R4 | 5 | 5 | ✓ | ✓ |
| 16 | Bradford Chromatic Adaptation | ORIGINAL_REQUEST §R5 | 5 | 5 | ✓ | ✓ |
| 17 | CSS Color 4 Polar Interpolation | ORIGINAL_REQUEST §R5 | 5 | 5 | ✓ | ✓ |
| 18 | `LayoutUnit` Box Snapping | ORIGINAL_REQUEST §R5 | 5 | 5 | ✓ | ✓ |
| 19 | `style_hint_to_node_flags` Mapping | ORIGINAL_REQUEST §R5 | 5 | 5 | ✓ | ✓ |

## Test Architecture & Layout
- `ace_core/tests/e2e_tier1_features.rs`: Contains isolated functional tests for all 19 features (>=95 tests).
- `ace_core/tests/e2e_tier2_boundaries.rs`: Contains boundary, corner, overflow, and zero-case tests for all 19 features (>=95 tests).
- `ace_core/tests/e2e_tier3_combinations.rs`: Contains cross-module integration tests for feature pairs (>=19 tests).
- `ace_core/tests/e2e_tier4_workloads.rs`: Contains realistic browser workload simulation tests (>=10 scenarios).

## Real-World Application Scenarios (Tier 4)
1. **Full Browser Navigation Pipeline**: Origin parsing, referrer policy resolution with credential stripping, token generation, MIME sniffing and percent decoding.
2. **High-Frequency Animation Frame Producer-Consumer**: 120 FPS rendering pipeline using `TripleBuffer` with color conversions and layout unit edge snapping.
3. **Heavy Event Loop Under Load**: Interleaved DOM events, network tasks, microtasks, and nested timers without starvation.
4. **Style Change Recalculation & Arena Tree Mutation**: Batch node allocation in `Arena`, style hint flag calculation, breadcrumb recording, and clean teardown.
5. **Security Origin & CSP Evaluation Pipeline**: Multi-origin sandboxing, wildcard domain matching against malicious vectors, and token generation.
6. **CSS Color 4/5 Space Transformation Pipeline**: sRGB -> Lab with Bradford D65<->D50 adaptation, polar interpolation in Oklch across hue boundaries, and alpha compositing.
7. **DOM Tree Lifecycle & Memory Stress**: Rapid allocation, node destruction, arena reuse, inline vector manipulation with Miri/Valgrind soundness validation.
8. **Network Stream MIME Boundary Sniffing**: Chunked network stream decoding with multi-byte UTF-8 split at 512-byte boundaries and percent-encoded query parsing.
9. **Timer Storm & Microtask Drain Reentrancy**: Recursive timer nesting triggering 4ms clamping and microtask recursion with reentrancy prevention.
10. **Composite Multi-Column Layout Snapping**: Adjacent layout boxes snapping to exact physical pixels with zero pixel cracking.

## Coverage Minimum Thresholds
- Tier 1: 95 tests (5 × 19 features)
- Tier 2: 95 tests (5 × 19 features)
- Tier 3: 19 tests (cross-feature pairs)
- Tier 4: 10 tests (application-level workloads)
- **Total Minimum: 219 E2E Test Cases**
