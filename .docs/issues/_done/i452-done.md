# I452 — AccountService Expansion — Move Remaining Account Handlers to services/accounts.rs

**Status:** Open
**Priority:** P1
**Version:** TBD
**Area:** Code Quality / Refactor

## Summary

Three thick account-related command handlers remain in `commands.rs` that should move to `services/accounts.rs`: `create_internal_organization` (~182 lines with transaction), `create_child_account` (the command handler, not the record-creation function which is already extracted), and `backfill_internal_meeting_associations`. These total ~249 lines.

`create_internal_organization` is the largest single handler — it runs a multi-step transaction that must be preserved atomically during extraction.

## Acceptance Criteria

1. `services/accounts.rs` is expanded with public methods for all 3 handlers:
   - `create_internal_organization()` — transaction preserved exactly
   - `create_child_account()` — command handler logic only (record function already in service)
   - `backfill_internal_meeting_associations()`
2. All 3 command handlers in `commands.rs` are thin wrappers (parse args → call service → return).
3. The `create_internal_organization` transaction boundary is preserved — all steps within a single DB transaction, rollback on any failure.
4. IPC surface unchanged — all Tauri commands have identical signatures and return types.
5. `cargo test` passes. `cargo clippy -- -D warnings` passes.
6. ~249 lines removed from `commands.rs`.

## Dependencies

- Builds on the existing `services/accounts.rs` established in Phase 1 (I380).
- No dependency on I450, I451, I453, I454.

## Rationale

The account service already exists from Phase 1 extraction. These 3 remaining handlers were left behind due to complexity (transaction logic, handler-vs-record distinction). Completing the extraction gives the account domain a single service boundary. The `create_internal_organization` transaction is the key risk — it must remain atomic.
