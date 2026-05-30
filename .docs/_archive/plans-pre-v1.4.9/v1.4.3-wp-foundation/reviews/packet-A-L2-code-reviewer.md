# Packet A — L2 code-reviewer (claude domain pass)

**Reviewer:** claude (code-reviewer domain, L2 diff review)
**Branch:** `dos-673-674-675-lifecycle-hardening` @ `3429022a` (4 commits since `a594cd4d`)
**L0 anchor:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md` V1.1.1
**Scope:** AC-adjacent (§7) + ADR + PR-introduced regression. Path-α residuals → maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`.

## Verdict

**APPROVE.** All 23 acceptance criteria are met or proven by fixture; the §4 substrate-reuse pattern is honored (no net-new primitives beyond the L0-sanctioned `SessionKeyLookup`, `KeychainBackend`, `KeychainCleanupTarget`, `SignedTransportFailureOutcome`, `explicit_sentinel_cleanup`); the §9 invariants have concrete grep/AST/runtime enforcement; CLAUDE.md "services own mutations" is preserved (every keychain side effect is in `services/`, every cleanup runs outside the writer closure). The one ambiguity worth raising is filed below as path-α (single Linear maintenance ticket), not a block.

## §5 implementation plan — commit-by-commit verification

### Commit `df8a668a` (DOS-673) — SessionKeyLookup + KeychainBackend trait + DOS-675 sentinel cleanup

DOS-675 is intentionally folded into this commit despite the subject line; the changes are co-located in `src-tauri/src/surface_runtime/mod.rs` and the commit message's scope is honest about the breadth (`L0 Packet A V1.1.1 §5.1, §5.1a, §6.1, §6.4`). DOS-675 wiring lives at `surface_runtime/mod.rs:329` (post-loop call replaced with `explicit_sentinel_cleanup`), `:415` (stop), `:472` (drop), `:949` (helper definition). **Not a defect** — the §5.4 helper is a 4-line wrapper around `remove_runtime_sentinel`; splitting it into a dedicated commit would have been theatrical. Worth flagging only because a reader looking at the commit list would not find a literal "DOS-675" commit.

### Commit `3429022a` (DOS-674) — KeychainCleanupTarget plumbing

All three `revoke_pairing_row` call sites are wired per §5.3:
1. `revoke_pairing` at `surface_pairing.rs:1316-1325` — collects target inside `with_transaction`, returns `(AuditEvent, KeychainCleanupTarget)`, command at `commands/surface_runtime.rs:78-83` runs cleanup after `db_write().await`.
2. `revoke_existing_pairing_for_site` at `:1970` — collects target before `revoke_pairing_row`, returns tuple; caller `complete_handshake` runs cleanup at `:633-636` AFTER `db.with_transaction(...)?` returns and BEFORE `persist_session_master_key` at `:643`. AC #13 satisfied.
3. `record_signed_transport_failure` at `:1442-1462` — collects + revokes inside `with_transaction`, returns `SignedTransportFailureOutcome { events, cleanup_target }`; runtime caller at `surface_runtime/mod.rs:1615-1620` runs cleanup after `db_write().await`. AC #14 satisfied.

