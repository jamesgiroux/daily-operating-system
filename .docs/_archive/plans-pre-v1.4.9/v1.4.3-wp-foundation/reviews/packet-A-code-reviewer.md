# L0 Packet A — code-reviewer verdict

**Verdict:** CONDITIONAL APPROVE

**Reviewer:** code-reviewer (claude), independent domain read
**Date:** 2026-05-17
**Packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md` V1.0
**Scope:** DOS-673, DOS-674, DOS-675 — lifecycle hardening on surface session keychain + pairing + runtime shutdown
**Citations:** all file:line references are against dev sha 9a33d347 (the packet's stated baseline).

Plan direction is sound. Three blockers, four substantive findings — fold into V2, then APPROVE.

---

## Summary

The packet correctly identifies three real defects and proposes the right
shape (classified enum, post-commit cleanup, async stop). It also correctly
constrains itself to operational hardening on the v1.4.2 substrate — no new
claim model, no read-path mutation, no remote-shaped defenses. That framing
matches the local-to-local threat model and is the right scope.

The substantive concerns are integration smells the packet doesn't surface:
one missing caller of `revoke_pairing_row`, one missing house-style
precedent it should cite, two test/CI claims that rest on infrastructure
that doesn't exist yet, and a missing wire-up step for the new
`stop_async`. None of these reframe the work. All are V2-fixable.

---

## Blockers — must resolve before L0 closure

### B1. Missing caller: `revoke_pairing_row` is invoked from THREE sites, packet enumerates TWO

Packet §5.3 lists `revoke_pairing_row` callers and §10 says DOS-674's
cleanup target plumbing reaches "every lifecycle transition site." The
packet's audit reaches only the two `revoke_pairing` direct call
(`src-tauri/src/services/surface_pairing.rs:1269-1272`) and
`revoke_existing_pairing_for_site` (`src-tauri/src/services/surface_pairing.rs:1855`).

It misses the third call site at `src-tauri/src/services/surface_pairing.rs:1383`,
inside `record_signed_transport_failure`:

```
1383:                    revoke_pairing_row(tx, &row, &now, "suspicious_replay")?;
```

This site:
- Runs inside `db.with_transaction(|tx| { ... })` opened at line 1297.
- Revokes a pairing (and therefore sessions) on suspicious-replay escalation.
- Will, under the packet's design, need to return a cleanup target out of
  the `with_transaction` closure so the caller can `delete_session_master_key`
  AFTER commit. This is the same pattern as `revoke_pairing` at 1269, but the
  closure boundary makes the plumbing different (the returned value of the
  closure needs to carry the cleanup target up to where the post-commit
  delete can run).
- Acceptance criterion #10 ("Explicit revoke … deletes the revoked
  session(s)' keychain entries after the DB transaction commits") doesn't
  cover this internal-suspicious-replay path. It should.

Action: add this call site to §5.3, add a per-criterion acceptance for
suspicious-replay-driven revoke cleanup (parallel to AC #10), and add a
fixture mirroring fixture #5 but driven by the 5-nonce-replay escalation
in `record_signed_transport_failure`.

### B2. Packet's claimed format anchor (W4-F-L0-packet.md) doesn't exist in the repo

§1 cites `.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md` as the
format conventions anchor. That file is not present:
- `find .docs/plans -name "W4-F*"` returns zero hits.
- The v1.4.2 project directory contains W3, W4-A, W4-A0, W4-B, W4-C, W4-D,
  W4-E packets. No W4-F.
- `git status` shows it as untracked at the repo root path level
  (`?? .docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md`) but the file
  is not present in the working tree at the time of this review.

Either the path is wrong or the anchor was never committed. Either way the
packet header's anchor needs to point at a real, durable artifact. The
nearest convention parallel is `.docs/plans/dos-546/v1.4.2-project/W4-E-L0-packet.md`
which uses a different section ordering than Packet A (Changelog after
header, no top-level "Status Snapshot" numbered section, etc.). If Packet A
intentionally diverges in section ordering from W4-E, say so; if not,
realign or update the anchor.

### B3. CI invariant #4 ("DB-writer-held-during-keychain-call counter") rests on infrastructure that doesn't exist

§9 invariant #4 says "Test-only `db_writer_observer` records writer-lock
duration; assert no lock exceeds N ms during lifecycle operations." A grep
across `src-tauri/src/` for `db_writer_observer`, `writer_lock_held`, or
`writer.lock.*duration` returns zero hits. There is no current
infrastructure for measuring `db_write` closure execution time from inside
a test.

This is not unfixable, but it's a NET-NEW substrate the packet treats as
existing. Either:
- Acknowledge the observer as a small implementation-time addition (its
  own line item under §5), with concrete shape (e.g., a thread-local
  `Instant` set at `db_write` entry and checked at exit, behind
  `#[cfg(test)]`), or
