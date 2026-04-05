# I380 — commands.rs Service Extraction Phase 1 — Complete services/ per SERVICE-CONTRACTS.md

**Status:** Open (0.13.2)
**Priority:** P1
**Version:** 0.13.2
**Area:** Code Quality / Refactor

## Summary

`commands.rs` is approximately 11,500 lines containing both IPC command handler boilerplate AND business logic. SERVICE-CONTRACTS.md defines the target architecture: command handlers parse args and call a service; all business logic lives in the service. Phase 1 extracts four core services (ActionService, AccountService, PersonService, MeetingService) from `commands.rs`, moving the business logic while keeping the IPC surface identical — no frontend changes required.

## Acceptance Criteria

From the v0.13.2 brief, verified in the codebase:

1. `services/actions.rs` is complete per the ActionService contract in SERVICE-CONTRACTS.md. All action business logic moved from `commands.rs`. The action handlers in `commands.rs` are: parse args → call service → serialize response. No business logic remains in the handler. `cargo test` passes after each extraction.
2. Same for `services/accounts.rs` — AccountService contract fulfilled.
3. Same for `services/people.rs` — PersonService contract fulfilled.
4. Same for `services/meetings.rs` — MeetingService contract fulfilled.
5. `commands.rs` line count is measurably reduced. Measure before: `wc -l src-tauri/src/commands.rs`. Target after Phase 1: ≤9,000 lines.
6. `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings` passes clean.
7. The IPC surface is unchanged. No commands are renamed, removed, or have their argument shapes changed. The frontend does not need any changes. Verify: `git diff src/` shows no frontend changes.

## Dependencies

- Should be done after I376 (enrichment audit) — audit may surface enrichment paths inside command handlers that need to move to services before extraction.
- Informed by SERVICE-CONTRACTS.md in `.docs/design/`.

## Notes / Rationale

An 11,500-line file cannot be reasoned about as a unit. Every bug fix, every new feature, every reviewer has to hold the entire file in their head to understand the context of any change. The service extraction follows the architecture already documented in SERVICE-CONTRACTS.md — this is implementation catching up to design intent, not new architecture decisions.
