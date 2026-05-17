# L0 Packet A — Surface session lifecycle hardening

**Current revision: V1.0 (initial draft, 2026-05-17). See §2 Changelog.**

## 1. Header

Date: 2026-05-17
Project: v1.4.3 — WordPress Foundation
Wave: Stabilization (first v1.4.3 work, predates W1+ primitive translation)
Issues:
- DOS-673 — keychain lookup error vs miss; false revocation on transient keychain failure
- DOS-674 — revoked/expired sessions leak signing secrets in macOS keychain
- DOS-675 — shutdown cleanup runs after listener abort
Surface: Tauri runtime endpoint + macOS keychain
Primary code: `src-tauri/src/services/surface_session_keychain.rs`, `src-tauri/src/services/surface_pairing.rs`, `src-tauri/src/surface_runtime/mod.rs`
Primary anchor: `.docs/plans/dos-546/v1.4.2-project/01-project-description.md` §"Threat model: local-to-local"
Diagnostic anchor: `.docs/plans/v1.4.3-wp-foundation/stabilization-investigation.md`
Downstream dependencies: every v1.4.3+ WP block ride on top of stable surface sessions; v1.4.4 W1 substrate work assumes session lifecycle is sound.

This packet covers three confirmed defects in the surface session lifecycle
surfaced by codex adversarial review of PR #298 (W4-F integrated diff). All
three share a single substrate area (keychain + pairing + runtime endpoint
shutdown). All three are operational hardening on the v1.4.2 substrate — no
new claim model, no new ability, no new contract. The fix shape preserves the
local-to-local threat model committed to in v1.4.2: long-lived sessions, no
remote-shaped defenses, no read-path mutation.

This packet is intentionally narrow. It does NOT include the WP preview /
runtime render stabilization (DOS-671/672) — that is L0 Packet B, separate
review track, separate PR.

## 2. Changelog

- **V1.0 (2026-05-17):** Initial L0 draft. Authored from
  `stabilization-investigation.md` plus direct verification of the cited
  file:line anchors. Reviewer panel set to codex challenge + code-reviewer +
  codex consult + CSO (mandatory — keychain, signing-secret persistence,
  session lifecycle = trust boundary).

## 3. Status Snapshot

- Linear tickets: DOS-673, DOS-674, DOS-675 (all Backlog, all in v1.4.3 — WordPress Foundation).
- Investigation evidence: `stabilization-investigation.md` §"DOS-673 Confirmed Root Cause" / §"DOS-674 Confirmed Root Cause" / §"DOS-675 Confirmed Root Cause".
- All three root causes confirmed against current code (sha 9a33d347 on dev).
- Acceptance criteria specified per ticket in §7 below.
- Recommended landing shape: single PR with three commit groups; see §10 Interlocks.
- Reviewer panel: see §14.

## 4. Pre-work — substrate reuse audit

This packet REUSES the following existing primitives. The L0 reviewer panel
must reject any net-new primitive in this packet that already exists:

| Capability | Existing primitive | File:line |
|---|---|---|
| Keychain write | `persist_session_master_key` | `src-tauri/src/services/surface_session_keychain.rs:60` |
| Keychain read | `load_session_master_key` (REPLACE return type) | `src-tauri/src/services/surface_session_keychain.rs:107` |
| Keychain delete (idempotent) | `delete_session_master_key` | `src-tauri/src/services/surface_session_keychain.rs:139` |
| Pairing revoke (DB) | `revoke_pairing_row` | `src-tauri/src/services/surface_pairing.rs:1993` |
| Session mark-revoked (DB) | `mark_session_revoked` | `src-tauri/src/services/surface_pairing.rs:2042` |
| Pairing mark-expired (DB) | `mark_pairing_expired` | `src-tauri/src/services/surface_pairing.rs:2059` |
| Pairing re-pair (DB) | `revoke_existing_pairing_for_site` | `src-tauri/src/services/surface_pairing.rs:1803` |
| Session rehydration on startup | `rehydrate_sessions_from_keychain` | `src-tauri/src/surface_runtime/mod.rs:667` |
| Runtime sentinel write | `write_runtime_sentinel` | `src-tauri/src/surface_runtime/mod.rs:~880` |
| Runtime sentinel remove | `remove_runtime_sentinel` | `src-tauri/src/surface_runtime/mod.rs:883` |
| Shutdown session flush | `flush_session_activity_on_shutdown` | `src-tauri/src/surface_runtime/mod.rs:753` |
| Listener task spawn | `tokio::spawn(run_listener(...))` | `src-tauri/src/surface_runtime/mod.rs:327` |
| Endpoint stop | `SurfaceEndpointState::stop` | `src-tauri/src/surface_runtime/mod.rs:404` |
| Endpoint drop | `impl Drop for SurfaceEndpointState` | `src-tauri/src/surface_runtime/mod.rs:468` |

