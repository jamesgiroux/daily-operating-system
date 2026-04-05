# I453 — MeetingService Expansion — Move Remaining Meeting Handlers to services/meetings.rs

**Status:** Open
**Priority:** P1
**Version:** TBD
**Area:** Code Quality / Refactor

## Summary

Two thick meeting command handlers remain in `commands.rs` that should move to `services/meetings.rs`: `refresh_meeting_preps` and `attach_meeting_transcript`. These total ~179 lines. `attach_meeting_transcript` is async and contains a TOCTOU (time-of-check-time-of-use) guard that must be preserved during extraction.

## Acceptance Criteria

1. `services/meetings.rs` is expanded with public methods for both handlers:
   - `refresh_meeting_preps()`
   - `attach_meeting_transcript()` — async, TOCTOU guard preserved
2. Both command handlers in `commands.rs` are thin wrappers (parse args → call service → return).
3. The TOCTOU guard in `attach_meeting_transcript` is preserved exactly — the check-then-act sequence must remain atomic to prevent duplicate transcript attachment.
4. IPC surface unchanged — both Tauri commands have identical signatures and return types.
5. `cargo test` passes. `cargo clippy -- -D warnings` passes.
6. ~179 lines removed from `commands.rs`.

## Dependencies

- Builds on the existing `services/meetings.rs` established in Phase 1 (I380).
- No dependency on I450–I452, I454.

## Rationale

The meeting service already exists from Phase 1 extraction. These 2 remaining handlers were left behind due to complexity (async transcript attachment with TOCTOU guard, prep refresh orchestration). Completing the extraction gives the meeting domain a single service boundary. The TOCTOU guard is the key risk — it prevents race conditions on transcript attachment and must not be weakened during the move.
