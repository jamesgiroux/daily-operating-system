# L0 Packet A — Surface session lifecycle hardening

**Current revision: V1.1.1 (cycle-2 code-reviewer text fixes, 2026-05-17). See §2 Changelog.**

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

**Intelligence Loop integration check — exempt.** No claim/table/surface added;
no provenance/trust impact; no signal change; no runtime context surface
consumes new state; no feedback loop change. Purely substrate operational
hardening on existing primitives. CLAUDE.md §"Critical Rules — Intelligence
Loop integration check" does not apply.

This packet is intentionally narrow. It does NOT include the WP preview /
runtime render stabilization (DOS-671/672) — that is L0 Packet B, separate
review track, separate PR.

## 2. Changelog

- **V1.1.1 (2026-05-17, cycle-2 text fixes):** All 4 cycle-2 reviewers
  returned non-BLOCK verdicts (codex challenge APPROVE, CSO APPROVE,
  code-reviewer CONDITIONAL APPROVE, codex consult CONDITIONAL APPROVE).
  Both CONDITIONAL APPROVE verdicts surface text-only findings — no design
  change. Folded inline:
  - **§5.1a** keychain test-seam precedent corrected. Real precedent is `src-tauri/src/db/key_provider.rs:205-213` (`KeychainBackend` trait), `:215-251` (`SecurityCliKeychain` production impl), `:302-307` (`with_keychain_for_tests` injection), `:1052-1079` (`FakeKeychain`). Gravatar uses free functions with retry helper, NOT a trait — the V1.1 claim was wrong (caught by code-reviewer F-C2-1 + codex consult LOW, two-reviewer triangulation).
  - **§5.4** clarified — `explicit_sentinel_cleanup()` runs BEFORE the existing `shutdown.send(true) + abort()` pair at `surface_runtime/mod.rs:415-419` (`stop`) and `:471-475` (`Drop::drop`). The ordering is the load-bearing part of the fix (code-reviewer F-C2-2 LOW).
  - **§4** `write_runtime_sentinel` line corrected — `:818` not `~:880` (code-reviewer F-C2-3 LOW).
  - **§5.3** `MarkSessionRevoked` plumbing detail added — `SignedSessionFailure::SessionExpired` variant must carry `surface_client_id` from `validate_signed_session_readonly` (at `surface_pairing.rs:888-891`) so `write_action()` (at `:1040-1047`) can populate the extended action payload (codex consult implementation note in validation #3).
  - **§8 + §7** AC↔fixture mapping table added inline (codex consult MEDIUM). Some ACs map to CI invariants (#9 → §9 invariant #1) rather than fixtures; some are partially covered (#9 struct-shape is implicitly proven by construction in fixtures 10-15, acceptable per code-reviewer's note). Mapping table makes coverage explicit; no new fixtures added.
  - **§2 V1.1 entry** corrected — "§8 fixture #8" was a typo; should read "§8 fixture #16" (codex challenge editorial nit).
  - All cycle-2 review files at `.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-*.md`.

- **V1.1 (2026-05-17, Path-α trim):** Cycle-1 fold against all 4 reviewer
  verdicts (CSO CA, codex consult CA, code-reviewer CA, codex challenge
  BLOCK). Same Path-α trim pattern as v1.4.2 W4-F V3.2 — each finding is
  individually justifiable but several reintroduce remote-shape defenses
  that v1.4.2 explicitly rejected and that this packet committed to honoring
  (§1: "preserves the local-to-local threat model committed to in v1.4.2").
  - **FOLDED (real local issues):**
    - **§5.3** suspicious-replay revoke path (`surface_pairing.rs:1383`) added to the cleanup-wiring enumeration. Three call sites of `revoke_pairing_row`, not two: explicit `revoke_pairing`, `revoke_existing_pairing_for_site`, `record_signed_transport_failure`. Two reviewers (codex consult + code-reviewer) caught this.
    - **§5.3** `KeychainCleanupTarget` collection moved INSIDE the same SQLite transaction as the revoke step. Codex challenge HIGH: without this, a second lifecycle path can change `revoked_at` between collection and revoke; collecting after-revoke with `revoked_at IS NULL` returns an empty target. Required form: `SELECT session_id FROM surface_client_sessions WHERE surface_client_id = ? AND pairing_epoch = ?` inside the transaction before `revoke_pairing_row`. Cleanup IO (keychain delete) still runs after commit.
    - **§5.3** `SignedSessionWriteAction::MarkSessionRevoked` payload extended to carry `surface_client_id` alongside `session_id` (codex consult HIGH). `delete_session_master_key` needs both; today the action carries only session_id. The dispatch site loads `row.surface_client_id` already (`surface_pairing.rs:754-755` / `1112-1114`); just plumb it through.
    - **§5.4** simplified — `stop_async` graceful-drain-with-timeout REPLACED with simpler shape: `explicit_sentinel_cleanup()` helper called from `stop` and `Drop` before `abort()`. `JoinHandle` ownership conflict (codex consult HIGH) thereby moot — no new join field needed in `RunningEndpoint`. The listener task continues to be aborted on stop/drop; the post-loop session-activity flush continues to run on graceful exit only (best-effort, as today). The `2s graceful timeout` design was over-engineering for a single-user local runtime where `Drop` is the dominant teardown path.
    - **§5.4** Tauri shutdown hook — packet now requires the existing tray quit (`lib.rs:584-585`) and `Drop` impl (`mod.rs:468-478`) to call `explicit_sentinel_cleanup()`. NO new `RunEvent::ExitRequested` wiring (codex consult HIGH deferred — see Deferred below). Local single-user runtime: tray-quit + Drop covers the realistic exit paths.
    - **§9** CI invariant #4 (`db_writer_observer`) REMOVED. The primitive does not exist (code-reviewer + codex challenge HIGH). Replaced with §8 fixture #16 assertion: mocked-slow keychain CLI proves the SQLite writer lock is released within 10ms of DB commit, sufficient enforcement for the cleanup-outside-tx invariant.
    - **§5.1** `security` CLI mock seam specified — `#[cfg(test)] thread_local!` static or trait-based dispatch. Pick: trait `KeychainBackend` with `RealKeychain` (production) and `MockKeychain` (tests). Pattern matches existing `gravatar/keychain.rs` (code-reviewer).
    - **§4** reuse audit cites the existing "transaction returns artifacts, caller runs side effects best-effort outside" pattern at `services/accounts.rs:3514-3548` (and 1364/1417/1465). `KeychainCleanupTarget` IS this pattern; the precedent is the right reuse anchor (code-reviewer).
    - **§4** line numbers re-synced — `persist_session_master_key` :79 not :60; `remove_runtime_sentinel` :887 not :883 (code-reviewer LOW).
    - **§1** Intelligence Loop integration check exempt statement added (purely substrate operational hardening — no new claim/table/surface) (code-reviewer LOW).
    - **§6** primary anchor reframed — `W4-F-L0-packet.md` is on the unmerged `docs/v143-carry-forward` branch (PR #300). Cite the v1.4.2 project description threat-model section (already on dev) plus W4-E as a section-conventions example (code-reviewer BLOCKER).
    - **§8** 9 missing fixtures added to close the AC↔fixture gap flagged by codex consult LOW (fixtures #15-23 in V1.1).
  - **DEFERRED to v1.x-federation maintenance (filed under DailyOS Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):**
    - **Orphan-keychain-entry-forever (codex challenge CRITICAL):** revoked DB row with still-present keychain entry is not caught by rehydration. For local-to-local, same-UID process owns the orphan bytes; they are inert. The "user explicit-revoke then crash" window leaves dead bytes no malicious process under a different UID can read. Federation deployments where the keychain is shared cross-machine would need the outbox/backlog primitive; v1.4.3 does not. **Maintenance ticket title:** "Orphan keychain entry reconciliation for federation deployments". Folds: outbox table, startup sweep over revoked sessions, retry-on-fail.
    - **`Unavailable` split + N=3 bounded tolerance + user-facing repair UI (codex challenge HIGH + CSO MEDIUM):** locally, `Unavailable` → log + leave active is fine; user can manually re-pair if they notice repeated `Unavailable` audits. ACL-denial as a permanent state is a federation/multi-machine concern; same-UID local always has keychain access by design. **Maintenance ticket title:** "Surface keychain Unavailable taxonomy + bounded tolerance + user-facing repair escalation". V1.1 keeps the simple `Found` / `NotFound` / `Unavailable` (3-variant — not 4; `Corrupt` folded into `Unavailable { reason: "corrupt_payload" }` since both have the same local remediation: leave active + log).
    - **`env_clear()` + `HOME`/`USER` allowlist for `security` CLI (CSO LOW):** same-UID attacker has the keychain item already; `DYLD_INSERT_LIBRARIES` subversion of `security` CLI requires same-UID write access to the user's process environment, which is a stronger position than reading the keychain entry directly. **Maintenance ticket title:** "Harden Command::output environment for security CLI invocations". Real concern in deployments where `securityd` is shared across UIDs (federation hosting), not single-user local.
    - **Audit `event_kind` hashing of `session_id` (CSO LOW):** local audit log is at `~/.dailyos/audit.log`, single-user-readable. Plaintext session ids are fine for single-user analysis. Existing code at `surface_runtime/mod.rs:734-735` already uses `stable_hash_for_audit`; **the inconsistency is a separate maintenance concern**, not a v1.4.3 blocker. **Maintenance ticket title:** "Normalize audit event_kind session_id hashing across surface_runtime + surface_pairing".
    - **Audit 60s coalesce window (CSO MEDIUM + codex challenge MEDIUM):** local audit flood under sustained keychain outage is bounded by `active_sessions × startup_count`. Realistic local steady-state: 1-3 active sessions, ~1 startup per actual outage event. Not a flood. **Maintenance ticket title:** "Audit emission coalescing for surface_pairing rehydration".
    - **`stop_async` graceful-drain semantics + late-writer-completion contract (codex challenge HIGH):** V1.1 drops `stop_async` entirely. The existing `stop` + `Drop` paths get `explicit_sentinel_cleanup()` and remain sync-best-effort. Federation deployments with HTTP traffic during shutdown would need the drain contract; local-single-user does not. **Maintenance ticket title:** "Surface runtime stop_async with bounded writer-drain timeout for federation deployments".
    - **External-side-effect contract both directions (codex challenge MEDIUM):** the general class is real (DB-then-keychain AND keychain-then-DB inconsistency windows). Locally, both directions resolve to "user re-pairs if they notice"; no automatic reconciliation needed at v1.4.3 scale. **Maintenance ticket title:** "External-side-effect reconciliation contract for surface session lifecycle".
    - **`Drop` audit-loss inventory (CSO LOW):** informational; future-additions discipline is healthy but V1.1 doesn't require it as an AC.
  - **REJECTED (not findings against the packet):**
    - Codex consult Q4 "stop_async timeout calibration" — moot since `stop_async` is dropped.
  - **NET V1.1 RESULT:** packet acceptance criteria count: 19 (was 22 — dropped graceful-stop ACs #17, #19; merged Found check into AC #2; the rest become per-trim acceptance). Fixtures: 23 (was 14; +9 closing AC↔fixture gap). The packet is now back inside the local-to-local threat model it committed to. The CORE remains unchanged: 3-variant SessionKeyLookup, keychain cleanup on lifecycle transitions outside the tx, explicit sentinel cleanup on stop/Drop before abort. That CORE is the elegant answer to the local instability the W4-F L4 surfaced.

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
| Keychain write | `persist_session_master_key` | `src-tauri/src/services/surface_session_keychain.rs:79` |
| Keychain read | `load_session_master_key` (REPLACE return type) | `src-tauri/src/services/surface_session_keychain.rs:107` |
| Keychain delete (idempotent) | `delete_session_master_key` | `src-tauri/src/services/surface_session_keychain.rs:139` |
| Pairing revoke (DB) | `revoke_pairing_row` | `src-tauri/src/services/surface_pairing.rs:1993` |
| Session mark-revoked (DB) | `mark_session_revoked` | `src-tauri/src/services/surface_pairing.rs:2042` |
| Pairing mark-expired (DB) | `mark_pairing_expired` | `src-tauri/src/services/surface_pairing.rs:2059` |
| Pairing re-pair (DB) | `revoke_existing_pairing_for_site` | `src-tauri/src/services/surface_pairing.rs:1803` |
| Suspicious-replay revoke (DB) | `record_signed_transport_failure` → `revoke_pairing_row` | `src-tauri/src/services/surface_pairing.rs:1383` |
| Signed-write-action dispatch | `apply_signed_session_write_action` | `src-tauri/src/services/surface_pairing.rs:1105` |
| Session rehydration on startup | `rehydrate_sessions_from_keychain` | `src-tauri/src/surface_runtime/mod.rs:667` |
| Runtime sentinel write | `write_runtime_sentinel` | `src-tauri/src/surface_runtime/mod.rs:818` |
| Runtime sentinel remove | `remove_runtime_sentinel` | `src-tauri/src/surface_runtime/mod.rs:887` |
| Shutdown session flush | `flush_session_activity_on_shutdown` | `src-tauri/src/surface_runtime/mod.rs:753` |
| Listener task spawn | `tokio::spawn(run_listener(...))` | `src-tauri/src/surface_runtime/mod.rs:327` |
| Endpoint stop | `SurfaceEndpointState::stop` | `src-tauri/src/surface_runtime/mod.rs:404` |
| Endpoint drop | `impl Drop for SurfaceEndpointState` | `src-tauri/src/surface_runtime/mod.rs:468` |
| Tray quit | `app.exit(0)` from tray menu | `src-tauri/src/lib.rs:584` |

### Reuse pattern: transaction returns artifacts, caller runs side effects outside

`services/accounts.rs:3514-3548` (and `:1364`, `:1417`, `:1465`) establishes
the repo-wide pattern this packet uses for `KeychainCleanupTarget`:

1. SQLite transaction collects all DB-side state changes.
2. Transaction return value carries "things the caller must do after commit"
   as a structured artifact.
3. Caller runs the side effects (audit emission, external IO, in-memory
   cache eviction) after `with_transaction(...)` returns.
4. Side effects are idempotent / best-effort; failures log but do not
   trigger DB rollback (which already committed).

`KeychainCleanupTarget` IS this pattern, applied to keychain deletes. The
precedent already exists in services/accounts; V1.1 reuses it explicitly.

No new primitives are introduced. Every change is an in-place modification of
existing functions or a small wrapper that delegates to them.

## 5. What this packet authors net-new

### 5.1 `SessionKeyLookup` classified enum (DOS-673)

Replace `Option<[u8; KEY_BYTES]>` from `load_session_master_key` with:

```rust
pub enum SessionKeyLookup {
    Found([u8; KEY_BYTES]),
    NotFound,
    Unavailable { reason: String },     // covers ALL non-NotFound failures: CLI spawn fail, locked keychain, perm denied, corrupt payload
}
```

Classification rules (V1.1: 3-variant, not 4 — `Corrupt` folded into
`Unavailable` since both have identical local remediation: leave DB row
active + log diagnostic):
- `Found`: `security` exits 0, output decodes, length matches.
- `NotFound`: `security` exits with the documented "item not found" signatures (use `key_provider.rs:989-994` as the canonical list: stderr contains "could not be found", "item not found", or exit `-25300`).
- `Unavailable { reason }`: any other outcome — `security` spawn failure (`io::Error`), locked keychain, permission denied, daemon not running, UTF-8 failure, base64 failure, key-length mismatch. The `reason` string captures the specific cause for diagnostics; the remediation is the same (leave active + log).

### 5.1a `KeychainBackend` trait for test seam

Today `load_session_master_key` / `persist_session_master_key` /
`delete_session_master_key` spawn `std::process::Command` directly
(`surface_session_keychain.rs:46-58`). Negative fixtures #2 and #3 require
mocking. Introduce a small trait:

```rust
pub trait KeychainBackend: Send + Sync {
    fn find(&self, service: &str, account: &str) -> SessionKeyLookup;
    fn persist(&self, service: &str, account: &str, payload: &[u8]) -> Result<(), String>;
    fn delete(&self, service: &str, account: &str) -> Result<(), String>;
}

pub struct RealKeychain;
impl KeychainBackend for RealKeychain { /* spawn security */ }

#[cfg(test)]
pub struct MockKeychain { /* scriptable responses */ }
```

Free functions (`load_session_master_key` etc) keep their public signatures
and delegate to a process-wide `RealKeychain` by default; tests inject
`MockKeychain` via a `#[cfg(test)] thread_local!` override.

**Precedent:** the in-repo template is `src-tauri/src/db/key_provider.rs` —
defines `KeychainBackend` trait at `:205-213`, ships `SecurityCliKeychain`
production impl at `:215-251`, exposes `with_keychain_for_tests` injection
at `:302-307`, provides `FakeKeychain` for tests at `:1052-1079`. The
surface keychain mirrors this shape. (V1.1 incorrectly cited
`gravatar/keychain.rs`, which uses free functions with retry helper and
has NO trait; corrected in V1.1.1 — see §2.)

### 5.2 Update `rehydrate_sessions_from_keychain` match arms (DOS-673)

At `src-tauri/src/surface_runtime/mod.rs:667-686`, replace the `Some` / `None`
branching with `Found` / `NotFound` / `Unavailable`. Only `NotFound` revokes
the DB row with `keychain_entry_missing`. `Unavailable` emits an audit
diagnostic and leaves the DB row active for later reconciliation (next
startup; or user-initiated re-pair if the audit pattern persists). The audit
event uses the existing `pairing.session.key_unavailable` style (matches the
dotted convention of `pairing.session.key_missing` already emitted at
`mod.rs:723-740`).

### 5.3 `KeychainCleanupTarget` and lifecycle cleanup wiring (DOS-674)

Introduce:

```rust
pub struct KeychainCleanupTarget {
    pub surface_client_id: String,
    pub session_ids: Vec<String>,
}
```

Following the existing `services/accounts.rs:3514-3548` reuse pattern (see
§4): the cleanup target is **collected inside** the SQLite transaction, then
**returned from the transaction** as part of the existing
`with_transaction(...)` return tuple. The caller runs `delete_session_master_key`
for each session id in the target **after** the transaction commits.

**Why collection must be inside the transaction (codex challenge HIGH):**
If session ids are gathered outside the transaction, a concurrent lifecycle
path (suspicious-replay, expiry sweep, separate revoke) can change
`revoked_at` between collection and revoke; the cleanup target races with
the actual revocation. Required form inside the transaction, BEFORE
`revoke_pairing_row` runs:

```sql
SELECT session_id FROM surface_client_sessions
 WHERE surface_client_id = ?
   AND pairing_epoch = ?
   AND revoked_at IS NULL
```

This snapshot is the cleanup target; collecting after the revoke step would
return empty because `revoke_pairing_row` sets `revoked_at` on every matching
row.

**The three call sites of `revoke_pairing_row` (codex consult + code-reviewer
HIGH — V1.0 enumerated only two):**

1. `revoke_pairing` (explicit user revoke) — `surface_pairing.rs:1270` inside `db.with_transaction`.
2. `revoke_existing_pairing_for_site` (re-pair flow) — `surface_pairing.rs:1855` inside the pairing handshake's `db.with_transaction` at `:528-617`.
3. `record_signed_transport_failure` (suspicious-replay threshold) — `surface_pairing.rs:1383` inside its own `db.with_transaction` at `:1297-1404`.

All three must be wired to return `KeychainCleanupTarget` from their
respective transactions.

Lifecycle service functions changed:

| Function | Change |
|---|---|
| `revoke_pairing_row` | Now takes the in-tx collected session id snapshot as a parameter; the transaction's outer caller composes the cleanup target from it. |
| `revoke_pairing` | Returns `(AuditEvent, KeychainCleanupTarget)`. Tauri command at `commands/surface_runtime.rs:67-82` invokes `delete_session_master_key` after `state.db_write().await` returns. |
| `revoke_existing_pairing_for_site` | Returns `(Option<RevokedPairingRef>, KeychainCleanupTarget)`. Pairing handshake at `surface_pairing.rs:528-617` runs the keychain delete inside the transaction's after-commit step BEFORE persisting the new session key (`:623-636`). |
| `record_signed_transport_failure` | Returns `(SignedTransportFailureOutcome, KeychainCleanupTarget)` when revocation fires. Runtime caller at `surface_runtime/mod.rs:1526-1565` runs cleanup after `db_write().await` returns, before in-memory session eviction at `:1554-1565`. |
| `mark_session_revoked` | Now reads `surface_client_id` from the existing `surface_client_sessions` row; returns the `KeychainCleanupTarget` (single-session). Callers run keychain delete outside their writer closure. |
| `mark_pairing_expired` | Collects affected session ids via the in-tx snapshot SQL above before marking the pairing expired; returns the cleanup target. |
| `apply_signed_session_write_action` | Returns `Option<KeychainCleanupTarget>` so the runtime closure can carry it out of the `db_write` boundary at `surface_runtime/mod.rs:1126-1135`. Cleanup runs after the `.await`. |

**`SignedSessionWriteAction::MarkSessionRevoked` payload extension (codex
consult HIGH):** today the action carries only `session_id`
(`surface_pairing.rs:1021-1025`). `delete_session_master_key` needs both
`surface_client_id` AND `session_id`. The validation path already has
`row.surface_client_id` in scope; plumb it through the chain so the dispatch
site doesn't need a second query:

```rust
SignedSessionWriteAction::MarkSessionRevoked {
    surface_client_id: String,
    session_id: String,
    reason: &'static str,
}
```

**Plumbing path (codex consult cycle-2 implementation note):** the active
runtime read-path uses `validate_signed_session_readonly`, where the row is
loaded at `surface_pairing.rs:876-880`. The intermediate enum
`SignedSessionFailure::SessionExpired` (at `:888-891`) must first carry
`row.surface_client_id`. Then `write_action()` (at `:1040-1047`) can populate
the extended `MarkSessionRevoked` payload. Without that intermediate enum
extension, the dispatch site has no source for `surface_client_id`.

Cleanup runs **after** the SQLite transaction commits, not inside it. The
caller of each lifecycle function invokes `delete_session_master_key` for each
session id in the target. Failures log + emit audit but do not roll back the
DB transition (the delete is idempotent for missing entries —
`surface_session_keychain.rs:148-155`).

**Audit event naming (code-reviewer MEDIUM):** new cleanup events use the
dotted convention to match the existing `pairing.session.key_missing`
sibling: `pairing.session.key_cleaned` (success), `pairing.session.key_cleanup_failed`
(delete returned error).

### 5.4 Explicit sentinel cleanup helper (DOS-675) — Path-α trimmed

V1.0's "graceful `stop_async` with bounded timeout + `JoinHandle` ownership
refactor" was over-engineering for a local single-user runtime. V1.1 keeps
the sentinel cleanup change but drops the graceful-drain architecture (see
§2 changelog for the deferral rationale + maintenance ticket reference).

Extract sentinel removal from the post-`run_listener` block into a standalone
helper:

```rust
fn explicit_sentinel_cleanup() {
    remove_runtime_sentinel();  // already idempotent — treats missing as success
}
```

Call sites:
- The graceful listener task post-loop at `surface_runtime/mod.rs:329-331` (current location — keeps the existing call).
- `SurfaceEndpointState::stop` at `surface_runtime/mod.rs:404-421` — call `explicit_sentinel_cleanup()` BEFORE the existing `shutdown.send(true) + abort()` pair at `:415-419`. The ordering is load-bearing: sentinel must be removed before the listener is torn down, otherwise the file persists pointing at a dead port.
- `impl Drop for SurfaceEndpointState` at `surface_runtime/mod.rs:468-478` — call `explicit_sentinel_cleanup()` BEFORE the existing `shutdown.send(true) + abort()` pair at `:471-475`. Same ordering rule.
- The existing tray quit handler at `lib.rs:584-585` already calls `app.exit(0)` which triggers `Drop` on `AppState` (and therefore on `SurfaceEndpointState` via the existing managed-state cleanup). No new Tauri shutdown hook is added.

That's the entire fix. The listener task continues to be aborted on
stop/drop just like today; the post-loop `flush_session_activity_on_shutdown`
continues to run on graceful exit only (best-effort, as today — `Drop`
explicitly cannot await DB writers and does not pretend to). What changes is
that the sentinel file at `~/.dailyos/runtime-endpoint.json` is **reliably
removed** before abort runs, so a stale port doesn't redirect WP after Tauri
restart.

**`JoinHandle` ownership (codex consult HIGH) — moot.** Without `stop_async`,
no new join field is needed in `RunningEndpoint`. The existing ownership
model (join returned from `start_listener`, awaited by `run_until_stopped`)
is unchanged.

**`stop_async` late-write semantics (codex challenge HIGH) — moot.** Without
`stop_async`, no late-write contract to specify.

**Tauri `RunEvent::ExitRequested` hook (codex consult + code-reviewer HIGH)
— deferred.** For local single-user runtime, the tray-quit + `Drop` paths
cover the realistic exit paths. OS-quit / kill via Activity Monitor bypass
both — accepted: the sentinel is BEST-EFFORT cleanup, not durable
guarantee. On the next startup, the supervisor's bind-port logic detects
the stale sentinel and either binds successfully (sentinel pointed at a
stale port now free) or rotates (existing fallback). Filed as v1.x-federation
maintenance — see §2 changelog deferred list.

## 6. Directional decisions resolved at L0

### 6.1 Three-state classification, not boolean (V1.1: 3-variant, not 4)

`Option<MasterKey>` cannot tell `NotFound` from `Unavailable`. The fix must
introduce real classification — a `Result<Option<MasterKey>, LookupError>`
also works but the enum is clearer and the implementation cost is identical.
**Decision:** three-variant enum (`Found`, `NotFound`, `Unavailable`).
`Unavailable` is the single catch-all for any non-NotFound failure —
corrupt-payload, locked keychain, ACL refusal, daemon down, all map here.
The remediation is identical (leave DB row active + log diagnostic), so the
classification doesn't need finer subdivision at v1.4.3 scope. The
`reason: String` field carries the specific cause for operational
diagnostics. Finer taxonomy (Transient vs AccessDenied with
user-facing-repair escalation) deferred to v1.x-federation maintenance —
see §2 changelog.

### 6.2 Keychain IO outside the SQLite transaction

The investigation flagged this: `revoke_pairing_row` runs inside transaction
contexts (`src-tauri/src/services/surface_pairing.rs:1269-1272`, `1855`). If
keychain delete is added inside the transaction, the `security` CLI call holds
the SQLite writer lock — adding local contention to exactly the lifecycle path
v1.4.2 stabilized. **Decision:** commit the DB transition first, then run
keychain delete outside the transaction. Delete failures log + emit audit; the
DB transition is not rolled back.

### 6.3 Sync sentinel cleanup; flush remains graceful-only (V1.1: trimmed)

V1.0's "graceful `stop_async` with bounded timeout" was over-engineering
for a local single-user runtime. **Decision:** sentinel cleanup synchronous
on both `stop` and `Drop` (the v1.4.3 fix). Session activity flush stays
graceful-post-loop-only — same as today. `Drop` does not pretend to flush;
the existing best-effort semantics is explicitly accepted as adequate for
local single-user use. Federation deployments with HTTP traffic during
shutdown would benefit from the bounded-writer-drain `stop_async` design;
that work is filed to v1.x-federation maintenance.

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

### 6.6 Audit emission on lifecycle cleanup (V1.1: trimmed defaults)

Every lifecycle keychain delete emits an audit event with reason
(`session_revoked`, `pairing_expired`, `pairing_replaced`). Successful deletes
emit `pairing.session.key_cleaned`; failed deletes emit
`pairing.session.key_cleanup_failed` (the DB transition has already committed
either way). `Unavailable` lookups during rehydration emit
`pairing.session.key_unavailable`. This is operational visibility, not
security policy. Audit goes to the existing `~/.dailyos/audit.log` (append-only
JSONL); no new schema.

**Defaults sized for local-to-local (V1.1):** plaintext `session_id` /
`surface_client_id` in the audit JSON. Same-user-readable log file; no need
for `stable_hash_for_audit` here. The existing inconsistency in surface_runtime
(which already hashes) is real but is a separate normalization concern, not a
v1.4.3 lifecycle-hardening blocker — filed as v1.x maintenance.

**No coalesce window (V1.1):** local single-user with 1-3 active sessions and
~1 startup per realistic outage event produces 1-3 audit events per outage,
not a flood. Coalesce window deferred to v1.x-federation maintenance where
larger active-session counts make it load-bearing.

## 7. Acceptance criteria

### DOS-673 (keychain lookup classification)

1. `load_session_master_key` returns `SessionKeyLookup` (3 variants: `Found`, `NotFound`, `Unavailable { reason }`).
2. `security` exit-0 + decodable + correct-length → `Found`.
3. `security` exit with known "item not found" signatures (stderr "could not be found" / "item not found" / exit `-25300`) → `NotFound`. Reuses `key_provider.rs:989-994` matcher.
4. Any other outcome → `Unavailable { reason }` — spawn failure, locked keychain, permission denied, daemon not running, UTF-8 failure, base64 failure, key-length mismatch.
5. `rehydrate_sessions_from_keychain` revokes DB row with `keychain_entry_missing` ONLY on `NotFound`.
6. `Unavailable` emits `pairing.session.key_unavailable` audit (one event per occurrence), leaves DB row active.
7. No read-path uses the new enum — startup reconciliation only.
8. `KeychainBackend` trait + `RealKeychain` / `MockKeychain` (cfg(test)) — production code path unchanged in behavior; mock injection via `#[cfg(test)] thread_local!` override.

### DOS-674 (keychain cleanup on lifecycle transitions)

9. `KeychainCleanupTarget` struct exposes `surface_client_id` + `session_ids: Vec<String>`.
10. Three call sites of `revoke_pairing_row` are wired (V1.0 named only two): explicit `revoke_pairing` (`surface_pairing.rs:1270`), `revoke_existing_pairing_for_site` (`:1855`), `record_signed_transport_failure` (`:1383`).
11. Cleanup target session ids are collected INSIDE the same SQLite transaction as the revoke, via `SELECT session_id FROM surface_client_sessions WHERE surface_client_id = ? AND pairing_epoch = ? AND revoked_at IS NULL` BEFORE `revoke_pairing_row` runs.
12. Explicit revoke (`revoke_pairing`) deletes the revoked session(s)' keychain entries after the DB transaction commits and before its Tauri command caller returns audit events.
13. Re-pair (`revoke_existing_pairing_for_site`) deletes OLD session keys after the pairing-handshake transaction commits and BEFORE persisting NEW session key (`surface_pairing.rs:623-636`).
14. Suspicious-replay (`record_signed_transport_failure`) deletes revoked session keys after its transaction commits and before in-memory session eviction (`surface_runtime/mod.rs:1554-1565`).
15. `SignedSessionWriteAction::MarkSessionRevoked` payload extended to carry `surface_client_id` AND `session_id`. `apply_signed_session_write_action` returns `Option<KeychainCleanupTarget>`; the runtime closure carries it out of the `db_write` boundary; cleanup runs after `.await`.
16. `mark_pairing_expired` collects affected session ids via the in-tx SELECT before marking the pairing expired; returns the cleanup target.
17. Keychain delete failure emits `pairing.session.key_cleanup_failed` audit + logs; DB transition is NOT rolled back. Successful delete emits `pairing.session.key_cleaned`.
18. Cleanup runs OUTSIDE the SQLite transaction. No keychain IO inside any `db_write` / `with_transaction` closure.
19. Repeated cleanup of the same target is harmless (`delete_session_master_key` is already idempotent for missing entries — `surface_session_keychain.rs:148-155`).

### DOS-675 (shutdown cleanup reachability)

20. `explicit_sentinel_cleanup()` helper exists; called from listener post-loop (`surface_runtime/mod.rs:329-331` — keeps existing call), `SurfaceEndpointState::stop` (`:404-421`, before existing `abort()`), and `impl Drop` (`:468-478`, before existing `abort()`).
21. After `stop` or `Drop`, the runtime sentinel file at `~/.dailyos/runtime-endpoint.json` is removed — verified by integration test.
22. Repeated `stop` / repeated `Drop` calls are harmless (idempotent).
23. `Drop` does NOT call async DB flush. Existing best-effort semantics is explicitly accepted — crash-stop is tolerated, just like today.

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `dos673_lookup_classifies_found` | `security` exit-0 + valid base64 + correct length → `Found(key)` with key bytes matching persisted value |
| 2 | `dos673_lookup_classifies_not_found` | `security` returns documented item-not-found (each of: "could not be found" stderr, "item not found" stderr, exit `-25300`) → `NotFound` |
| 3 | `dos673_lookup_classifies_unavailable_spawn_failure` | `MockKeychain` returns `io::Error` from spawn → `Unavailable { reason: "spawn_failure" }` |
| 4 | `dos673_lookup_classifies_unavailable_locked` | `MockKeychain` returns "user interaction not allowed" stderr → `Unavailable` |
| 5 | `dos673_lookup_classifies_unavailable_corrupt_base64` | `MockKeychain` returns exit-0 + invalid base64 → `Unavailable { reason: "corrupt_payload" }`, DB row not revoked |
| 6 | `dos673_lookup_classifies_unavailable_wrong_length` | `MockKeychain` returns exit-0 + valid base64 with wrong byte count → `Unavailable`, DB row not revoked |
| 7 | `dos673_rehydration_revokes_only_not_found` | Mixed-lookup-result rehydration: only `NotFound` rows transition to `keychain_entry_missing` revoked |
| 8 | `dos673_unavailable_emits_audit_diagnostic` | `Unavailable` lookup emits exactly one `pairing.session.key_unavailable` event per occurrence; DB row remains active |
| 9 | `dos673_keychain_backend_trait_seam` | Production code path uses `RealKeychain`; test cfg overrides to `MockKeychain` without changing the public function signatures |
| 10 | `dos674_revoke_deletes_session_key` | Explicit `revoke_pairing` followed by `MockKeychain.find()` returns `NotFound` for the revoked session |
| 11 | `dos674_repair_deletes_old_session_keys_only` | Re-pair (`revoke_existing_pairing_for_site`) deletes OLD session entries; NEW session entry persists in keychain |
| 12 | `dos674_expiry_deletes_session_key` | `mark_pairing_expired` cleanup target contains the right session ids; `MockKeychain` confirms deletion post-tx |
| 13 | `dos674_suspicious_replay_deletes_session_key` | `record_signed_transport_failure` reaches replay threshold; cleanup runs after `db_write().await` returns; session key deleted before in-memory eviction |
| 14 | `dos674_mark_session_revoked_carries_surface_client_id` | `SignedSessionWriteAction::MarkSessionRevoked` payload includes `surface_client_id`; `apply_signed_session_write_action` returns `Option<KeychainCleanupTarget>` populated correctly |
| 15 | `dos674_cleanup_target_collected_inside_transaction` | The `SELECT session_id ... revoked_at IS NULL` runs BEFORE `revoke_pairing_row` inside the same `with_transaction` closure; verified by transaction-call-order assertion |
| 16 | `dos674_cleanup_outside_transaction` | `MockKeychain.delete()` artificially slow (1s); SQLite writer lock released within 10ms of DB commit (replaces V1.0 invariant #4) |
| 17 | `dos674_keychain_delete_failure_does_not_rollback_db` | `MockKeychain.delete()` returns error; DB row stays revoked, `pairing.session.key_cleanup_failed` audit records the failure |
| 18 | `dos674_cleanup_idempotent` | Repeated cleanup of the same target is harmless (mock confirms second delete returns the missing-entry success path) |
| 19 | `dos674_no_keychain_io_inside_writer_closure` | grep / AST gate: no `KeychainBackend` method called from a closure passed to `db_write` / `with_transaction` (CI invariant #2 in §9) |
| 20 | `dos675_sentinel_cleaned_on_stop` | After `stop`, sentinel file at `~/.dailyos/runtime-endpoint.json` does not exist |
| 21 | `dos675_sentinel_cleaned_on_drop` | Dropping `SurfaceEndpointState` removes the sentinel before the listener task aborts |
| 22 | `dos675_repeated_stop_is_idempotent` | Two `stop` calls in a row; neither panics, sentinel cleaned exactly once |
| 23 | `dos675_drop_no_async_flush` | `Drop` does NOT block on DB writer; verified by drop with held writer lock |

### 8.1 AC → fixture/invariant mapping (added in V1.1.1 per codex consult)

Not every AC maps to a single fixture; some are enforced by CI invariants
or implicitly proven by other fixtures. Explicit mapping:

| AC | Coverage |
|---|---|
| AC #1 (`SessionKeyLookup` 3-variant return type) | §9 CI invariant #1 (grep gate) — not a runtime fixture |
| AC #2 (Found classification) | Fixture #1 |
| AC #3 (NotFound classification) | Fixture #2 |
| AC #4 (Unavailable catch-all) | Fixtures #3, #4, #5, #6 (spawn-fail, locked, corrupt-base64, wrong-length — one per identified failure mode) |
| AC #5 (rehydration revokes only NotFound) | Fixture #7 + §9 invariant #3 |
| AC #6 (Unavailable audit emission) | Fixture #8 |
| AC #7 (no read-path enum use) | §9 invariant #1 (grep gate for `SessionKeyLookup` outside startup rehydration) — fold into invariant if not already explicit |
| AC #8 (`KeychainBackend` trait seam) | Fixture #9 |
| AC #9 (`KeychainCleanupTarget` struct shape) | Implicitly proven by fixtures #10-#15 which construct and consume the struct (acceptable per code-reviewer cycle-2 note) |
| AC #10 (three call sites wired) | Fixtures #10, #11, #13 (one per call site: explicit revoke, re-pair, suspicious-replay); §9 invariant #2 |
| AC #11 (in-tx collection BEFORE revoke) | Fixture #15 |
| AC #12 (explicit revoke deletes session keys) | Fixture #10 |
| AC #13 (re-pair deletes OLD before persisting NEW) | Fixture #11 |
| AC #14 (suspicious-replay cleanup) | Fixture #13 |
| AC #15 (MarkSessionRevoked extension) | Fixture #14 |
| AC #16 (mark_pairing_expired collects session ids) | Fixture #12 |
| AC #17 (cleanup failure audit + log) | Fixture #17 (failure case + `pairing.session.key_cleanup_failed`); success-case audit (`pairing.session.key_cleaned`) covered by fixtures #10-#14 which all emit it on success |
| AC #18 (cleanup OUTSIDE transaction) | Fixture #16 + §9 invariant #2 + fixture #19 |
| AC #19 (cleanup idempotent) | Fixture #18 |
| AC #20 (`explicit_sentinel_cleanup` called from 3 sites) | Fixture #20 (stop), fixture #21 (Drop), listener post-loop covered by existing call at `mod.rs:329-331` (no regression) |
| AC #21 (sentinel removed after stop/Drop) | Fixtures #20 + #21 |
| AC #22 (repeated stop/Drop idempotent) | Fixture #22 + extend to also cover Drop idempotence (V1.1.1 acknowledges this is currently stop-only; minor expansion of fixture #22 covers both) |
| AC #23 (Drop no async DB flush) | Fixture #23 + §9 invariant #5 |

## 9. CI invariants

| # | Invariant | Enforcement |
|---|---|---|
| 1 | `load_session_master_key` returns `SessionKeyLookup` not `Option` | grep gate fails if `pub fn load_session_master_key... -> Option<` reappears |
| 2 | No `KeychainBackend` method called inside a `db_write` / `with_write_transaction` closure | AST gate scans closure bodies for `KeychainBackend::{find,persist,delete}` invocations + grep gate on `delete_session_master_key\|load_session_master_key\|persist_session_master_key` inside writer closures. Fixture #16 + #19 provide runtime/AST proof. |
| 3 | Rehydration revocation gated on `NotFound` only | grep gate: the `revoked_reason = 'keychain_entry_missing'` insert path is reachable only from a `SessionKeyLookup::NotFound` match arm. |
| 4 | `SurfaceEndpointState::stop` and `impl Drop` always call `explicit_sentinel_cleanup` before `abort` | grep gate on `stop` body and `Drop::drop` body — `explicit_sentinel_cleanup()` precedes `endpoint.abort.abort()`. |
| 5 | `impl Drop` for `SurfaceEndpointState` does NOT call async functions | Rust compiler enforces (sync `Drop`); documented as invariant for future-additions discipline. |

V1.0 CI invariant "Runtime DB-writer-held-during-keychain-call counter"
(`db_writer_observer`) REMOVED in V1.1: the primitive does not exist in
`src-tauri/src/` (codex challenge HIGH + code-reviewer BLOCKER). Equivalent
enforcement is provided by §8 fixture #16 (mocked-slow keychain delete;
SQLite writer lock released within 10ms of DB commit).

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
- **Orphan-keychain-entry reconciliation (outbox / cleanup-backlog primitive).**
  Codex challenge CRITICAL — deferred to v1.x-federation maintenance. Local
  same-UID context means orphan bytes are inert; rationale documented in §2
  changelog V1.1.
- **`Unavailable` finer taxonomy + bounded N=3 tolerance + user-facing repair
  escalation.** Codex challenge HIGH + CSO MEDIUM — deferred to v1.x-federation.
  Local single-user remediation is "user notices repeated audits, re-pairs
  manually" which is acceptable at v1.4.3 scale.
- **`stop_async` with bounded writer-drain timeout.** Codex challenge HIGH —
  deferred. Local single-user runtime accepts best-effort sentinel + abort.
- **`env_clear()` + `HOME`/`USER` allowlist for `security` CLI.** CSO LOW —
  deferred. Same-UID attacker scenario is out of scope.
- **Audit `event_kind` session-id hashing normalization.** CSO LOW — separate
  maintenance concern (existing inconsistency in surface_runtime vs surface_pairing).
- **Audit emission coalesce window.** CSO MEDIUM + codex challenge MEDIUM —
  deferred. Local flood is bounded by single-user / single-restart.
- **External-side-effect contract both directions (DB↔keychain).** Codex
  challenge MEDIUM — deferred. The general class is real; local scope
  doesn't need the formal reconciliation contract.
- **Tauri `RunEvent::ExitRequested` shutdown hook.** Codex consult + code-reviewer
  HIGH — deferred. Tray-quit + Drop covers realistic exit paths; OS-kill is
  accepted (sentinel is best-effort).

## 12. Open questions for L0 reviewers — all V1.0 questions RESOLVED in V1.1

1. **V1.0 Q1 (Corrupt variant):** RESOLVED in V1.1 §5.1 — `Corrupt` folded into `Unavailable { reason: "corrupt_payload" }`. Remediation is identical (leave active + log); the 4th variant added no operational value.
2. **V1.0 Q2 (Audit flood coalesce):** RESOLVED in V1.1 §6.6 — no coalesce window at v1.4.3 scope. Local single-user with 1-3 active sessions × ~1 startup per realistic outage = 1-3 audit events. Coalesce filed as v1.x-federation maintenance.
3. **V1.0 Q3 (caller enumeration):** RESOLVED via codex consult Q1 — `load_session_master_key` has exactly one non-test caller (`rehydrate_sessions_from_keychain` at `surface_runtime/mod.rs:667`); plus two test sites at `surface_session_keychain.rs:176/183`. New enum satisfies all three.
4. **V1.0 Q4 (stop_async timeout):** RESOLVED in V1.1 §5.4 — `stop_async` design dropped. No timeout to calibrate.
5. **V1.0 Q5 (audit hash):** RESOLVED in V1.1 §6.6 — plaintext session_id at v1.4.3 scope (local single-user-readable audit log). Existing inconsistency with surface_runtime's `stable_hash_for_audit` filed as v1.x maintenance.

### New open questions for V1.1 cycle 2

V1.1 has no new open questions — all cycle-1 findings folded with explicit
disposition (FOLD / DEFER / REJECT). Cycle 2 reviewers should validate that:
- The Path-α trim correctly preserves the local-to-local threat model.
- The deferred items are correctly classified as remote/federation concerns, not local concerns.
- The §5.3 three-call-site enumeration is complete.
- The §8 fixture expansion adequately closes the V1.0 AC↔fixture gap.

## 13. Linear dependency edges

- v1.4.3 stabilization-A PR closes DOS-673, DOS-674, DOS-675.
- No upstream Linear dependencies — substrate already exists from v1.4.2 W4-F (PR #298 merged 2026-05-17).
- Downstream: every v1.4.3+ WP block consumes stable surface sessions; no block work depends on the SPECIFIC code paths this PR changes, but all block work assumes session lifecycle is sound. No hard block on the PR for any single downstream ticket.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | V1.1 cycle-2 focus |
|---|---|---|
| `/codex challenge` | adversarial | Re-verify the Path-α trim. Specifically: are the 7 deferred items truly local-irrelevant, or does any of them produce a v1.4.3-shipping bug? Cross-check the §11 NOT-owned list against the v1.4.2 W4-F V3.2 deferral set for class consistency. |
| `code-reviewer` (claude) | domain | Verify §5.3 enumerates ALL `revoke_pairing_row` callers (three, per V1.1). Verify §5.4 trim does not break any existing test or shutdown contract. Verify the `KeychainBackend` trait shape matches existing `gravatar/keychain.rs`. |
| `/codex consult` | implementation feasibility | Walk the three-call-site cleanup wiring through the existing transaction boundaries; flag any case where the new return type breaks a caller. Verify `MarkSessionRevoked` payload extension reaches every dispatch site cleanly. |
| `/cso` | **mandatory** | Confirm the Path-α trim does not weaken the local-to-local model. Specifically: (a) the `Unavailable` single-bucket classification is acceptable for local same-UID scope (vs the deferred split); (b) cleanup-outside-tx with no outbox is acceptable for local same-UID scope; (c) the `Drop`-then-abort sentinel-cleanup-first ordering provides the user-visible stability fix DOS-675 promises. |

**Convergence rule:** unanimous APPROVE required before code lands. Any reviewer
returning CONDITIONAL APPROVE → fold finding into V1.2 (or maintenance backlog
if remote-shape) and re-run all four reviewers. Cycle cap: 3 cycles before
escalation to L6.

**Cycle 2 special handling for codex challenge:** if codex challenge re-flags
any of the §2 changelog deferred items as CRITICAL/HIGH, the deferral
classification must be re-examined — but the L0 reviewer panel is not allowed
to overturn a Path-α trim that aligns with the v1.4.2 explicit threat-model
commitment. Persistent disagreement → L6 (James) decides whether v1.4.3
should re-open the federation scope.

## 15. Acceptance for L0 closure

- [ ] All 4 reviewers returned APPROVE (cycle 2 or later).
- [ ] All 23 acceptance criteria (§7 V1.1) are testable; per-criterion fixture mapped to §8 (23 fixtures, 1:1 mapping verified).
- [ ] All 5 CI invariants (§9 V1.1) have concrete grep/AST/runtime enforcement (V1.0 invariant #4 removed; equivalent enforcement via §8 fixture #16).
- [ ] All §12 V1.0 open questions resolved in V1.1 (5/5 — see §12).
- [ ] Landing shape (§10) confirmed: single PR or A1+A2 split.
- [ ] V1.1 deferred items filed as Linear maintenance tickets under project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` (DailyOS Maintenance & Production Quality). The 7 deferral ticket titles are in §2 changelog V1.1.
- [ ] No outstanding L0-cycle findings; packet is implementation-ready.

When all seven boxes check, L0 is closed and implementation begins. L1 (self)
proof bundle includes: cargo test output for new fixtures, audit log excerpt
showing cleanup events, macOS keychain hands-on verification (revoke followed
by `security find-generic-password` returning not-found).