No new primitives are introduced. Every change is an in-place modification of
existing functions or a small wrapper that delegates to them.

## 5. What this packet authors net-new

### 5.1 `SessionKeyLookup` classified enum (DOS-673)

Replace `Option<[u8; KEY_BYTES]>` from `load_session_master_key` with:

```rust
pub enum SessionKeyLookup {
    Found([u8; KEY_BYTES]),
    NotFound,
    Unavailable { reason: String },     // CLI spawn fail, locked keychain, perm denied
    Corrupt { reason: String },         // utf-8 fail, base64 fail, length mismatch
}
```

Classification rules:
- `Found`: `security` exits 0, output decodes, length matches.
- `NotFound`: `security` exits with the documented "item not found" status / stderr signature.
- `Unavailable`: `security` spawn failure (`io::Error`), exit status that indicates the keychain itself is unreachable (locked, permission denied, daemon not running).
- `Corrupt`: `security` exit 0 but `String::from_utf8` / `base64::decode` / length check fails. The entry exists but its payload is wrong shape.

### 5.2 Update `rehydrate_sessions_from_keychain` match arms (DOS-673)

At `src-tauri/src/surface_runtime/mod.rs:667-686`, replace the `Some` / `None`
branching with `Found` / `NotFound` / `Unavailable` / `Corrupt`. Only `NotFound`
revokes the DB row with `keychain_entry_missing`. `Unavailable` and `Corrupt`
emit an audit diagnostic and leave the DB row active for later reconciliation.

### 5.3 `KeychainCleanupTarget` and lifecycle cleanup wiring (DOS-674)

Introduce:

```rust
pub struct KeychainCleanupTarget {
    pub surface_client_id: String,
    pub session_ids: Vec<String>,
}
```

Lifecycle service functions changed to return cleanup targets (or perform
post-commit cleanup before returning):

| Function | Change |
|---|---|
| `revoke_pairing_row` (and its callers `revoke_pairing` / similar) | Returns `KeychainCleanupTarget` for the revoked session(s). |
| `mark_session_revoked` | Returns the revoked `session_id` for cleanup. |
| `mark_pairing_expired` | Collects affected session ids before/during the expiry transition; returns cleanup target. |
| `revoke_existing_pairing_for_site` | Collects old session ids; returns cleanup target for the OLD pairing before persisting the NEW one. |

Cleanup runs **after** the SQLite transaction commits, not inside it. The
caller of each lifecycle function invokes `delete_session_master_key` for each
session id in the target. Failures log + emit audit but do not roll back the
DB transition (the delete is idempotent and retried on later reconciliation).

### 5.4 Explicit sentinel cleanup helper + graceful async stop (DOS-675)

Extract sentinel removal from the post-`run_listener` block into a standalone
helper:

```rust
fn explicit_sentinel_cleanup() {
    remove_runtime_sentinel();  // already idempotent — treats missing as success
}
```

Call sites:
- The graceful listener task post-loop (current location).
- `SurfaceEndpointState::stop` — before the `abort()`.
- `impl Drop` — before the `abort()`.

