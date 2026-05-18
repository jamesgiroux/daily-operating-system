# L2 CSO security review — Packet A (lifecycle hardening)

**Reviewer:** CSO (security domain)
**Scope:** L2 pre-merge against L0 Packet A V1.1.1 (cycle-2 CSO APPROVE).
**Branch:** `dos-673-674-675-lifecycle-hardening` @ HEAD, vs `a594cd4d`.
**Diff size:** 11 files, +1606/-178; 4 substantive commits (1 chore for migration replay guard, 1 unrelated linear/signals fix).
**Bounded to:** AC adjacency. Theoretical hardening → `→ Linear maintenance` (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`), NOT a BLOCK.

## Verdict: **APPROVE**

The CORE that cycle-2 APPROVE attached to is implemented faithfully and inside the local-to-local threat envelope. No remote-shape defenses reintroduced. The 6 V1.1-deferred items remain deferred. Two MEDIUM-class observations and a few path-α maintenance follow-ups noted; none block.

## Validation against the six L2-scope questions

**1. Three-variant `SessionKeyLookup` — PASS.**
`surface_session_keychain.rs:47-52` defines exactly `Found / NotFound / Unavailable { reason }`. Classifier at `:108-132` routes only the three documented `is_keychain_item_not_found` signatures ("could not be found", "item not found", "-25300") to `NotFound`; every other non-success path goes `Unavailable`. Corrupt payload and length mismatch → `Unavailable` (not `NotFound`). Rehydration at `surface_runtime/mod.rs:685-690` honors the split — `NotFound` is the only arm that queues `keychain_entry_missing` revocation; `Unavailable` leaves the DB row active and emits `pairing.session.key_unavailable` audit with `decision: left_active`. CI invariant `dos673_rehydration_keychain_missing_revocation_is_notfound_only` (services/tests/dos673_674_675_invariants.rs:90) enforces this structurally. This closes the false-revocation-on-transient-keychain-failure attack surface that motivated DOS-673.

**2. Cleanup-outside-tx + rehydration `revoked_at IS NULL` filter — PASS.**
Initial SELECT at `surface_runtime/mod.rs:639` filters `s.revoked_at IS NULL` (load-bearing per cycle-2 CSO APPROVE on the orphan-keychain deferral); the post-rehydrate UPDATE at `:738` filters `revoked_at IS NULL` again to avoid double-write. CI invariant `dos674_no_direct_keychain_delete_inside_pairing_writer_closures` (services/tests/dos673_674_675_invariants.rs:57) provides a structural fence: `delete_session_master_key(` appears exactly once in `surface_pairing.rs` (only inside `cleanup_session_keychain_entries`), and the lexical-line guard rejects any caller that places a keychain call on the same source line as `db_write` or `with_transaction`. Sufficient enforcement for the cleanup-outside-tx invariant.

**3. `KeychainCleanupTarget` plumbing — PASS.**
Three revoke paths (`revoke_pairing` :1320, `revoke_existing_pairing_for_site` :1969-1970, `record_signed_transport_failure` :1442) all collect via `collect_pairing_cleanup_target` BEFORE `revoke_pairing_row`, inside the surrounding `db.with_transaction` boundary. `mark_session_revoked` (:2210) and `mark_pairing_expired` (:2246-2308) both return `Option<KeychainCleanupTarget>` and run cleanup-collection in-tx. All four external call sites (`commands/surface_runtime.rs:81`, `surface_runtime/mod.rs:1191/1616/1662`) invoke `cleanup_session_keychain_entries` AFTER `.await` on the `db_write` future returns. AC #18 (cleanup outside transaction) satisfied.

**4. `MarkSessionRevoked` payload extension — PASS.**
`SignedSessionFailure::SessionExpired { surface_client_id, session_id }` (:1021-1024) carries both identifiers from `validate_signed_session_readonly` (:909-912; sourced from `row.surface_client_id`, not the caller-supplied input field). `write_action()` propagates both into `SignedSessionWriteAction::MarkSessionRevoked { surface_client_id, session_id, reason }` (:1088-1091). Dispatch at `apply_signed_session_write_action` (:1161) passes them into `mark_session_revoked`, which constructs the `KeychainCleanupTarget` with both fields populated (:2240-2243). End-to-end plumbing intact.

**5. `explicit_sentinel_cleanup` ordering — PASS.**
`stop` at `surface_runtime/mod.rs:415` calls `explicit_sentinel_cleanup()` BEFORE `endpoint.abort.abort()` at `:419`. `Drop::drop` at `:472` calls cleanup BEFORE `abort()` at `:476`. CI invariant `dos675_stop_and_drop_cleanup_before_abort` (services/tests/dos673_674_675_invariants.rs:113) enforces lexical ordering by source-offset comparison in both functions. `dos675_drop_does_not_call_async_flush_or_db_writer` (:133) enforces no `.await`, `db_write`, `flush_session_activity_on_shutdown`, or `stop_async` inside `Drop`. No race window where the sentinel persists pointing at a dead port.

**6. V1.1 deferrals not reintroduced — PASS.**
Diff scan confirms none of the six deferred items appear: no `outbox` table or sweep, no 4-variant `Unavailable` taxonomy split, no `env_clear()` / `HOME`/`USER` allowlist on `security` CLI invocation (`run_security_cmd` at `:65` still uses bare `Command::new("security")`), no `session_id` hashing in the new `pairing.session.key_cleaned` / `key_cleanup_failed` / `key_missing` / `key_unavailable` audit events, no coalesce window on rehydration audits, no `stop_async` graceful-drain. Posture matches V1.1.1.

## Findings

**MEDIUM — M-L2-CSO-1: Cleanup-target collected at request time, before write-action escalation completes.**
For the dispatch path at `surface_runtime/mod.rs:1167-1196`, the collection of session_ids for the cleanup target happens inside `mark_session_revoked`, which runs inside `db_write` AFTER the readonly validation already returned the `(surface_client_id, session_id)` pair. Between the readonly `db_read` and the escalated `db_write`, a concurrent `revoke_pairing` or `record_signed_transport_failure` could have already marked the session revoked; `mark_session_revoked`'s pre-UPDATE SELECT correctly handles this by returning `None` cleanup_target when no `revoked_at IS NULL` row exists. Race outcome is correct (no double-keychain-delete, no DB inconsistency), but the audit attribution for which path "won" the revoke can be ambiguous in tight-window concurrent flows.
**→ Linear maintenance: Surface session revoke audit attribution under concurrent lifecycle transitions** — purely an audit-clarity question, no security impact on local-to-local; aligns with the deferred external-side-effect contract item.

**MEDIUM — M-L2-CSO-2: Keychain payload UTF-8 transport via `-w` argv leaks key bytes to process argv visibility.**
`RealKeychain::persist` (`surface_session_keychain.rs:142-159`) base64-encodes the 32-byte master key and passes it as the value of the `-w` argv flag to `security add-generic-password`. On macOS, `/usr/sbin/security`'s argv is visible to any same-UID process via `ps -axww`. For local-to-local single-user, same-UID adversary already wins (can `SecItemCopyMatching` the entry directly), so the threat model still holds. Worth noting because the W4-F threat model anchor (v1.4.2 §"Threat model: local-to-local") implicitly assumes the trust boundary is the UID, not the process. If the federation threat model ever lifts to multi-UID isolation, this becomes a real exposure window.
**→ Linear maintenance: Use security CLI stdin keychain-payload transport instead of argv** — folds with the deferred `env_clear()` hardening item; both are federation-shape and inert at single-UID local.

**LOW — L-L2-CSO-3: Ephemeral issue refs in code comments.**
Two refs added in this diff violate the "no ephemeral issue refs in code comments" rule:
- `surface_pairing.rs:648` — `log::warn!("surface session key keychain persist failed (DOS-646): {err}");` (pre-existing log statement reformatted in this packet, retains the DOS-646 ref).
- `surface_pairing.rs:2251` — `// Per L0 Packet A V1.1.1 AC #16 + cycle-2 codex challenge HIGH:`
Neither has security impact; out-of-domain for CSO but flagging since CI lints this.
**→ Linear maintenance: Strip ephemeral issue refs (DOS-646, L0 V1.1.1, cycle-2) from surface_pairing.rs comments** — code-reviewer's lane; not a CSO BLOCK.

**LOW — L-L2-CSO-4: Plaintext session_id in new audit events extends existing inconsistency.**
The four new audit events (`pairing.session.key_cleaned`, `key_cleanup_failed`, `key_missing`, `key_unavailable`) emit `session_id` as plaintext in `detail`. This matches V1.1's explicit deferral of "Audit `event_kind` hashing of session_id" to maintenance; the new events extend the same pattern. No new exposure beyond what V1.1 accepted.
**→ Already covered by the existing deferred maintenance ticket** — Normalize audit event_kind session_id hashing across surface_runtime + surface_pairing.

## Acceptance against L0 Packet A V1.1.1

- AC #1-#9 (`SessionKeyLookup` classification + KeychainBackend trait + cleanup struct shape): satisfied (§9 invariant #1 grep + fixtures #1-#9).
- AC #10-#16 (three call sites wired, in-tx collection, three lifecycle paths, MarkSessionRevoked extension, mark_pairing_expired in-tx): satisfied (commit 97a02070 added the missing `with_transaction` wrap on `mark_pairing_expired`).
- AC #17-#19 (cleanup audit + outside-tx + idempotent): satisfied (§9 invariant #2 + dos674 fence test).
- AC #20-#23 (sentinel before abort, idempotent, Drop no-async): satisfied (§9 invariant #5 + dos675 fence tests).

## Security posture summary

Packet A removes three confirmed local-instability defects without adding any new attack surface, without breaking the local-to-local trust boundary, and without reintroducing the remote-shape defenses V1.1 explicitly trimmed. CI fences (`dos673_…`, `dos674_…`, `dos675_…` source-text invariants) lock the structural shape so cycle-N+1 regressions fail at compile/test time, not in production. No CSO BLOCK; merge.
