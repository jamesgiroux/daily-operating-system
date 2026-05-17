# CSO review cycle 2 — L0 Packet A (Surface session lifecycle hardening) V1.1

**Verdict: APPROVE**

Reviewer: CSO mode (Claude)
Date: 2026-05-17
Packet: `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md` V1.1
Prior review: `.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cso.md` (cycle 1, CONDITIONAL APPROVE)
Source verified: `surface_session_keychain.rs`, `surface_runtime/mod.rs:660-746` @ dev sha 9a33d347.

V1.1 Path-α trim is sound for the v1.4.3 local-to-local threat model. All deferrals fall outside the same-UID single-user adversary capability surface. No new findings rise above LOW. The single MEDIUM I left on the table (audit coalesce) is genuinely scale-bound and the V1.1 rationale (1-3 sessions × 1 startup per outage) holds. Deferral classifications align with v1.4.2 W4-F V3.2 precedent.

## Per-validation answer

### 1. Same-UID threat model assumption — **CORRECT**, severity **LOW**

For v1.4.3 local-to-local, the same-UID adversary already has:
- Direct `security find-generic-password` access to every keychain entry the user has access to (code-signing ACL gates the entry to the DailyOS-signed binary, but a same-UID attacker can run the signed binary or, more directly, attach via `task_for_pid` / dtrace / `vmmap` against the running Tauri process).
- Direct read of `~/.dailyos/{db,audit.log,runtime-endpoint.json}`.
- Process-environment write (the basis of the DYLD finding).
- SQLite file write (can flip `revoked_at` themselves; do not need an orphan keychain entry).

I checked for a same-UID attack path NOT covered by direct keychain access. None found. Every "exploit" requires capability ≥ direct keychain read. The v1.4.2 threat-model commitment is internally consistent.

The only scenario where same-UID DOES NOT imply keychain access is the macOS "user explicitly revoked DailyOS keychain ACL via Keychain Access.app, then attacker runs as same UID." That's outside the v1.4.2 model (user has actively broken their own pairing) and the v1.1 `Unavailable` → leave active + log behavior gives the user the audit signal to re-pair.

### 2. Orphan keychain entry post-revocation — **REFUTE re-use; CONFIRM inert at local scope**, severity **LOW**

Walked `rehydrate_sessions_from_keychain` (`surface_runtime/mod.rs:660-746`) and every call site of `load_session_master_key`:

- `load_session_master_key` has **exactly one non-test caller**: `rehydrate_sessions_from_keychain` at `surface_runtime/mod.rs:667`.
- Rehydration's query (the `rows` collection upstream of line 666) filters `revoked_at IS NULL` — this is the invariant V1.1 §9 CI invariant #3 locks in. A revoked row is never re-read.
- No render-path or read-path call site reads the keychain (V1.1 §6.4 contract).
- `persist_session_master_key` is only called from the pairing handshake path (new session establish), never from a revoked-row context.

The orphan entry IS recoverable by `security find-generic-password` under same-UID, but per validation #1 that capability is presumed. Bytes are dead in-app; the only realistic exposure is the same-UID attacker who already has equivalent access. CRITICAL deferral is correctly classified — escalates only when keychain shares cross UID or machine boundaries (federation). No L6 escalation needed.

### 3. `Unavailable` deferral creates DoS window — **REFUTE for security; CONFIRM for operational pain**, severity **LOW**

Same-UID attacker who can intermittently deny keychain access (kill `securityd`, hold lock, exhaust descriptors) has *strictly stronger* capability than reading the keychain entry directly — they have process control. The "keep session almost-revoked" attack reduces to "I can read the master key whenever I want," which is the same-UID-presumed capability from validation #1. No incremental attack value.

The remaining concern is purely operational: a genuine `securityd` outage leaves a session "active in DB, unrenderable in practice" until the user notices the audit pattern. V1.1's rationale ("user-visible repeated audit is the user's signal to re-pair") accepts this. APPROVE; the bounded N=3 tolerance is a federation maintenance concern as classified.

### 4. `stop_async` deferral / `Drop` mid-DB-write user-data corruption — **NONE**, severity **LOW**

Walked `flush_session_activity_on_shutdown` (`surface_runtime/mod.rs:753+`). Confirmed it writes only `last_seen_at` and `last_used_at`. Per the cycle-1 trace, neither field gates security policy (revocation, scope checks, HMAC validation, ability authorization don't read them — they're admin diagnostics labeled "explicitly accepted staleness").

`Drop` aborting mid-write therefore loses at most: timestamp advancement on N sessions. No revocation state can be lost (committed at the lifecycle service call). No keychain cleanup can be lost (committed at post-transaction cleanup; the deferred completion of post-revoke cleanup is the codex-challenge CRITICAL deferred in validation #2). No audit can be lost (audit emit is synchronous against the JSONL file). No claim/provenance/trust state involved.

V1.0's graceful stop_async would have prevented two timestamp UPDATEs from being lost during an OS-quit. That's not a user-data corruption path. APPROVE the trim.

### 5. Audit hash deferral — **NONE**, severity **LOW**

`~/.dailyos/audit.log` is file-mode-default (umask) under `$HOME` — same-UID readable. Plaintext `session_id` in the file provides the same-UID attacker no escalation: they can read the running session in `~/.dailyos/db` directly, and the SQLite row already carries plaintext `session_id`. The inconsistency with `surface_runtime/mod.rs:734-735` (which hashes) is a stylistic normalization concern, correctly classified as separate maintenance.

One note for the maintenance ticket when it's authored: prefer hashing as the project-wide default for future-additions discipline (defense-in-depth if audit log ever ships off-machine). Not a v1.4.3 blocker.

### 6. `env_clear` deferral — **CORRECT**, severity **LOW**

The DYLD subversion path requires same-UID write to the running Tauri process's environment. That capability is *strictly stronger* than reading the keychain entry under same-UID: env write requires either `task_for_pid` (defeats code-signing-ACL the same way direct keychain read does) or being able to launch the Tauri process with the malicious env set (which means write to the user's launch agents / login items, which means write to `~/Library/LaunchAgents/`, which means full same-UID write — at which point direct keychain `security find-generic-password` is a one-liner).

The analysis in V1.1 §2 changelog ("strictly stronger position than direct keychain read") is correct. The CSO-cycle-1 LOW was authored with federation in mind; for v1.4.3 local-to-local, the defense provides no incremental security. Federation maintenance ticket is the right destination. APPROVE.

## Cycle-2 disposition

All 6 deferrals validated against the same-UID adversary model. None opens a v1.4.3-shipping security bug. Deferral classifications align with v1.4.2 W4-F V3.2 trim shape (same Path-α pattern, same threat-model commitment).

§9 CI invariant #3 (rehydration's `revoked_at IS NULL` filter) is the load-bearing structural gate that makes validation #2 hold post-merge. The fixture #15 (cleanup-target collected inside transaction) plus #16 (cleanup-outside-tx with 10ms writer-lock-release proof) close the cycle-1 LOW that would otherwise leave the post-revoke window unbounded.

**Verdict: APPROVE** — no further amendments required. Packet is L0-closed from the CSO panel side.

Closing note: this is the correct calibration. Cycle 1 was right to flag the items as LOW/MEDIUM; cycle 2 is right to accept the deferrals because the threat model commitment is the load-bearing constraint, not the individual capability gaps. The packet preserves "local-to-local with same-UID = trusted" without painting it on top of federation defenses that don't exist yet.