`SignedSessionWriteAction::MarkSessionRevoked` payload extension (AC #15) lives at `surface_pairing.rs:1054-1058`; `SessionExpired` variant carries `surface_client_id` at `:1021-1024`; the in-scope plumbing at `validate_signed_session_readonly` (`:909-910`) populates it correctly. Dispatch returns `Option<KeychainCleanupTarget>` at `:1151-1170` and the runtime hop at `surface_runtime/mod.rs:1178-1196` consumes it outside the writer closure. Clean.

The in-tx SELECT (AC #11) lives at `collect_pairing_cleanup_target` (`:2125-2150`) and is called BEFORE `revoke_pairing_row` in all three sites — verified manually + by fixture `dos674_cleanup_target_collected_inside_transaction` at `:3548-3578`.

### Commit `0edda6ff` (chore: v178 table_exists guard)

Pre-existing build-tax fix per commit message; gates ALTER TABLE behind `SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?`. Out of L0 packet scope but mechanically necessary for the test suite to build. Self-contained, low blast radius. **APPROVE as a prereq.**

### Commit `ef6c3ce2` (fix: linear sync ServiceContext)

Pre-existing build-tax: a prior refactor renamed `crate::signals::bus::emit_signal_and_propagate` → `crate::services::signals::emit_and_propagate` and added a `ServiceContext` parameter; the linear sync caller path was never updated. Minimal threading of `state.live_service_context().with_actor("linear_sync")` through `upsert_issues → emit_issue_change_signals`. Self-contained. **APPROVE as a prereq.**

## §4 substrate-reuse audit

No net-new primitives beyond the four named in the L0 packet (`SessionKeyLookup`, `KeychainBackend` + `RealKeychain`/`MockKeychain`, `KeychainCleanupTarget`, `SignedTransportFailureOutcome` + `cleanup_session_keychain_entries` helper). `KeychainBackend` follows the `db/key_provider.rs` precedent (V1.1.1 corrected the gravatar misattribution; the code matches `key_provider.rs:205-307` shape — trait + production impl + thread_local override + `with_keychain_for_tests` helper). `KeychainCleanupTarget` is the `services/accounts.rs:3514-3548` "transaction returns artifacts, caller runs side effects outside" pattern, applied to keychain deletes. **APPROVE.**

## §9 CI invariants — fixture / grep gate coverage

| # | Invariant | Where enforced |
|---|---|---|
| 1 | `load_session_master_key` returns `SessionKeyLookup` | `tests/dos673_674_675_invariants.rs::dos673_load_session_master_key_returns_classified_lookup` — AST gate over function body |
| 2 | No `KeychainBackend` method inside writer closure | `tests/dos673_674_675_invariants.rs::dos674_no_direct_keychain_delete_inside_pairing_writer_closures` — grep that single `delete_session_master_key(` site lives in `cleanup_session_keychain_entries`; runtime fixture `dos674_cleanup_outside_transaction` proves 100 ms keychain delay does NOT delay the SQLite tx commit |
| 3 | Rehydration revocation gated on `NotFound` only | `tests/dos673_674_675_invariants.rs::dos673_rehydration_keychain_missing_revocation_is_notfound_only` — verifies `missing.push(row)` lives only inside the `NotFound` arm |
| 4 | `stop` and `Drop` call `explicit_sentinel_cleanup` before `abort` | `tests/dos673_674_675_invariants.rs::dos675_stop_and_drop_cleanup_before_abort` — string-order assertion over both function bodies |
| 5 | `Drop` does not call async | `tests/dos673_674_675_invariants.rs::dos675_drop_does_not_call_async_flush_or_db_writer` — asserts no `.await`, no `db_write`, no `flush_session_activity_on_shutdown`, no `stop_async` in drop body |

All five have concrete enforcement. The AST gates are intentionally fragile (string-match over source text) but adequate for the invariant; if surface_pairing or surface_runtime is restructured, the failure mode is "test fails, dev re-reads the invariant" which is exactly what we want.

## §7 AC ↔ fixture mapping spot-check

Confirmed end-to-end for the load-bearing ACs:
- AC #11 (in-tx collection BEFORE revoke) — fixture `dos674_cleanup_target_collected_inside_transaction` asserts `cleanup_target.session_ids` is non-empty after `revoke_pairing`, with comment that "collection after revoke with revoked_at IS NULL would be empty"
- AC #13 (re-pair deletes OLD before persisting NEW) — fixture `dos674_repair_deletes_old_session_keys_only` asserts first session key is `NotFound` and second is `Found` after two handshakes
- AC #17 (cleanup failure does not rollback) — fixture `dos674_keychain_delete_failure_does_not_rollback_db` asserts `revoked_reason` persists and audit emits `pairing.session.key_cleanup_failed`
- AC #20–23 (sentinel reach + idempotence + no async drop) — fixtures `dos675_sentinel_cleaned_on_{stop,drop}`, `dos675_repeated_stop_is_idempotent`, `dos675_drop_no_async_flush` (50 ms wall-clock budget assertion)

## Code-quality observations

**Audit emission consistency.** New cleanup events use `category: "security"` (`surface_pairing.rs:1506,1526`); the existing rehydration audit `pairing.session.key_missing` and the new `pairing.session.key_unavailable` use `category: "surface_pairing"`. Both naming conventions exist in the codebase; choosing `security` for cleanup events vs `surface_pairing` for rehydration diagnostics is defensible (cleanup is a security-relevant lifecycle write; rehydration audit is operational). **Not a blocker** — flag for the audit-event-kind normalization maintenance ticket that v1.4.3 explicitly defers.

**Doc-comment coverage on new public API.** `SessionKeyLookup`, `KeychainBackend`, `KeychainCleanupTarget`, `SignedTransportFailureOutcome`, and `cleanup_session_keychain_entries` are all `pub` without doc-comments. The existing primitives in the same files (`persist_session_master_key`, `revoke_pairing_row`) do carry doc-comments. Path-α — file as maintenance ticket; does not violate any §7 AC.

**Error-handling completeness.** `signed_transport_response` at `surface_runtime/mod.rs:1188` silently drops both `Err(_)` (writer-mutex acquisition failure) and `Ok(None)` (no cleanup needed). The previous code carried an explicit `#[allow(clippy::let_underscore_must_use, reason = "quarantine write best-effort")]` comment that documented the intent; the new code retains the semantics but loses the rationale. This is a wash — the new shape is structurally clearer (no `let_` underscore at all), but a code reader sees a `Result` matched only on `Ok(Some(_))` without an explicit `// best-effort: rejection response is the meaningful outcome` comment. Path-α (cosmetic / docs).

**`mark_pairing_expired` re-queries `pairing_epoch`.** The query at `:2251-2265` uses the same `surface_client_id + lifecycle_state = 'active' + expires_at <= now` filter as the subsequent UPDATE at `:2293-2300`. Both are inside the caller's `with_transaction`, so the race window is closed by SQLite's transaction isolation; the duplicate filter is repetitive but correct. Path-α candidate for refactor (collect inside a `SELECT ... RETURNING` if SQLite version allows, or hoist into a single helper); does not violate any §7 AC.

**`SignedSessionFailure::SessionExpired` plumbing in `validate_signed_session` (non-readonly).** At `:777-783`, the call site uses `let _ = mark_session_revoked(...)?` and discards the `Option<KeychainCleanupTarget>` return. This is the legacy non-readonly validate path — the comment on `apply_signed_session_write_action` makes clear that the readonly path is where the cleanup target gets carried out to the runtime. The non-readonly path is no longer the primary surface (per W4-F), but it still exists. **No cleanup runs on this path.** Per §11 ("What this packet explicitly does NOT own") the readonly path is the production surface; legacy `validate_signed_session` callers expire keys lazily. Path-α candidate — file to maintenance as "wire `validate_signed_session` legacy path through cleanup target for parity."

## Critical Rules check (CLAUDE.md)

- **All mutations go through `services/`.** Every keychain write/delete + every DB write originates in `services/surface_pairing.rs` or `services/surface_session_keychain.rs`. Commands at `commands/surface_runtime.rs:67-95` are a thin orchestrator: `db_write` → service call → consume returned cleanup target → call `cleanup_session_keychain_entries` (also in services) → emit audit. **No direct DB writes from command handlers.** PASS.
- **No customer-specific data.** Tests use `surface_test`, `session_test`, `surface_found`, `surface_missing`, `surface_unavailable` etc. PASS.
- **No PII in commit messages.** All four commits use neutral wording. PASS.
- **Definition of Done.** ACs 1-23 satisfied by code + fixture; the `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` gate is implementation-team responsibility for L1, not L2 — but the new code follows existing rustfmt/clippy conventions and the AST-shape fixtures are constructed to fail loudly if invariants regress.

## Path-α findings (file to maintenance project, NOT a block)

- → Linear maintenance: Doc-comment coverage for new surface session lifecycle public API (`SessionKeyLookup`, `KeychainBackend`, `KeychainCleanupTarget`, `SignedTransportFailureOutcome`, `cleanup_session_keychain_entries`)
- → Linear maintenance: Restore best-effort rationale comment at `surface_runtime/mod.rs:1188` (`signed_transport_response` cleanup target handling)
- → Linear maintenance: Hoist `mark_pairing_expired` duplicate `surface_client_id + active + expires_at <= now` filter into single helper or `SELECT ... RETURNING`
- → Linear maintenance: Wire `validate_signed_session` (legacy non-readonly path) through `KeychainCleanupTarget` for cleanup parity with `validate_signed_session_readonly`
- → Linear maintenance: Normalize audit `category` field between `pairing.session.key_*` events (currently mixed `surface_pairing` and `security`)

None of the above is a §7 AC violation, ADR violation, or PR-introduced regression. The substrate is sound; the maintenance items are healthy follow-on hygiene.

## L2 verdict: APPROVE
