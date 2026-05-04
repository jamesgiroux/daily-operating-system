# I514 — Backend Module Decomposition (commands/db Boundary)

**Priority:** P1  
**Area:** Backend / Architecture  
**Version:** v1.0.0 (Phase 3a)  
**Depends on:** I512 (ServiceLayer mandatory path)  
**Blocks:** Phase 3 backend maintainability signoff

## Problem

`commands.rs` and `db/mod.rs` still carry god-module characteristics:
1. High change coupling across unrelated domains.
2. Review and test blast radius is larger than the behavior being changed.
3. Boundaries are unclear, encouraging direct DB access from command handlers.
4. File size and mixed concerns increase regression risk for v1.0.0 hardening work.

## Design

### 1. Domain command split

- Split `commands.rs` into domain-scoped command modules (accounts, projects, people, meetings, inbox/emails, settings/system, reports, recovery/devtools as needed).
- Keep a thin registration/re-export layer to preserve Tauri command discovery and avoid route-level churn.
- Move shared helpers to dedicated support modules (not back into another god file).

### 2. DB module split

- Make `db/mod.rs` a re-export hub only.
- Move query/write logic into domain modules aligned with command/service boundaries.
- Keep naming explicit and searchable (domain-first module names).

### 3. Boundary enforcement

- Command handlers call ServiceLayer for mutations; no direct SQL writes in handlers.
- Query helpers remain in db domain modules; no cross-domain hidden coupling.
- Add/update enforcement checks where practical (existing service-layer boundary checks remain mandatory).

### 4. Size and ownership constraints

- `commands.rs` target: routing/exports only, no domain logic accumulation.
- `db/mod.rs` target: module declarations + exports only.
- New modules include focused tests where behavior moved could regress.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/commands.rs` | Convert to thin module hub / command registration |
| `src-tauri/src/commands/*.rs` (new/updated) | Domain command handlers by area |
| `src-tauri/src/db/mod.rs` | Re-export hub only |
| `src-tauri/src/db/*.rs` | Domain query modules and helpers |
| `scripts/check_service_layer_boundary.sh` (if needed) | Keep mutation boundary enforcement current |

## Acceptance Criteria

1. `commands.rs` no longer contains mixed domain implementations; it acts as registration/dispatch hub.
2. `db/mod.rs` is a re-export hub; domain logic lives in domain modules.
3. No new direct DB mutation paths are introduced in command handlers.
4. Existing command behavior remains unchanged (no contract regressions).
5. Moved logic has regression coverage (unit/integration tests where behavior changed).
6. `cargo test` passes.
7. `cargo clippy --workspace --all-features --lib --bins -- -D warnings` passes.

## Out of Scope

- New product features (search/offline/export/privacy/editorial)
- Retry/circuit-breaker work (I515)
- Schema redesign and migration runner safety work (I511)