- Drop invariant #4 to "covered by §8 fixture #8 (`dos674_cleanup_outside_transaction`)
  asserting writer-lock release within 10ms of DB commit" — fixture #8 is
  the actual enforcement; invariant #4 as written reads like a separate gate
  but it isn't.

Pick one. Don't leave a CI invariant whose enforcement primitive doesn't
yet exist.

---

## Substantive findings — fold into V2

### F1. The substrate reuse audit (§4) misses the established "DB-transaction-returns-cleanup-target" precedent

§4 enumerates 14 reused primitives, all keychain/pairing/runtime specific.
It does not cite the established repo-wide pattern for "DB transaction
returns artifacts, caller runs side effects best-effort outside" — which is
*exactly* what `KeychainCleanupTarget` proposes.

Prior art lives at `src-tauri/src/services/accounts.rs:3354` (header
comment: "Wraps all DB writes in a transaction. Filesystem writes are
best-effort after commit.") and the implementation pattern at lines
3500-3548 where `db_write(|db| { ... return (artifacts...) })` returns the
artifacts and then `let _ = create_dir_all(...)`, `let _ = write_account_json(...)`
runs outside the closure. Header comments at
`src-tauri/src/services/accounts.rs:1364, 1417, 1465` and
`src-tauri/src/services/intelligence.rs:1711, 2119, 2263` explicitly call
out "post-commit side effects" and "best-effort post-commit work" as the
house pattern.

The packet's `KeychainCleanupTarget` IS this pattern. Calling out the
precedent in §4 (a) reassures L0 reviewers that DOS-674 isn't inventing a
new architecture, (b) gives the implementer the exact reference for closure
return-value plumbing, and (c) is consistent with the "REUSE BEFORE CREATE"
rule §4 opens with.

Action: add a row to §4: "Post-commit side-effect pattern (transaction
returns artifacts, caller runs side effects) | `services/accounts.rs:3514-3548` (and 1364/1417/1465 header docstrings)".

### F2. Audit event naming convention is split; packet doesn't pick

Existing audit `event_kind` values in `src-tauri/src/services/surface_pairing.rs`:
- Dotted: `pairing.session.key_missing` (mod.rs:728), `pairing.exfiltration.suspected_replay` (surface_pairing.rs:1351), `pairing.exfiltration.nonce_replay` (mod.rs:3175).
- Snake_case: `pairing_revoked` (surface_pairing.rs:1454), `pairing_created` (surface_pairing.rs:3167), `pairing_code_failed` (surface_pairing.rs:444).

The packet's §6.6 specifies cleanup-audit "reasons" (`session_revoked`,
`pairing_expired`, `pairing_replaced`) but doesn't specify the `event_kind`.
Given the keychain cleanup belongs to the same family as the existing
`pairing.session.key_missing` event (both are session-keychain-lifecycle
events emitted from outside a transaction), the dotted family is the closer
fit. Recommend pinning:
- `pairing.session.key_cleaned` (with `detail.reason` = `session_revoked` | `pairing_expired` | `pairing_replaced` | `suspicious_replay`)
- `pairing.session.key_cleanup_failed` (when `delete_session_master_key` returns Err)