Graceful async stop path:
- `RunningEndpoint` (`src-tauri/src/surface_runtime/mod.rs:182`) gains a
  `join: Option<JoinHandle<()>>` field alongside the existing `abort` handle.
- Add `pub async fn stop_async(&self, timeout: Duration) -> StopResult`:
  - sends shutdown signal,
  - awaits the join handle with a bounded timeout (e.g. 2s),
  - if timeout fires, runs `explicit_sentinel_cleanup()` and aborts.
- Tauri's normal shutdown flow calls `stop_async`. `Drop` keeps best-effort
  sentinel cleanup + abort (sync, no async DB flush — explicitly accepted).

`flush_session_activity_on_shutdown` keeps its current location inside the
graceful post-loop block. The graceful async stop is what reaches it on normal
exit. `Drop` does NOT pretend to flush — that's a sync context with no DB
writer access, and the investigation explicitly calls this out.

## 6. Directional decisions resolved at L0

### 6.1 Three-state classification, not boolean

`Option<MasterKey>` cannot tell `NotFound` from `Unavailable`. The fix must
introduce real classification — a `Result<Option<MasterKey>, LookupError>`
also works but the enum is clearer and the implementation cost is identical.
**Decision:** four-variant enum (`Found`, `NotFound`, `Unavailable`,
`Corrupt`). `Corrupt` is split from `Unavailable` because corrupt-payload is
a different remediation signal (entry exists; user-initiated re-pair, not
keychain-daemon repair).

### 6.2 Keychain IO outside the SQLite transaction

The investigation flagged this: `revoke_pairing_row` runs inside transaction
contexts (`src-tauri/src/services/surface_pairing.rs:1269-1272`, `1855`). If
keychain delete is added inside the transaction, the `security` CLI call holds
the SQLite writer lock — adding local contention to exactly the lifecycle path
v1.4.2 stabilized. **Decision:** commit the DB transition first, then run
keychain delete outside the transaction. Delete failures log + emit audit; the
DB transition is not rolled back.

### 6.3 Graceful async stop, not sync drop

The investigation rejects "synchronous cleanup on stop/drop" as the full fix
because session activity flush is async (requires `AppState` + DB writer).
**Decision:** sentinel cleanup synchronous on both `stop` and `Drop`; session
activity flush only on graceful `stop_async` with bounded timeout. `Drop` is
best-effort sentinel-only.

### 6.4 No read-path keychain probing

DOS-673 fix is startup-reconciliation only. Render-path reads continue to
authenticate against the in-memory signed-transport state cached at session
register time, NOT against a fresh keychain probe per request. The read-path
read-only contract from v1.4.2 W4-F is preserved.

### 6.5 No short-TTL eviction as cleanup mechanism

The investigation rejects "let the TTL expire and clean up dead keys naturally"
because v1.4.2 set TTL to 365 days for local-to-local. **Decision:** explicit
cleanup follows user intent and lifecycle state. The 365-day TTL is the right
local-to-local choice; cleanup is event-driven, not timer-driven.

### 6.6 Audit emission on lifecycle cleanup

Every lifecycle keychain delete emits an audit event with reason
(`session_revoked`, `pairing_expired`, `pairing_replaced`). This is operational
visibility, not security policy. Audit goes to the existing audit table; no
new schema.

## 7. Acceptance criteria

### DOS-673 (keychain lookup classification)

1. `load_session_master_key` returns `SessionKeyLookup` (4 variants).
2. `security` exit-0 + decodable + correct-length → `Found`.
3. `security` exit with known "item not found" signature → `NotFound`.
4. `security` spawn failure / locked-keychain / permission-denied → `Unavailable { reason }`.
5. `security` exit-0 with malformed payload (utf-8 / base64 / length) → `Corrupt { reason }`.
6. `rehydrate_sessions_from_keychain` revokes DB row with `keychain_entry_missing` ONLY on `NotFound`.
7. `Unavailable` and `Corrupt` emit audit diagnostic (one event per occurrence), leave DB row active.
8. No read-path uses the new enum — startup reconciliation only.

