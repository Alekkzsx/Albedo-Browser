# Project: Albedo Browser DRY Refactoring

## Architecture
- `albedo-jit`: Just-In-Time compilation crate.
- `src/ace/`: Albedo Core Engine (parser, engine, DOM, CSS style, layout, runtime bindings, networking, etc.).
- `src/browser/`: Browser shell orchestration, tabs, bookmark managers.
- `src/renderer/`: Window management, graphics composition.
- `src/ui/`: Slint-based user interface.
- `src/utils/`: Common helpers (base64, color, crypto, hex, paths, sysinfo, time, uuid).

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Explore & Identify Duplicates | Scan all directories and list code duplications, create a concrete refactoring plan | None | DONE |
| 2 | Refactor Utilities & Contracts | Clean up `contracts.rs` duplicates and unify type conversions/errors/hex/base64 utilities | M1 | DONE |
| 3 | Refactor Core Engine & Runtime | Deduplicate logic in HTML/CSS/DOM engines and JS runtime bindings | M2 | DONE |
| 4 | Refactor Browser, UI & Renderer | Clean up layout, style, rendering, and UI event/tab management boilerplate | M3 | IN_PROGRESS |
| 5 | Network & Infrastructure | Deduplicate resource loading and network request boilerplates | M4 | PLANNED |
| 6 | Verification & Forensic Audit | Run compilation, tests, and forensic audit to ensure correctness and zero regressions | M5 | PLANNED |

## Interface Contracts
### `albedo` ↔ `albedo-jit`
- No cyclic dependencies: `albedo-jit` must not import `albedo`.
- Common contracts defined in `albedo-jit/src/contracts.rs` must be mirrored or re-exported cleanly in `albedo` to avoid duplicated code.
