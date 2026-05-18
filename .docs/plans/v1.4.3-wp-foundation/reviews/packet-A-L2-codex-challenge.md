Verdict: APPROVE

Packet A mostly lands the requested lifecycle hardening: `SessionKeyLookup` separates
missing entries from unavailable keychain conditions, all three `revoke_pairing_row`
call sites collect cleanup targets before revocation inside their write transaction,
sentinel cleanup runs before abort in both forced stop paths, and
`MarkSessionRevoked` now carries `surface_client_id` through the readonly failure
path into post-commit cleanup. Cycle 2 re-verify confirms the pairing-expiry
cleanup now uses a SQLite transaction for the required collect-then-expire
snapshot. Cycle 1 targeted verification: `cargo test dos67` passed, including
27 relevant unit tests; warnings were pre-existing unused imports in unrelated
test targets.

## AC-adjacent findings

### CRITICAL

None.

### HIGH

1. AC #16 says `mark_pairing_expired` must collect affected session ids via the
   in-transaction snapshot before marking the pairing expired, but the final
   implementation runs a direct `SELECT` and a later direct `UPDATE` without a
   `db.with_transaction` boundary. Evidence: the cleanup target is selected with
   `db.conn_ref().query_row` / `prepare` at
   `src-tauri/src/services/surface_pairing.rs:2251` and
   `src-tauri/src/services/surface_pairing.rs:2267`, then the expiry write runs
   separately at `src-tauri/src/services/surface_pairing.rs:2292`. The runtime
   signed-session path reaches this directly through
   `apply_signed_session_write_action` at
   `src-tauri/src/services/surface_pairing.rs:1162`, so the AC's SQLite
   transaction contract is not met even though cleanup itself still runs after
   `db_write().await`.

### MEDIUM

None.

## Linear maintenance

- Legacy `validate_signed_session` ignores returned cleanup targets from
  `mark_session_revoked` / `mark_pairing_expired`; maintenance because no
  production call site was found in this diff.

## Per-question CHALLENGE answers

1. SessionKeyLookup classification: pass from diff evidence, with one real-world
   caveat. `SessionKeyLookup` has exactly `Found`, `NotFound`, and
   `Unavailable { reason }` at
   `src-tauri/src/services/surface_session_keychain.rs:47`. The not-found matcher
   mirrors `key_provider.rs` by checking `could not be found`, `item not found`,
   and `-25300` at
   `src-tauri/src/services/surface_session_keychain.rs:95`. Non-success output
   only becomes `NotFound` through that matcher; all other non-success output
   becomes `Unavailable` at
   `src-tauri/src/services/surface_session_keychain.rs:108`. Decode failures and
   length mismatch also become `Unavailable` at
   `src-tauri/src/services/surface_session_keychain.rs:117` and
   `src-tauri/src/services/surface_session_keychain.rs:126`. Diff alone cannot
   prove every macOS `security` stderr variant, but it implements the packet's
   named matcher.

2. KeychainCleanupTarget collection across the three revoke call sites: pass.
   Explicit revoke collects inside `db.with_transaction` before
   `revoke_pairing_row` at `src-tauri/src/services/surface_pairing.rs:1319`.
   Re-pair calls `revoke_existing_pairing_for_site` inside the handshake
   transaction at `src-tauri/src/services/surface_pairing.rs:539`, then collects
   before revoke at `src-tauri/src/services/surface_pairing.rs:1970`.
   Suspicious replay runs inside `db.with_transaction` at
   `src-tauri/src/services/surface_pairing.rs:1355`, then collects before
   `revoke_pairing_row` at `src-tauri/src/services/surface_pairing.rs:1442`.
   The shared snapshot SQL includes `surface_client_id`, `pairing_epoch`, and
   `revoked_at IS NULL` at `src-tauri/src/services/surface_pairing.rs:2125`.

3. explicit_sentinel_cleanup ordering: pass. The listener post-loop calls the
   helper at `src-tauri/src/surface_runtime/mod.rs:331`. `stop` calls it before
   `shutdown.send(true)` and `abort()` at
   `src-tauri/src/surface_runtime/mod.rs:415`. `Drop::drop` calls it before
   `shutdown.send(true)` and `abort()` at
   `src-tauri/src/surface_runtime/mod.rs:472`. The helper itself delegates to
   `remove_runtime_sentinel` at `src-tauri/src/surface_runtime/mod.rs:948`.

4. SignedSessionWriteAction MarkSessionRevoked payload extension: pass.
   `validate_signed_session_readonly` attaches `row.surface_client_id` to
   `SignedSessionFailure::SessionExpired` at
   `src-tauri/src/services/surface_pairing.rs:908`. The failure enum carries
   both fields at `src-tauri/src/services/surface_pairing.rs:1021`, and
   `write_action()` copies both into `SignedSessionWriteAction::MarkSessionRevoked`
   at `src-tauri/src/services/surface_pairing.rs:1085`. The action enum stores
   `surface_client_id` at `src-tauri/src/services/surface_pairing.rs:1053`, and
   `apply_signed_session_write_action` passes it into `mark_session_revoked` at
   `src-tauri/src/services/surface_pairing.rs:1157`. Runtime carries the returned
   cleanup target past `db_write().await` and performs keychain cleanup at
   `src-tauri/src/surface_runtime/mod.rs:1178`.

