# I450 — ProjectService Extraction — Move Project Handlers from commands.rs to services/projects.rs

**Status:** Open
**Priority:** P1
**Version:** TBD
**Area:** Code Quality / Refactor

## Summary

`commands.rs` still contains 6 thick project command handlers with business logic that belongs in the service layer: `get_projects_list`, `create_project`, `update_project_field`, `update_project_notes`, `bulk_create_projects`, and `archive_project`. These total ~276 lines. Moving them to `services/projects.rs` follows the pattern established by `services/accounts.rs` — methods take `&AppState` when they need config/workspace access.

## Acceptance Criteria

1. `services/projects.rs` exists with public methods for all 6 handlers:
   - `get_projects_list()`
   - `create_project()`
   - `update_project_field()`
   - `update_project_notes()`
   - `bulk_create_projects()`
   - `archive_project()`
2. All 6 command handlers in `commands.rs` are thin wrappers (parse args → call service → return).
3. IPC surface unchanged — all Tauri commands have identical signatures and return types.
4. `cargo test` passes. `cargo clippy -- -D warnings` passes.
5. ~276 lines removed from `commands.rs`.

## Dependencies

- Builds on the Phase 1 service extraction pattern (I380).
- Pattern reference: `services/accounts.rs` for `&AppState` method signatures.
- No dependency on I451–I454.

## Rationale

`commands.rs` is the largest file in the backend and a frequent merge conflict zone. Extracting project handlers to their own service module improves readability, reduces coupling, and makes the project domain independently testable. This is a mechanical refactor with no behavioral change.
