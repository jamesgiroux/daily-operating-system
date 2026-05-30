CONDITIONAL APPROVE

V1.1 resolves the Cycle 1 implementation blockers around the missing
suspicious-replay cleanup path, the `JoinHandle` ownership conflict, and the
over-scoped Tauri shutdown hook. The remaining implementation shape is feasible,
but the packet still overstates fixture coverage: 23 fixtures exist, yet they do
not map 1:1 to the 23 acceptance criteria.

## 1. Section 5.4 Simplified Stop/Drop Design — PASS
The simplified helper shape is implementable as described. Current
`RunningEndpoint` has no listener `JoinHandle` field, only `shutdown` and
`abort` at `src-tauri/src/surface_runtime/mod.rs:182-188`, while the listener
`JoinHandle` remains returned from `start_listener` at
`src-tauri/src/surface_runtime/mod.rs:239-243` and awaited by
`run_until_stopped` at `src-tauri/src/surface_runtime/mod.rs:385-399`.

The current listener task already removes the sentinel after `run_listener`
returns at `src-tauri/src/surface_runtime/mod.rs:327-331`. Adding
`explicit_sentinel_cleanup()` before the existing abort in `stop` at
`src-tauri/src/surface_runtime/mod.rs:404-420` and before the existing abort in
`Drop` at `src-tauri/src/surface_runtime/mod.rs:468-476` does not require new
endpoint state.

V1.1 also correctly avoids new Tauri run-event wiring: the current tray quit
path is only `app.exit(0)` at `src-tauri/src/lib.rs:584-585`, and the app still
uses plain `.run(...)` without a callback at `src-tauri/src/lib.rs:1164`. That
leaves OS-quit coverage deferred exactly as §5.4 says.

## 2. Suspicious-Replay Cleanup Through Runtime Caller — PASS
The `record_signed_transport_failure` runtime bridge can carry cleanup targets
out of the writer closure. The current call is inside `app_state.db_write(...)`
at `src-tauri/src/surface_runtime/mod.rs:1526-1536`, and `db_write` returns the
closure result after the writer call resolves at `src-tauri/src/state.rs:1495-1524`.

The service already wraps the suspicious-replay writes in
`db.with_transaction(...)` at `src-tauri/src/services/surface_pairing.rs:1297-1404`,
returning `events` to the runtime. Changing that return value to
`(events, cleanup_target)` is mechanically feasible.

The required cleanup slot exists after `.await` returns at
`src-tauri/src/surface_runtime/mod.rs:1536-1550` and before in-memory eviction in
the event loop at `src-tauri/src/surface_runtime/mod.rs:1554-1565`. The
implementation should run keychain deletes in that gap, before iterating events
that can remove signed-transport sessions.

## 3. `MarkSessionRevoked` Payload Extension — PASS WITH NOTE
Extending `SignedSessionWriteAction::MarkSessionRevoked` is feasible. The
current action carries only `session_id` and `reason` at
`src-tauri/src/services/surface_pairing.rs:1021-1025`, and dispatch currently
calls `mark_session_revoked` without a surface client at
`src-tauri/src/services/surface_pairing.rs:1105-1114`.

The packet's cited `surface_pairing.rs:754-755` does show `row` loaded in the
older mutating validation path, and that row has `surface_client_id` in use later
at `src-tauri/src/services/surface_pairing.rs:790`. The active runtime read-path
uses `validate_signed_session_readonly`, where the same row is loaded at
`src-tauri/src/services/surface_pairing.rs:876-880`.

Implementation note: for the runtime path, `SignedSessionFailure::SessionExpired`
must first carry `row.surface_client_id` from the readonly check at
`src-tauri/src/services/surface_pairing.rs:888-891`, then `write_action()` can
populate the extended action at `src-tauri/src/services/surface_pairing.rs:1040-1047`.
Without that intermediate enum change, the dispatch site still lacks the surface
client id.

## 4. In-Transaction Cleanup Target SELECTs — PASS
The required in-transaction session-id SELECT is feasible at all three
`revoke_pairing_row` call sites. Explicit revoke enters `db.with_transaction(...)`
at `src-tauri/src/services/surface_pairing.rs:1269-1272`; the transaction already
has `row.surface_client_id` and `row.pairing_epoch` from `RevocationTarget`
loaded at `src-tauri/src/services/surface_pairing.rs:1253-1263`.

