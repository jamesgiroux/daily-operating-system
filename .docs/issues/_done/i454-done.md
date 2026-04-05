# I454 — SettingsService Extraction — Create services/settings.rs for Settings Handlers

**Status:** Open
**Priority:** P1
**Version:** TBD
**Area:** Code Quality / Refactor

## Summary

`commands.rs` contains 7 thick settings command handlers with no corresponding service module: `set_entity_mode`, `set_workspace_path`, `set_ai_model`, `set_hygiene_config`, `set_schedule`, `set_user_profile`, and `set_user_domains`. These total ~302 lines. A new `services/settings.rs` should be created following the established service pattern.

## Acceptance Criteria

1. `services/settings.rs` exists as a new file with public methods for all 7 handlers:
   - `set_entity_mode()`
   - `set_workspace_path()`
   - `set_ai_model()`
   - `set_hygiene_config()`
   - `set_schedule()`
   - `set_user_profile()`
   - `set_user_domains()`
2. `services/mod.rs` exports the new settings module.
3. All 7 command handlers in `commands.rs` are thin wrappers (parse args → call service → return).
4. IPC surface unchanged — all Tauri commands have identical signatures and return types.
5. `cargo test` passes. `cargo clippy -- -D warnings` passes.
6. ~302 lines removed from `commands.rs`.

## Dependencies

- Builds on the Phase 1 service extraction pattern (I380).
- Pattern reference: `services/accounts.rs` for `&AppState` method signatures.
- No dependency on I450–I453.

## Rationale

Settings handlers are the last major domain without a service module. Unlike the other I450-series issues which expand existing services, this creates a new service file. The handlers are straightforward (validate input → write to DB/config → return) with no signal emissions or complex transactions, making this the lowest-risk extraction in the batch. Completing it brings the full settings domain under the service layer.