### DOS-674 (keychain cleanup on lifecycle transitions)

9. `KeychainCleanupTarget` struct exposes `surface_client_id` + `session_ids: Vec<String>`.
10. Explicit revoke (`revoke_pairing`) deletes the revoked session(s)' keychain entries after the DB transaction commits.
11. Re-pair (`revoke_existing_pairing_for_site`) deletes OLD session keys before persisting NEW session keys; uses captured-old-session-ids (not `surface_client_id`-wide delete).
12. Session expiry (`SignedSessionWriteAction::MarkSessionRevoked` callers and `mark_pairing_expired`) deletes affected session keys.
13. Keychain delete failure logs + emits audit; DB transition is NOT rolled back.
14. Cleanup runs OUTSIDE the SQLite transaction. The lifecycle function returns the cleanup target; the caller invokes delete.
15. Repeated cleanup of the same target is harmless (`delete_session_master_key` is already idempotent for missing entries — `surface_session_keychain.rs:148-155`).

### DOS-675 (shutdown cleanup reachability)

16. `explicit_sentinel_cleanup()` helper exists; called from listener post-loop, `stop`, and `Drop`.
17. `SurfaceEndpointState::stop_async(timeout: Duration) -> StopResult` exists; awaits join with timeout, runs sentinel cleanup if timeout fires.
18. Normal Tauri shutdown calls `stop_async` with bounded timeout (initial value: 2s; tunable).
19. `flush_session_activity_on_shutdown` runs on graceful async stop. Documented as best-effort: crash-stop is tolerated, just like today.
20. `Drop` does NOT call async DB flush. `Drop` does sync sentinel cleanup + abort.
21. After `stop` or `stop_async`, the runtime sentinel file at `~/.dailyos/runtime-endpoint.json` is removed (or marked stale) — verified by integration test.
22. Repeated `stop` / `stop_async` calls are harmless (idempotent).

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `dos673_lookup_classifies_not_found` | `security` returns documented item-not-found → `NotFound` |
| 2 | `dos673_lookup_classifies_unavailable` | Mocked spawn failure → `Unavailable`, DB row not revoked |
| 3 | `dos673_lookup_classifies_corrupt` | Mocked exit-0 + malformed base64 → `Corrupt`, DB row not revoked |
| 4 | `dos673_rehydration_revokes_only_not_found` | Mixed-lookup-result rehydration: only `NotFound` rows transition to `keychain_entry_missing` revoked |
| 5 | `dos674_revoke_deletes_session_key` | Explicit revoke followed by direct keychain read returns `NotFound` for the revoked session |
| 6 | `dos674_repair_deletes_old_session_keys_only` | Re-pair deletes OLD session entries; NEW session entry persists |
| 7 | `dos674_expiry_deletes_session_key` | `mark_pairing_expired` cleanup target deletes the right session ids |
| 8 | `dos674_cleanup_outside_transaction` | Mocked keychain CLI artificially slow (1s); SQLite writer lock released within 10ms of DB commit |
| 9 | `dos674_keychain_delete_failure_does_not_rollback_db` | Mocked delete returns error; DB row stays revoked, audit records cleanup failure |
| 10 | `dos675_sentinel_cleaned_on_stop` | After `stop`, sentinel file does not exist (or is marked stale) |
| 11 | `dos675_graceful_stop_runs_flush` | After `stop_async`, last_seen_at advances on active sessions |
| 12 | `dos675_timeout_aborts_with_sentinel_cleaned` | Mocked listener that ignores shutdown signal → `stop_async` timeout, sentinel still cleaned, abort fires |
| 13 | `dos675_repeated_stop_is_idempotent` | Two `stop` calls in a row; neither panics, sentinel cleaned once |
| 14 | `dos675_drop_no_async_flush` | `Drop` does NOT block on DB writer; verified by drop with held writer lock |