5. Cross-commit regressions: no blocker found. Commit 1's Linear signal context
   wiring remains in final state at `src-tauri/src/linear/sync.rs:12` and
   `src-tauri/src/services/linear_issue_signals.rs:129`. Commit 2's v178
   `table_exists` guard remains at
   `src-tauri/src/migrations/v178_dos_285_linear_issue_state.rs:16`. Commit 3's
   keychain enum/backend seam remains at
   `src-tauri/src/services/surface_session_keychain.rs:47` and
   `src-tauri/src/services/surface_session_keychain.rs:54`. Commit 4 adds the
   lifecycle consumers rather than regressing those earlier changes.

## Cycle 2 re-verify

Verdict: APPROVE for AC-adjacent L2 scope.

1. AC #16 re-check: PASS. `git show 97a02070 -- src-tauri/src/services/surface_pairing.rs`
   shows `mark_pairing_expired` replaced the prior direct `db.conn_ref()`
   sequence with one transaction; in the final branch the transaction boundary
   starts at `src-tauri/src/services/surface_pairing.rs:2256` and returns through
   `src-tauri/src/services/surface_pairing.rs:2310`.

2. The pairing_epoch lookup is inside that transaction: `tx.conn_ref()` begins
   the lookup at `src-tauri/src/services/surface_pairing.rs:2257`, the
   `SELECT pairing_epoch` body is at
   `src-tauri/src/services/surface_pairing.rs:2260`, and the optional result is
   completed before the closure continues at
   `src-tauri/src/services/surface_pairing.rs:2271`.

3. The session_ids collection is inside the same transaction: the statement is
   prepared from the tx connection at
   `src-tauri/src/services/surface_pairing.rs:2273`, the `SELECT session_id`
   body is at `src-tauri/src/services/surface_pairing.rs:2276`, and the
   collected `Vec` is completed at
   `src-tauri/src/services/surface_pairing.rs:2290`.

4. The lifecycle UPDATE is inside the same transaction: `tx.conn_ref().execute`
   begins at `src-tauri/src/services/surface_pairing.rs:2298`, the
   `UPDATE surface_client_pairings` statement is at
   `src-tauri/src/services/surface_pairing.rs:2300`, and the execute error path
   completes before the closure returns at
   `src-tauri/src/services/surface_pairing.rs:2307`.

5. Commit 8639c82a did not weaken the AC #16 fix. Its final effect in the
   touched function is the source comment at
   `src-tauri/src/services/surface_pairing.rs:2251` through
   `src-tauri/src/services/surface_pairing.rs:2255`; the tx boundary and all
   three DB operations remain at
   `src-tauri/src/services/surface_pairing.rs:2256` through
   `src-tauri/src/services/surface_pairing.rs:2310`.

6. No other AC-adjacent regression was found in commits 97a02070 and 8639c82a.
   The inspected combined diff is limited to the `mark_pairing_expired` hunk
   whose final executable code is
   `src-tauri/src/services/surface_pairing.rs:2256` through
   `src-tauri/src/services/surface_pairing.rs:2310`; no other file:line claim is
   available because the "no other file changed" part is a git diff file-list
   fact, not a source-line fact.

7. The cycle-1 path-alpha maintenance finding still applies. The legacy
   `validate_signed_session` path still discards the cleanup target from
   `mark_session_revoked` via `let _ =` at
   `src-tauri/src/services/surface_pairing.rs:777`, and still discards the
   cleanup target from `mark_pairing_expired` via `let _ =` at
   `src-tauri/src/services/surface_pairing.rs:810`.

8. Commit 8639c82a did not accidentally address that legacy path-alpha finding:
   the legacy validation function remains at
   `src-tauri/src/services/surface_pairing.rs:758` through
   `src-tauri/src/services/surface_pairing.rs:825`, while 8639c82a's final
   source-comment effect is confined to
   `src-tauri/src/services/surface_pairing.rs:2251` through
   `src-tauri/src/services/surface_pairing.rs:2255`.

9. Per L2 scope, the path-alpha cleanup-target discard remains maintenance-only
   and should be filed to Linear maintenance project
   `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`; it is not an AC-adjacent BLOCK
   because the runtime write-action path still returns the cleanup target from
   `apply_signed_session_write_action` at
   `src-tauri/src/services/surface_pairing.rs:1150` through
   `src-tauri/src/services/surface_pairing.rs:1164`.

Tests not run; this was a targeted diff/source re-verify only.