Either way, pin it in §6.6 so the implementer doesn't have to invent
naming under L1 time pressure.

Also: §12 open question #2 (CSO: should audit emission on `Unavailable`
lookup coalesce during sustained keychain outage) is real. The audit log
backend is JSONL-file-based and uncoalesced — sustained keychain outage
across N sessions emits N events per rehydration cycle. Rehydration only
runs on startup so the bound is "N sessions per Tauri launch" — that bound
is fine. State this explicitly in §6.6 so the open question resolves
itself.

### F3. Tests for the new `SessionKeyLookup` variants require a mock seam that doesn't exist

§8 fixtures #2 and #3 call for "Mocked spawn failure" and "Mocked exit-0
+ malformed base64". The current implementation at
`src-tauri/src/services/surface_session_keychain.rs:46-74` invokes
`std::process::Command::new("security")` directly with no injection point.
Existing tests at lines 167-186 are real-keychain integration tests gated
by `#[cfg_attr(not(target_os = "macos"), ignore)]`.

To get fixtures #2-3 the impl needs a test seam — typically either:
- A `cfg(test)` `MOCK_SECURITY_OUTPUT: Mutex<Option<MockResponse>>` static
  the test can populate (cheap, intrusive), or
- A `trait KeychainCli { fn run(args: &[&str]) -> Result<Output, String>; }`
  with a real impl and a `#[cfg(test)] MockKeychainCli` (cleaner, larger
  refactor, would also benefit `gravatar/keychain.rs`).

Sibling module `gravatar/keychain.rs` has the same pattern and the same
test gap. Recommend pinning the chosen seam shape in §5.1 (the enum is the
visible contract change, but it's not testable without the seam) and
deciding whether to backport the seam to `gravatar/keychain.rs` in the same
PR or file as maintenance.

### F4. `stop_async` has no current Tauri caller; wire-up step is missing

§5.4 says "Tauri's normal shutdown flow calls `stop_async`." The current
Tauri shutdown flow does not call `.stop()` on the surface endpoint at
all. The window close handler at `src-tauri/src/lib.rs:611-624` calls
`window_clone.hide()` and `api.prevent_close()` — close is intercepted, the
process keeps running. The Drop impl at `src-tauri/src/surface_runtime/mod.rs:468-478`
is what actually fires when the process exits.

For DOS-675 acceptance criterion #18 ("Normal Tauri shutdown calls
`stop_async` with bounded timeout") to be testable, the packet needs to
specify WHERE in `lib.rs` the new hook gets wired:
- A `tauri::RunEvent::ExitRequested` handler in `app.run(|app, event| ...)`?
- An `on_window_event` arm for the main window's true-close (distinct from
  the hide-on-close intercept)?
- A new Tauri menu "Quit" action that explicitly calls `surface_runtime_endpoint.stop_async(...)`
  before `app.exit(0)`?

The investigation doc may have this. The packet should pull it in or punt
the wire-up to a follow-up acceptance criterion that explicitly says
"shutdown call-site exists at <location>." Right now AC #18 has no
identifiable code location.

---

## Confirmations against the codebase

### C1. §4 substrate-reuse table is otherwise accurate

