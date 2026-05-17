# L0 Packet A — Cycle 2 code-reviewer verdict

**Date:** 2026-05-17
**Reviewer:** code-reviewer (claude domain)
**Packet revision under review:** V1.1 (cycle-1 fold)
**Path:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md`
**Prior verdict:** CONDITIONAL APPROVE (cycle 1) — 3 BLOCKERS + 4 substantive findings.

## Verdict — cycle 2

**CONDITIONAL APPROVE.** Cycle 1 BLOCKERS resolved. One new finding (F-C2-1) is a packet-accuracy fix — citation typo + mis-stated reuse precedent that needs a sentence rewrite in §5.1a + §2 changelog, no design change required. Two LOW findings filed for clarity. No remediation outside the packet text itself.

## Cycle-1 BLOCKER re-verification

### B1 — Three call sites of `revoke_pairing_row` (cycle-1 BLOCKER)

**Resolved.** §5.3 V1.1 enumerates three call sites at exactly the lines grep confirms:

```
1270:  revoke_pairing_row(tx, &row, &now, &input.reason)            // revoke_pairing
1383:  revoke_pairing_row(tx, &row, &now, "suspicious_replay")?;    // record_signed_transport_failure
1855:  revoke_pairing_row(db, &row, &format_ts(*now), reason)?;     // revoke_existing_pairing_for_site
1993:  fn revoke_pairing_row(                                       // definition
```

§7 AC #10 and §8 fixtures #10/#11/#13 cover all three. ✓

### B2 — §9 invariant #4 (`db_writer_observer`) referenced non-existent primitive (cycle-1 BLOCKER)

**Resolved.** V1.1 §9 explicitly removes invariant #4 and replaces it with §8 fixture #16 (mocked-slow keychain delete, asserts SQLite writer lock released within 10ms of DB commit). The replacement enforcement is runtime-observable and does not require a phantom `db_writer_observer` primitive. ✓

### B3 — §6 anchor cited an unmerged branch (cycle-1 BLOCKER)

**Resolved.** §1 now anchors to `.docs/plans/dos-546/v1.4.2-project/01-project-description.md` §"Threat model: local-to-local" (on `dev`, verified present in `git status`). Diagnostic anchor unchanged. ✓

## Cycle-1 substantive findings re-verification

### S1 — `KeychainBackend` test seam unspecified

**Resolved with caveat — see F-C2-1 below.** §5.1a now specifies the trait shape (`KeychainBackend` with `find`/`persist`/`delete`, `RealKeychain` default, `MockKeychain` for cfg(test), thread_local override seam). Shape is reasonable. The PROBLEM is the citation: see F-C2-1.

### S2 — `services/accounts.rs:3514-3548` reuse pattern citation

**Resolved.** Verified the cited region at `src-tauri/src/services/accounts.rs:3514-3548`: the pattern matches — transaction returns `Ok((tuple))`, caller then runs filesystem writes (`std::fs::create_dir_all`, `write_account_json`, `write_account_markdown`) outside the closure with `let _ = ...` best-effort discard. `KeychainCleanupTarget` returning from the tx and the caller invoking `delete_session_master_key` is a faithful mapping of this existing repo pattern. §4 reuse table + §5.3 prose both cite it correctly. ✓

### S3 — Line numbers re-synced

**Resolved.** Verified against current `dev`:
- `persist_session_master_key` at `surface_session_keychain.rs:79` ✓ (was :60 in V1.0)
- `load_session_master_key` at `:107` ✓
- `remove_runtime_sentinel` at `surface_runtime/mod.rs:887` ✓ (was :883 in V1.0)
- `write_runtime_sentinel` at `:818` (table says `~:880` — drift; see F-C2-3 LOW)

### S4 — Audit naming convention (`pairing.session.key_*`)

**Resolved.** §5.3 + §6.6 + §7 AC #17 all consistently use `pairing.session.key_cleaned` / `.key_cleanup_failed` / `.key_unavailable`. Matches the existing `pairing.session.key_missing` sibling at `surface_runtime/mod.rs:734-735`. ✓

## New cycle-2 findings

### F-C2-1 (MEDIUM) — `KeychainBackend` "pattern matches existing gravatar/keychain.rs" is wrong on two counts

**§5.1a** says: *"Pattern matches the existing `src-tauri/src/services/gravatar/keychain.rs` trait shape."*
**§2 V1.1 changelog** says: *"Pattern matches existing `gravatar/keychain.rs`."*

Both are inaccurate:

1. **Path is wrong.** Actual location is `src-tauri/src/gravatar/keychain.rs` (NOT under `services/`). Verified by `find src-tauri/src -name "keychain*"`:
   - `src-tauri/src/gravatar/keychain.rs`
   - `src-tauri/src/services/surface_session_keychain.rs`

2. **Shape is wrong — there is no existing trait.** `gravatar/keychain.rs` is free functions (`get_gravatar_api_key`, `save_gravatar_api_key`, `delete_gravatar_api_key`) calling `std::process::Command::new("security")` directly with a retry helper. There is no `KeychainBackend` trait, no test seam, no `Mock*` impl in the existing module. The packet's `KeychainBackend` trait is a NEW pattern, not a reuse of an existing one.

This isn't fatal — the trait is still the right design for the test seam — but the claim "matches existing pattern" misrepresents the codebase to the L0 panel. **Required edit:** drop the "pattern matches existing" wording from both §5.1a and §2 changelog; replace with honest framing ("introduce a small trait modeled on the standard adapter pattern; no prior trait-based keychain abstraction exists in the codebase"). No design change required.

### F-C2-2 (LOW) — `Drop`/`stop` already send `shutdown.send(true)` before `abort()`; explicit sentinel cleanup ordering needs clarification

Verified at `surface_runtime/mod.rs:415-419` (stop) and `:471-475` (Drop):

```rust
if endpoint.shutdown.send(true).is_err() { ... }
endpoint.abort.abort();
```

The listener task at `:328-331` removes the sentinel AFTER `run_listener` returns. The race exposed by DOS-675 is: `abort()` cancels the task before `run_listener` finishes draining the shutdown signal → sentinel cleanup at `:331` never executes. §5.4's fix (call `explicit_sentinel_cleanup()` from `stop` and `Drop` **before** `abort()`) correctly removes the race.

But §5.4 doesn't explicitly state the ordering relative to the existing `shutdown.send(true)`. Recommend §5.4 spell out: *"In `stop` and `Drop`: call `explicit_sentinel_cleanup()` BEFORE the existing `shutdown.send(true)` + `endpoint.abort.abort()` pair, so sentinel removal does not depend on the listener task draining the shutdown signal before being aborted."* One-sentence clarification.

### F-C2-3 (LOW) — §4 reuse table off by ~7 lines on one entry

`write_runtime_sentinel` listed as `surface_runtime/mod.rs:~880`; actual is `:818`. Tilde acknowledges approximation, but the drift is large enough to be wrong (~62 lines). Replace with `:818`. Other line numbers in §4 verified correct.

## Deferred items — codebase reuse audit (per cycle-2 prompt question 4)

Checked each DEFERRED item in §2 changelog / §11 NOT-owned against the existing codebase to ensure deferral does not skip a pre-existing reuse target:

- **Tauri `RunEvent::ExitRequested` hook:** `grep -rn "RunEvent" src-tauri/src/` returns ZERO matches. No existing hook to reuse. Deferral does not miss anything. ✓
- **`stop_async` graceful drain:** `grep -rn "stop_async"` returns ZERO matches. No prior art. ✓
- **Orphan keychain reconciliation outbox:** no existing outbox/sweep primitive in `surface_pairing.rs` or `surface_runtime/mod.rs`. Deferral is correct — this would be net-new substrate. ✓
- **Audit coalesce window:** no existing coalesce primitive in audit emission code paths. Deferral correct. ✓
- **`env_clear()` for `security` CLI:** neither `gravatar/keychain.rs` (`run_security_cmd` at :10-44) nor `surface_session_keychain.rs` does `env_clear()` today. Class concern; consistent deferral. ✓
- **`Unavailable` finer taxonomy:** no existing taxonomy. ✓
- **External-side-effect reconciliation contract:** the `accounts.rs:3514-3548` pattern is "best-effort discard"; no formal reconciliation contract exists. Consistent with deferral. ✓

No deferred item collides with an existing primitive the packet should reuse instead. ✓

## §7 AC ↔ §8 fixture mapping

Counted: 23 ACs, 23 fixtures.

- **DOS-673 (ACs 1-8 → fixtures 1-9):** 9 fixtures for 8 ACs because fixture #9 covers AC #8 (`KeychainBackend` trait seam) and fixtures 3-6 enumerate four `Unavailable` causes (spawn-failure, locked, corrupt-base64, wrong-length) under AC #4. Reasonable over-coverage. ✓
- **DOS-674 (ACs 9-19 → fixtures 10-19):** 10 fixtures for 11 ACs. **AC #9** ("`KeychainCleanupTarget` struct exposes `surface_client_id` + `session_ids: Vec<String>`") has no dedicated fixture — it's a compile-time struct-shape assertion implicitly proven by every fixture that constructs the struct (10, 11, 12, 13, 14, 15). Acceptable; no remediation. ACs 10-19 each map cleanly to at least one fixture (10→10/11/13, 11→15, 12→10, 13→11, 14→13, 15→14, 16→12, 17→17, 18→16/19, 19→18). ✓
- **DOS-675 (ACs 20-23 → fixtures 20-23):** 1:1. ✓

## Class-pattern check (per CLAUDE.md "same-shape findings twice")

Cycle 1's three BLOCKERS were "wave-author missed a call site" (B1), "invariant references non-existent primitive" (B2), "anchor cites unmerged branch" (B3). Cycle 2 F-C2-1 is the same shape as B1 (citation does not match codebase). One occurrence here is acceptable; flag for sweep IF a third cycle surfaces it.

## Acceptance

**CONDITIONAL APPROVE.** Required edits before merge of L0:
1. F-C2-1: rewrite §5.1a + §2 changelog citation of `gravatar/keychain.rs` (text-only edit, no design change).
2. F-C2-2: one-sentence clarification in §5.4 of cleanup ordering relative to existing `shutdown.send(true)`.
3. F-C2-3: replace `~:880` with `:818` in §4 reuse table.

None of these block re-running L0 reviewer panel cycle 3 — they're packet-text accuracy fixes that fold into V1.2 alongside any other cycle-2 reviewer findings.
