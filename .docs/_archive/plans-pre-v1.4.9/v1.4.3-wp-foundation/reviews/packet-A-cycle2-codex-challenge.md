# Cycle-2 Adversarial L0 — Packet A V1.1

## Verdict

APPROVE

Observed evidence: V1.1 folds the local-shipping defects called out in cycle 1 and classifies the remaining concerns under the local-to-local threat model stated in §1 lines 20-26 and the Cycle 2 rule in §14 lines 576-581.

No deferred item is wrong-for-local under the §14 burden. The cycle-1 CRITICAL orphan-keychain issue remains real as a general lifecycle/reconciliation problem, but V1.1 correctly narrows v1.4.3 to local same-UID harm and defers federation-style durable reconciliation.

Hypothesis: if DailyOS later expands surface sessions beyond single-user local runtime, the deferred items should be treated as design requirements rather than optional hardening. That is already the v1.x-federation maintenance shape in §2 lines 59-67 and §11 lines 513-537.

## Folded items validation

- PASS - Suspicious-replay revoke path added to cleanup wiring.
  Evidence: §2 line 47 claims three `revoke_pairing_row` call sites, and §5.3 lines 224-232 enumerates explicit revoke, re-pair, and suspicious-replay threshold. Source grep also observes those three call sites at `surface_pairing.rs:1270`, `:1383`, and `:1855`.

- PASS - Cleanup target collection moved inside the same transaction as revoke.
  Evidence: §2 line 48 claims the in-transaction snapshot requirement; §5.3 lines 200-222 requires collection inside `with_transaction` before `revoke_pairing_row`; AC #11 repeats the exact predicate at §7 lines 409-411.

- PASS - `SignedSessionWriteAction::MarkSessionRevoked` carries `surface_client_id`.
  Evidence: §2 line 49 claims the payload extension; §5.3 lines 246-260 specifies `surface_client_id`, `session_id`, and `reason`; AC #15 at §7 line 415 makes it acceptance-tested.

- PASS - `stop_async` graceful-drain design replaced with synchronous sentinel cleanup.
  Evidence: §2 line 50 says `stop_async` is dropped; §5.4 lines 273-301 specifies `explicit_sentinel_cleanup()` called before abort from `stop` and `Drop`; §6.3 lines 347-356 accepts graceful flush as post-loop-only.

- PASS - Tauri shutdown-hook scope is trimmed.
  Evidence: §2 line 51 says no new `RunEvent::ExitRequested`; §5.4 lines 289-318 relies on listener post-loop, `stop`, `Drop`, and tray quit via existing `app.exit(0)`, with OS kill accepted as best-effort.

- PASS - `db_writer_observer` primitive removed and replaced with enforceable fixture/gates.
  Evidence: prior cycle-1 HIGH at review lines 59-70 said the observer did not exist; §9 lines 466-470 removes it and points to fixture #16; §8 lines 446-450 covers slow keychain delete outside the writer and the no-keychain-IO gate.

- PASS - `KeychainBackend` test seam specified.
  Evidence: §2 line 53 claims a trait seam; §5.1a lines 152-176 defines `KeychainBackend`, `RealKeychain`, and `MockKeychain`; §8 line 440 adds a fixture that production uses `RealKeychain` and tests override with `MockKeychain`.

- PASS - Transaction-returned-artifacts reuse anchor added.
  Evidence: §2 line 54 claims the accounts precedent; §4 lines 112-126 documents the existing transaction-return-value pattern and maps `KeychainCleanupTarget` to it.

- PASS - Line-number resync and primary anchor correction are addressed.
  Evidence: §4 lines 92-110 uses `persist_session_master_key` at `:79` and `remove_runtime_sentinel` at `:887`; §1 line 16 now cites the v1.4.2 project threat-model anchor on dev.

- PASS - Intelligence Loop exemption is explicit and bounded.
  Evidence: §1 lines 28-32 states no claim/table/surface/provenance/signal/runtime-context/feedback change; this matches the packet scope in §1 lines 20-26.

- PASS - Missing fixture coverage expanded.
  Evidence: §2 line 58 claims fixtures #15-23 were added; §8 lines 446-454 adds fixtures for in-transaction collection, cleanup outside tx, delete failure, idempotence, no writer closure IO, sentinel stop/drop, repeated stop, and no async flush.

## Deferred items validation