Spot-checked:
- `persist_session_master_key` at `src-tauri/src/services/surface_session_keychain.rs:79`. Packet says line 60. Close (within section); not a blocker.
- `load_session_master_key` at `src-tauri/src/services/surface_session_keychain.rs:107`. Match.
- `delete_session_master_key` at `src-tauri/src/services/surface_session_keychain.rs:139`. Match.
- `revoke_pairing_row` at `src-tauri/src/services/surface_pairing.rs:1993`. Match.
- `mark_session_revoked` at `src-tauri/src/services/surface_pairing.rs:2042`. Match.
- `mark_pairing_expired` at `src-tauri/src/services/surface_pairing.rs:2059`. Match.
- `revoke_existing_pairing_for_site` at `src-tauri/src/services/surface_pairing.rs:1803`. Match.
- `rehydrate_sessions_from_keychain` site at `src-tauri/src/surface_runtime/mod.rs:667`. Match.
- `write_runtime_sentinel` at `src-tauri/src/surface_runtime/mod.rs:818`. Packet says `~880` — drift but flagged with "~".
- `remove_runtime_sentinel` at `src-tauri/src/surface_runtime/mod.rs:887`. Packet says 883. Drift.
- `flush_session_activity_on_shutdown` at `src-tauri/src/surface_runtime/mod.rs:753`. Match.
- `SurfaceEndpointState::stop` at `src-tauri/src/surface_runtime/mod.rs:404`. Match.
- `impl Drop for SurfaceEndpointState` at `src-tauri/src/surface_runtime/mod.rs:468`. Match.

Suggest re-running the §4 line numbers against the head sha once at V2 to
catch drift; current numbers are good enough for L0 review but will rot if
V2 takes more than a few days.

### C2. §6.4 (no read-path keychain probing) is consistent with the v1.4.2 read-only contract

Verified that `load_session_master_key` is currently invoked ONLY from
`rehydrate_sessions_from_keychain` (grep across `src-tauri/src/` returns
one non-test caller at `src-tauri/src/surface_runtime/mod.rs:667`). §12 open
question #3 to the code-reviewer asked for exactly this confirmation;
answer: confirmed, only the rehydration site exists today.

### C3. §6.2 (commit DB, then keychain delete) matches house style

See F1 — this is the same pattern as
`src-tauri/src/services/accounts.rs:3514-3548`. The packet's decision is
correct; the missing piece is naming the precedent.

### C4. Intelligence-Loop integration check (CLAUDE.md "Critical Rules") — legitimately exempt

CLAUDE.md requires every new table / claim field / user-visible
intelligence surface to answer the 5 IL questions. DOS-673/674/675 add no
table, no claim field, no user-visible intelligence surface. The work is
substrate operational hardening on existing audit + keychain + lifecycle
plumbing. Audit emission is the existing audit table.

The packet's §6.6 ("audit emission on lifecycle cleanup … no new schema")
is the correct framing. **No IL check required.** Recommend §1 or §2 add a
single line declaring "Intelligence Loop integration check: N/A —
operational hardening only; no new claim/table/surface" so the next L0
reviewer doesn't ask.

---

## Open-question dispatching (for §12 answers)

- §12 #1 (codex challenge — `Corrupt` vs `Unavailable`): Keep them split. Different remediation: `Unavailable` says "keychain layer is sick, retry"; `Corrupt` says "this specific entry is poisoned, user must re-pair." Same audit channel, different `reason` field, different downstream UX hint. Splitting is cheap; collapsing later is cheap. Default to split.
- §12 #2 (CSO — audit flood during keychain outage): See F2. Rehydration runs once per Tauri launch; the bound is N-sessions-per-launch which is fine. State this in §6.6.
- §12 #3 (code-reviewer — other callers of `load_session_master_key`): Confirmed. Only the rehydration site. (C2 above.)
- §12 #4 (codex consult — `stop_async` 2s timeout): Depends on `flush_session_activity_on_shutdown` worst case. Current impl is 2 coalesced UPDATEs on `surface_client_sessions` + `surface_client_pairings` — fast on a healthy DB but unbounded if the writer mutex is contended. 2s feels right; instrument and tune in L4. State the instrumentation expectation in AC #19.
- §12 #5 (CSO — session_id in audit cleanup events): Packet correctly notes existing audit events already reference session ids in plaintext. Match precedent — plaintext is fine, hashing creates a join problem with existing events.

---

## Closing

The packet is doing the right work in the right shape. The three blockers
are integration gaps (missing call site, missing format anchor, missing CI
infrastructure), not architectural defects. The four substantive findings
are precedent-naming and wire-up specificity. V2 fold should be a few
hours, not a re-architecture.

CONDITIONAL APPROVE. Re-review V2.