## 9. CI invariants

| # | Invariant | Enforcement |
|---|---|---|
| 1 | `load_session_master_key` returns `SessionKeyLookup` not `Option` | grep-based CI gate fails if `pub fn load_session_master_key... -> Option<` reappears |
| 2 | Every lifecycle transition site that revokes/expires a session also has cleanup wiring | grep for `mark_session_revoked\|mark_pairing_expired\|revoke_pairing_row` in service code; every caller chain must reach `delete_session_master_key` (verified by integration test, not static — too many control flow paths for grep) |
| 3 | No keychain IO inside a `db_write` closure | grep gate on the body of `db_write` / `with_write_transaction` for `delete_session_master_key\|load_session_master_key\|persist_session_master_key\|security` (this is approximation — full proof requires runtime check, see #4) |
| 4 | Runtime DB-writer-held-during-keychain-call counter is zero on lifecycle test runs | Test-only `db_writer_observer` records writer-lock duration; assert no lock exceeds N ms during lifecycle operations |
| 5 | `SurfaceEndpointState::stop` always calls `explicit_sentinel_cleanup` before `abort` | grep gate on `stop` body for `explicit_sentinel_cleanup` ordering before `abort` |
| 6 | `impl Drop` for `SurfaceEndpointState` does NOT call async functions | clippy or AST check that drop body does not contain `.await` (rust compiler enforces this for sync `Drop`, but the rule is documented as an invariant) |

## 10. Interlocks

DOS-673 and DOS-674 share a common test seam (`surface_session_keychain.rs`).
Both should land together; splitting them creates duplicate brittle macOS
`security` command setup in tests.

DOS-675 shares `surface_runtime/mod.rs` with both DOS-673 (rehydration path)
and DOS-674 (lifecycle cleanup paths via the bridge). The investigation notes
that landing DOS-675 alongside the keychain work avoids cross-PR rebases on
the same file.

**Landing shape:** single v1.4.3 stabilization-A PR with three commit groups:
1. DOS-673 — keychain lookup classification.
2. DOS-674 — keychain cleanup on lifecycle transitions.
3. DOS-675 — shutdown cleanup reachability.

If review size forces a split, the only valid split is:
- PR A1: DOS-673 (foundation — new enum, no behavior change to lifecycle paths).
- PR A2: DOS-674 + DOS-675 (depends on A1 for the cleanup target plumbing pattern).

Do NOT land DOS-674 without DOS-673 (cleanup target plumbing relies on the
enum's classification for correct edge cases).

## 11. What this packet explicitly does NOT own

- **DOS-671 / DOS-672 (WP preview/runtime render stabilization).** Separate L0
  packet (B). Different file surface (`wp/dailyos/` + `surface_runtime` render
  route + `composition_render_orchestrator` + `surface_client` bridge), different
  threat-model framing (render-path mutation removal, render-read decharge),
  different reviewer concerns (rate-budget gating, cache key correctness).
- **C1 starter kit** (block.json / render.php / producer / projection rule /
  integration fixture). Distinct work track; will get its own L0 packet after
  the stabilization tickets land.
- **Wave 1 primitive blocks** (Pill, HealthBadge, etc as Gutenberg blocks).
  Distinct work track. Depends on C1 starter kit existing.
- **Studio sandbox compatibility** for runtime discovery (C3 in v1.4.3 project
  description). Distinct work track; will get its own L0 packet.
- **Feedback write infrastructure** (W4-E nonce + W5-A click-bound router from
  v1.4.2 → v1.4.3). Distinct work track.
- **Audit / clean-machine validation** (DOS-576, DOS-577). Distinct work
  tracks; their L0 packets are already authored as part of v1.4.2.
- **Federation / multi-machine session model.** v1.4.2 explicitly punted
  federation hardening to a later release. This packet does not pre-emptively
  defend against remote threats.

## 12. Open questions for L0 reviewers

1. **(For codex challenge):** The `Corrupt` variant exists as a separate
   classification but is the remediation actually different from `Unavailable`?
   If the user has to re-pair either way, does the split add value, or should
   `Corrupt` collapse into `Unavailable { reason: "corrupt_payload" }`?
2. **(For CSO):** Audit emission on every `Unavailable` lookup could flood the
   audit log during a sustained macOS keychain outage. Should there be
   coalescing / rate-limiting on audit emission for this class? If yes, what
   coalesce window?
3. **(For code-reviewer):** Does any caller of `load_session_master_key`
   outside `rehydrate_sessions_from_keychain` exist today? If yes, the new
   enum's variants must satisfy that caller's semantics too. Grep shows only
   the rehydration site at L0 read; please confirm.
4. **(For codex consult):** `stop_async` timeout default of 2s — is that long
   enough to avoid aborting normal-shutdown work, short enough to avoid hanging
   Tauri quit? What's the typical post-loop work duration?
5. **(For CSO):** `KeychainCleanupTarget` carries plaintext session ids. These
   are not secret but are tied to user activity. Should the audit event for
   cleanup include the session id, or just a hash? Today's audit events already
   reference session ids in plaintext, so the precedent says yes — please
   confirm.

## 13. Linear dependency edges

- v1.4.3 stabilization-A PR closes DOS-673, DOS-674, DOS-675.
- No upstream Linear dependencies — substrate already exists from v1.4.2 W4-F (PR #298 merged 2026-05-17).
- Downstream: every v1.4.3+ WP block consumes stable surface sessions; no block work depends on the SPECIFIC code paths this PR changes, but all block work assumes session lifecycle is sound. No hard block on the PR for any single downstream ticket.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | Why |
|---|---|---|
| `/codex challenge` | adversarial | Specifically: stress-test the `SessionKeyLookup` classification against macOS `security` quirks (locked keychain mid-call, keychain access prompt timeout, daemon restart). Stress-test `stop_async` timeout against listener that's mid-DB-write at signal time. |
| `code-reviewer` (claude) | domain | Pairing service + runtime endpoint code is dense; the changes touch service-layer boundaries that have specific patterns (transaction handling, error propagation, audit emission). Independent read by the domain reviewer catches integration smells. |
| `/codex consult` | implementation feasibility | Walk the proposed `KeychainCleanupTarget` plumbing through every existing caller of the lifecycle service functions; flag callers where the new return type breaks. |
| `/cso` | **mandatory** | Keychain (signing-secret persistence), session lifecycle (revocation contract), and runtime shutdown (sentinel cleanup ordering) are all trust-boundary concerns. CSO must verify: (a) `Unavailable` classification does not weaken revocation in a real attack scenario; (b) cleanup-outside-transaction does not open a window where a revoked session retains a usable signing secret; (c) `Drop`-without-flush does not leak audit-relevant state on abnormal exit. |

**Convergence rule:** unanimous APPROVE required before code lands. Any reviewer
returning CONDITIONAL APPROVE → fold finding into V2 of this packet, re-run all
four reviewers. Cycle cap: 3 cycles before escalation to L6.

## 15. Acceptance for L0 closure

- [ ] All 4 reviewers returned APPROVE.
- [ ] All 22 acceptance criteria (§7) are testable; per-criterion fixture mapped (§8 has 14 — gap analysis in cycle 1).
- [ ] All 6 CI invariants (§9) have concrete grep/AST/runtime enforcement.
- [ ] All §12 open questions resolved.
- [ ] Landing shape (§10) confirmed: single PR or A1+A2 split.
- [ ] No outstanding L0-cycle findings; packet is implementation-ready.

When all six boxes check, L0 is closed and implementation begins. L1 (self)
proof bundle includes: cargo test output for new fixtures, audit log excerpt
showing cleanup events, macOS keychain hands-on verification (revoke followed
by `security find-generic-password` returning not-found).
