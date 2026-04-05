# I381 — db/mod.rs Domain Migration — Move Queries into Domain Modules per SERVICE-CONTRACTS.md

**Status:** Open (0.13.2)
**Priority:** P2
**Version:** 0.13.2
**Area:** Code Quality / Refactor

## Summary

`db/mod.rs` is approximately 9,700 lines containing SQL queries for every entity type intermixed. SERVICE-CONTRACTS.md defines the target: domain-specific modules (`db/actions.rs`, `db/accounts.rs`, `db/meetings.rs`, `db/people.rs`) own their respective queries, and `db/mod.rs` retains only the connection struct, transaction helper, shared constants, and re-exports. Target: ≤600 lines in `db/mod.rs` (down from ~9,700).

This is a purely structural refactor — no logic changes, no new SQL, no behavior changes.

## Acceptance Criteria

From the v0.13.2 brief, verified in the codebase:

1. `db/actions.rs` owns all action queries. `db/mod.rs` contains no action-specific SQL (`grep "actions\|DbAction" src-tauri/src/db/mod.rs` returns only re-exports and column constants, not SQL strings).
2. `db/accounts.rs` owns all account queries. Same test.
3. `db/meetings.rs` owns meeting, attendee, and entity link queries. Same test.
4. `db/people.rs` owns all people CRUD and merge queries. Same test.
5. `db/mod.rs` retains only: `ActionDb` struct definition + `open()` + `conn_ref()`, `with_transaction()`, shared column `const` lists, and re-exports. Measure after: `wc -l src-tauri/src/db/mod.rs`. Target: ≤600 lines (down from ~9,700).
6. `cargo test` passes. `cargo clippy -- -D warnings` passes. No behavior changes — this is purely structural.

## Dependencies

- Independent — purely structural, no logic changes, no dependencies on other v0.13.2 issues.
- Can be worked in parallel with I376, I377, I378, I379.

## Notes / Rationale

A 9,700-line database module is a maintenance hazard: every database change requires understanding the full file to avoid breaking something, and domain expertise is scattered rather than co-located. The domain module split follows standard Rust module organization and matches the service layer that I380 establishes. Together, I380 + I381 bring the codebase from "god files" to a maintainable layered architecture.