The re-pair flow wraps `revoke_existing_pairing_for_site` in the handshake
transaction at `src-tauri/src/services/surface_pairing.rs:528-617`, and that
helper has the previous pairing's `surface_client_id` and `pairing_epoch` at
`src-tauri/src/services/surface_pairing.rs:1847-1855`. Suspicious replay opens
its own transaction at `src-tauri/src/services/surface_pairing.rs:1297-1404` and
has the target row before calling `revoke_pairing_row` at
`src-tauri/src/services/surface_pairing.rs:1381-1384`.

`ActionDb::with_transaction` passes the same `ActionDb` view into the closure
and commits only after the closure returns at `src-tauri/src/db/core.rs:95-115`.
That supports collecting session ids before `revoke_pairing_row` mutates rows at
`src-tauri/src/services/surface_pairing.rs:1993-2019`.

## 5. KeychainBackend Test Seam — PASS WITH NOTE

The trait-based seam is workable, but the packet's exact precedent claim is only
partially grounded. `src-tauri/src/gravatar/keychain.rs` uses the same direct
`security` CLI helper shape at `src-tauri/src/gravatar/keychain.rs:8-18`, but it
does not define a `KeychainBackend` trait or a `thread_local!` test override.

The stronger in-repo precedent is `src-tauri/src/db/key_provider.rs`, which
defines a `KeychainBackend` trait at `src-tauri/src/db/key_provider.rs:205-213`,
implements the production `SecurityCliKeychain` at
`src-tauri/src/db/key_provider.rs:215-251`, and injects a fake backend in tests
via `with_keychain_for_tests` at `src-tauri/src/db/key_provider.rs:302-307` plus
`FakeKeychain` at `src-tauri/src/db/key_provider.rs:1052-1079`.

A `#[cfg(test)] thread_local!` override remains implementable for
`surface_session_keychain` because the public API is free functions at
`src-tauri/src/services/surface_session_keychain.rs:79-83`,
`src-tauri/src/services/surface_session_keychain.rs:107-110`, and
`src-tauri/src/services/surface_session_keychain.rs:139`. The specific "matches
gravatar/keychain.rs" claim is unverified.

## 6. Fixture-to-Acceptance Mapping — FAIL

The fixture count now matches the acceptance-criteria count, but the mapping is
not 1:1. Section 7 lists AC #1-23 at
`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:394-426`,
while Section 8 lists fixtures #1-23 at
`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:428-454`.

Several criteria remain unmapped or only partially mapped: AC #1 is covered by
CI invariant #1 rather than a fixture; AC #7 has no fixture proving read paths do
not use the enum; AC #9 has no fixture/static assertion for the
`KeychainCleanupTarget` public shape; AC #17 lacks success-audit and logging
coverage; AC #20 does not prove the helper remains called from listener
post-loop; and AC #22 covers repeated `stop` but not repeated `Drop`.

This is a test-plan accuracy issue, not a core implementation blocker. The
packet should replace the count-equality claim with an explicit
AC-to-fixture/invariant mapping table and add or reword fixtures where the
current rows only prove part of the criterion.

## Findings

MEDIUM — Fixture coverage is still overstated as 1:1.
Pointer: `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:585-587`
says all 23 acceptance criteria are fixture-mapped, but the criteria at
`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:394-426`
and fixtures at
`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:428-454`
do not line up one-for-one. Recommended fix: add an explicit AC →
fixture/invariant table and close the unmapped criteria listed in validation #6.

LOW — The keychain test-seam precedent points at the wrong file.
Pointer: `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:152-176`
says the trait/thread-local shape matches `src-tauri/src/services/gravatar/keychain.rs`,
but the actual Gravatar file is `src-tauri/src/gravatar/keychain.rs` and it only
has direct CLI helpers at `src-tauri/src/gravatar/keychain.rs:8-18`. Recommended
fix: cite `src-tauri/src/db/key_provider.rs:205-213` and
`src-tauri/src/db/key_provider.rs:302-307` for the trait/test-injection
precedent, or narrow the Gravatar citation to the shared `security` CLI helper
pattern.

Final recommendation: CONDITIONAL APPROVE after V1.2 fixes the fixture mapping
claim and corrects the keychain precedent citation; no remaining Cycle 1 medium
implementation blocker is still valid.
