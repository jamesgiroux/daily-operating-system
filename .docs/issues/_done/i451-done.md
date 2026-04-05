# I451 — EmailService Expansion — Move Email Handlers from commands.rs to services/emails.rs

**Status:** Open
**Priority:** P1
**Version:** TBD
**Area:** Code Quality / Refactor

## Summary

`commands.rs` contains 6 thick email command handlers that should move to `services/emails.rs`: `get_entity_emails`, `update_email_entity`, `dismiss_email_signal`, `dismiss_email_item`, `archive_low_priority_emails`, and `refresh_emails`. These total ~275 lines. Several handlers emit signals (dismiss_email_signal, dismiss_email_item) that must be preserved exactly during extraction.

## Acceptance Criteria

1. `services/emails.rs` exists (or is expanded if it already exists) with public methods for all 6 handlers:
   - `get_entity_emails()`
   - `update_email_entity()`
   - `dismiss_email_signal()`
   - `dismiss_email_item()`
   - `archive_low_priority_emails()`
   - `refresh_emails()`
2. All 6 command handlers in `commands.rs` are thin wrappers (parse args → call service → return).
3. Signal emissions in `dismiss_email_signal` and `dismiss_email_item` are preserved exactly — same signal type, source, confidence, and use of `emit_signal_and_propagate()` where applicable.
4. IPC surface unchanged — all Tauri commands have identical signatures and return types.
5. `cargo test` passes. `cargo clippy -- -D warnings` passes.
6. ~275 lines removed from `commands.rs`.

## Dependencies

- Builds on the Phase 1 service extraction pattern (I380).
- Signal emission patterns must follow the signal bus integration checklist (see MEMORY.md).
- No dependency on I450, I452–I454.

## Rationale

Email handlers are self-contained domain logic with clear signal emission requirements. Extracting them reduces `commands.rs` size and isolates email business logic for independent testing. The signal-emitting handlers are the most important to get right — signal source and confidence values must not change during extraction.