- Orphan keychain entries - CORRECT.
  Observed evidence: cycle-1 review lines 3-14 correctly found revoked rows with surviving keychain entries are not reconciled because rehydration only reads active rows. V1.1 acknowledges this at §2 line 60 and §11 lines 516-519.
  Local reasoning: after revoke-then-crash, the DB row is revoked and the restarted runtime memory is empty; §6.4 lines 358-362 preserves no read-path keychain probing, so orphan bytes are not reactivated into a user-visible session. Same-UID residual secret material is a hygiene concern, but under §1 lines 24-26 it is not a different-UID or remote exploit in v1.4.3.
  Hypothesis: a same-UID malicious process able to read the orphan keychain item is already inside the explicitly out-of-scope same-user boundary. Federation/shared-keychain deployments change that assumption, so the outbox/backlog deferral is correct.

- `Unavailable` taxonomy split + N=3 escalation - CORRECT.
  Observed evidence: cycle-1 review lines 16-30 identified the permanent-vs-transient ambiguity. V1.1 keeps a 3-variant enum at §5.1 lines 145-150, leaves rows active on `Unavailable` at §5.2 lines 180-187, and defers finer taxonomy at §2 line 61 and §11 lines 520-523.
  Local reasoning: log-and-leave-active is not worse for the local shipping bug because it prevents false revocation on transient keychain failure, which is DOS-673. N=3 escalation would require a user-facing repair state beyond this local substrate packet; the local fallback remains manual re-pair if repeated audits are noticed.

- `env_clear()` + `HOME`/`USER` allowlist - CORRECT.
  Observed evidence: §2 line 62 and §11 lines 526-527 classify this as same-UID hardening rather than v1.4.3 local lifecycle behavior.
  Local reasoning: the scenario depends on the user's own process environment or same-UID code execution. That is not a local-to-local surface-session lifecycle bug under §1 lines 24-26; it belongs with federation/shared-hosting hardening.

- Audit `event_kind` hashing normalization - CORRECT.
  Observed evidence: §2 line 63 and §6.6 lines 383-387 acknowledge an existing inconsistency with `surface_runtime` hashing while permitting plaintext ids in same-user-readable audit JSON.
  Local reasoning: this is normalization, not a v1.4.3 shipping failure. It does not change session validity, keychain cleanup, or sentinel behavior.

- Audit coalesce window - CORRECT.
  Observed evidence: cycle-1 review lines 72-86 derived the blast-radius formula as active sessions times startup attempts. V1.1 explicitly sizes local defaults at §6.6 lines 389-392 and resolves the open question at §12 line 542.
  Local reasoning: at 1-3 active sessions times approximately one startup per outage, the expected volume is 1-3 JSONL entries. That is acceptable operational visibility, not an audit flood, for v1.4.3 local scope.

- `stop_async` graceful drain + late-writer contract - CORRECT.
  Observed evidence: cycle-1 review lines 44-57 targeted the proposed `stop_async` design. V1.1 removes that design at §2 line 65, §5.4 lines 273-309, and §6.3 lines 347-356.
  Local reasoning: DOS-675's user-visible failure is stale sentinel redirection after stop/drop. Calling `explicit_sentinel_cleanup()` before abort, required by §7 lines 421-426 and gated by §9 lines 463-464, gives that fix without claiming DB writer drain.

- External-side-effect contract both directions - CORRECT.
  Observed evidence: cycle-1 review lines 88-94 named the DB/keychain inconsistency class. V1.1 accepts the class at §2 line 66 and §11 lines 532-534 while shipping idempotent best-effort cleanup with audit at §5.3 lines 262-270.
  Local reasoning: for v1.4.3 local, the immediate lifecycle bug is explicit cleanup on known transitions without writer-lock IO. Durable reconciliation across both directions is the federation-scale mechanism, not required to make the local transitions work.

- `Drop` audit-loss inventory - CORRECT.
  Observed evidence: §2 line 67 says informational only; §7 line 426 explicitly states `Drop` does not call async DB flush and crash-stop is tolerated.
  Local reasoning: no local user-visible lifecycle fix depends on new audit inventory. The required user-visible fix is sentinel cleanup before abort, not guaranteed audit flush from `Drop`.

## New cycle-2 findings [if any]

No material cycle-2 findings.

Non-blocking editorial note: §2 line 52 says the `db_writer_observer` replacement is "§8 fixture #8", but §8 lines 446-447 and §9 lines 466-470 show the mocked-slow keychain replacement is fixture #16. The enforcement substance is correct; the changelog pointer should be corrected opportunistically.

## Notes

- I did not re-fire the cycle-1 CRITICAL/HIGH/MEDIUM items that V1.1 explicitly deferred under §2 lines 59-67 and §11 lines 513-537.
- I did not escalate to L6 because no deferred item is wrong-for-local under §14 lines 576-581.
- Observed source spot-checks align with packet assumptions: current rehydration filters `s.revoked_at IS NULL`, current `stop`/`Drop` abort without sentinel cleanup, and current `delete_session_master_key` is idempotent for missing entries. Those observations support the V1.1 local fix/defer split rather than contradicting it.
