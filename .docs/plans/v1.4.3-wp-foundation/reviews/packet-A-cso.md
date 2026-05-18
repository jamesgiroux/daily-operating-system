# CSO review — L0 Packet A (Surface session lifecycle hardening)

**Verdict: CONDITIONAL APPROVE**

Reviewer: CSO mode (Claude)
Date: 2026-05-17
Packet: `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md` V1.0
Source SHAs read: `surface_session_keychain.rs` @ working tree, `surface_runtime/mod.rs` @ working tree, `surface_pairing.rs` @ working tree.
Threat-model anchor: v1.4.2 project description + `stabilization-investigation.md` §"Threat Model Constraint". The W4-F L0 packet referenced in the prompt is not present in this checkout (`docs/v143-l0-packets` branch); framing reconstructed from the project description and the investigation doc.

Three concerns force conditional approval. None block the substrate work; two require packet text amendments (a defined acceptance criterion for §5.1 fallback semantics and a §6.2 invariant statement). One open-question answer is folded in below as a recommended default.

## Summary table

| # | Concern | Severity | Disposition |
|---|---|---|---|
| 1 | `Unavailable` weakens revocation under privileged-local DoS | **LOW** | Approved as drafted; add §7 AC #23 covering the "Unavailable since startup" reconciliation contract. |
| 2 | Cleanup-outside-transaction leaves a usable signing secret window | **LOW** | Approved as drafted; verified no read-path reads keychain after DB revocation. Add §9 CI invariant #7 to keep it that way. |
| 3 | `Drop`-without-async-flush leaks audit-relevant state | **LOW** | Approved as drafted; add §7 AC #24 naming what `Drop` is explicitly allowed to lose. |
| 4 | Audit flood from `Unavailable` (open question 2) | **MEDIUM** | Recommend coalescing window of **60 seconds per (surface_client_id, session_id, reason)**, plus a one-shot "outage cleared" event on first subsequent `Found`. |
| 5 | Plaintext session ids in audit (open question 5) | **LOW** | Existing audit code already hashes session_id / surface_client_id via `stable_hash_for_audit`. Reject plaintext for the new events; require hash-only. Codified as §7 AC #25. |
| 6 | Capability-scope sweep (privilege/env/path/error messages) | **LOW** | One finding worth fixing: `Command::output()` inherits the parent process environment. Specify `Command::env_clear()` + minimal allowlist for `HOME` / `USER`. Otherwise clean. |

## Concern 1 — `Unavailable` and revocation under DoS

**Verdict: LOW — approve as drafted, with one acceptance-criterion addition.**

Attack model considered: a same-UID local attacker who can deny keychain access (kill `securityd`, hold the lock, intercept the user's keychain access prompt, exhaust descriptor table). Under the v1.0 design, `Unavailable` returns "leave DB row active." If the runtime then restarts, the in-memory `signed_transport` state is empty and the session was never re-registered — the user's revoke action wouldn't be the path being attacked. The attacker would have to keep `Unavailable` firing AND prevent the user from ever issuing a revoke through the WP admin AND prevent the user from noticing the audit diagnostic. That's not a meaningful escalation path under the v1.4.2 local-to-local threat model.

The competing failure mode — transient keychain hiccup (laptop sleep/wake, `securityd` restart on macOS update) — is the actual operational pain v1.4.2 W4-F stabilized for. The Tauri runtime is the keychain client; the attacker is co-located with the same trust boundary; same-UID processes already have ambient access. Hardening this further would be remote-shaped defense.

**Required amendment to packet §7 (Acceptance criteria for DOS-673):**

> 23. If `load_session_master_key` returns `Unavailable` for the SAME `(surface_client_id, session_id)` on N consecutive startups (N=3 as default), the rehydration path SHALL revoke the row with `revoked_reason = 'keychain_unavailable_persistent'` and emit a distinct audit event. This bounds the "leave active forever" window — a genuinely unrecoverable keychain entry transitions to a user-visible repair state instead of accumulating zombie DB rows.

State tracked in the DB (a small counter column on `surface_client_sessions`, or a side table). This is the floor: it converts the open-ended "leave active" into a bounded-tolerance contract.

## Concern 2 — Cleanup outside the transaction

**Verdict: LOW — approve as drafted; verified empirically.**

The packet's §6.2 decision is correct. I verified the window concern: at T=0 (DB commit), T=1ms (process crash before keychain delete), T=10s (next startup), the DB says revoked. Question: does any code path re-read the keychain entry and resurrect it?

Answer: **No.** `load_session_master_key` has exactly one caller outside its own test module: `rehydrate_sessions_from_keychain` at `surface_runtime/mod.rs:667`. That function reads from `surface_client_sessions` WHERE `revoked_at IS NULL` (i.e., only non-revoked rows participate in rehydration — confirmed by the rehydration query shape — the revoked row from the previous session is excluded). So the orphaned keychain entry has no in-memory consumer. It is dead bytes until the next reconciliation pass or until `delete_session_master_key` is called with a stable cleanup target stored elsewhere.

One small risk worth naming: the orphan keychain entry IS recoverable by a same-UID attacker via `security find-generic-password`, but that's true the moment it's persisted at all. The session-revoke event doesn't change that boundary; what protects the master key is code-signing-bound ACL (already enforced per the module docstring) plus single-app-keychain-access. The 1ms-to-10s window is not a meaningful escalation.

**Required addition to packet §9 (CI invariants):**

> 7. `rehydrate_sessions_from_keychain` SHALL only consider rows WHERE `revoked_at IS NULL`. Grep gate on the rehydration query; CI fails if `revoked_at IS NULL` clause is removed. This is the invariant that makes "delete outside the transaction is safe even on crash" hold.

Without that gate, a future refactor that reconsiders revoked rows for any reason would silently re-promote the orphan keychain entry to active state.

## Concern 3 — `Drop`-without-async-flush leaks audit-relevant state

**Verdict: LOW — approve as drafted; one AC addition for clarity.**

`flush_session_activity_on_shutdown` writes `last_seen_at` and `last_used_at` — described in code comments as "explicitly accepted staleness" and "approximate admin diagnostics." Neither field gates security policy: revocation, scope checks, and HMAC validation do not read either column. The audit log itself is file-based JSONL (`emit_pairing_audit_event` → `audit_log::emit_surface_audit`), NOT held in the listener task — audit events written during the lifetime of the runtime are already on disk before the listener task is aborted. So `Drop` doesn't lose audit; it loses two activity-stamp UPDATEs.

There is no per-session audit drain pending at shutdown. The audit log writer is synchronous against the JSONL file at emit time, not buffered.

**Required addition to packet §7 (Acceptance criteria for DOS-675):**

> 25. `Drop` is documented as explicitly permitted to lose: `last_seen_at` advancement, `last_used_at` advancement, and any in-flight non-graceful shutdown writes. `Drop` is NOT permitted to lose: audit events (already on disk), revocation state (committed at the lifecycle service call), keychain cleanup (committed at the post-transaction cleanup call). This becomes a documented contract that future audit/state additions must explicitly opt out of `Drop` survival.

Reviewer note: any future work that adds new shutdown-time DB writes (e.g., flushing a rate-limit counter, snapshotting cache stats) MUST be evaluated against this contract before adding to the post-loop block.

## Concern 4 — Audit flood from `Unavailable` (open question 2)

**Verdict: MEDIUM — specific recommendation.**

A sustained macOS keychain outage (`securityd` crashed and not restarted) during startup over N sessions would emit one audit event per `(session_id, surface_client_id)` pair per startup. With 5–20 active sessions and a flapping daemon, this could be a few-hundred-event-per-minute floor under sustained outage. Not a security risk; an audit-log hygiene risk that makes real findings harder to spot.

**Recommend:**

- **Coalesce window: 60 seconds per `(surface_client_id, session_id, reason)`**. Within the window, suppress duplicate audit events; increment a counter in memory; emit a single `keychain_lookup_unavailable` event at the end of the window with `occurrence_count`.
- **One-shot "outage cleared" event** when the next `Found` lookup succeeds for a `(surface_client_id, session_id)` that previously fired `Unavailable`. This gives the operator a closing event so post-mortem can bound the outage window.
- **Hard floor: still emit the first event immediately.** Coalescing is for the storm, not for the leading edge — first-event-must-fire ensures operators see the issue at occurrence time.

The 60s window is short enough that operational paging on the leading event reaches a human before the closing event; long enough that a flapping `securityd` produces one event per minute per session rather than thousands.

Justification for 60s specifically: it matches the cadence at which a human operator can act on a paging signal, and it doesn't aggregate across "session" boundaries — each new session restart re-arms the window so the operator sees one event per session per minute, not one event for the whole runtime per minute.

**Required amendment to §7 AC for DOS-673:**

> 26. Audit event for `Unavailable` lookup SHALL coalesce on `(surface_client_id, session_id, reason)` with a 60-second window. First event in the window fires immediately. Subsequent events within the window are suppressed and counted; a coalesce-summary event fires at window close with `occurrence_count`. First subsequent `Found` for a previously-Unavailable pair emits `keychain_lookup_recovered`.

## Concern 5 — Plaintext session ids in audit events (open question 5)

**Verdict: LOW — reject the precedent claim. Existing audit code already hashes.**

The packet asks: "Today's audit events already reference session ids in plaintext, so the precedent says yes — please confirm."

I read `surface_pairing.rs` and `surface_runtime/mod.rs:734-735`. The audit emission for `pairing.session.key_missing` already uses `stable_hash_for_audit` for both `session_id` and `surface_client_id`. Other audit events in `surface_pairing.rs` (`pairing_revoked_audit_event`, `pairing_code_failure_audit_event`, the audit_event in `complete_pairing`) consistently use `stable_hash(domain, value)` for session-attributable identifiers. **The precedent is hash, not plaintext.**

**Required amendment to §7 AC for DOS-674:**

> 27. Audit events for keychain lifecycle cleanup (`session_revoked`, `pairing_expired`, `pairing_replaced`) SHALL emit `session_id_hash` and `surface_client_id_hash` via `stable_hash_for_audit`, not plaintext. Matches existing precedent at `surface_runtime/mod.rs:734-735`. CI invariant: grep gate on new audit events for raw `session_id` field names in the JSON detail body.

## Concern 6 — Capability-scope sweep

**Verdict: LOW — one fix required, rest is clean.**

- **`security` CLI privilege escalation:** No. The `security` CLI runs as the current UID with the current user's keychain. No setuid, no sudo. Same-UID processes already have ambient keychain access. The CLI invocation does not escalate.
- **`Command::output()` environment leak:** This is the one finding. The current code at `surface_session_keychain.rs:51` calls `std::process::Command::new("security").args(args).output()` and inherits the full parent environment. A malicious env var (`DYLD_INSERT_LIBRARIES`, `SECURITYAGENTPLUGIN_DIR`, `TMPDIR` redirected to attacker-writable path) could subvert the `security` CLI's behavior even if the binary itself is trusted. The attacker model here would be a separate compromise that injects env vars into the Tauri runtime — narrow but a known macOS technique. **Recommend:** `Command::new("security").env_clear().env("HOME", env::var_os("HOME").unwrap_or_default()).env("USER", env::var_os("USER").unwrap_or_default()).args(args).output()`. Minimal allowlist. **Required amendment to §7:** new AC #28 "All `security` CLI invocations call `env_clear()` and explicitly pass only `HOME` and `USER`."
- **Path injection in `service_name`:** No. `surface_client_id` is `sc_<UUID-simple>` (lowercase hex only, no shell metachars). `session_id` is `sess_<UUID-simple>`. Format string `"{SERVICE_PREFIX}.{surface_client_id}"` is shell-safe and keychain-service-name-safe. CLI uses argv (not shell), so even if these strings contained metacharacters there would be no shell expansion. Path injection is not viable.
- **Insecure error messages disclosing keychain internals:** The `reason: String` field of `Unavailable` and `Corrupt` carries `security` stderr. macOS keychain stderr can include local path information (`securityd` socket paths, app bundle identifiers). For local-to-local threat model this is not a disclosure (the attacker is co-located), but for forensic hygiene I'd recommend the audit `reason` be a **classified string** (one of `spawn_failure`, `keychain_locked`, `permission_denied`, `daemon_unreachable`, `unknown_error`) and the raw stderr be `log::warn!`-only (not in the audit event). This keeps audit greppable and bounded. **Recommend** (not required) — packet §7 AC #29: "`SessionKeyLookup::Unavailable.reason` in audit events SHALL be one of the named classification strings; raw stderr goes to log only."

## L0 closure recommendation

Approve the packet for implementation **after the following amendments are folded into V1.1**:

- §7 add AC #23 (persistent-Unavailable bounded tolerance)
- §7 add AC #24 (Drop survival contract — what's permitted to lose)
- §7 add AC #25 (audit hashing — confirm not plaintext)
- §7 add AC #26 (Unavailable coalesce + recovered event)
- §7 add AC #27 (cleanup audit events hash session_id/surface_client_id)
- §7 add AC #28 (`env_clear()` on security CLI)
- §7 add AC #29 (recommended — classified reason in audit)
- §9 add CI invariant #7 (rehydration query SHALL filter `revoked_at IS NULL`)

None of these expand scope. All are tightening the contract the packet already commits to. The substrate work and primitive choices are sound under the v1.4.2 local-to-local threat model.

**Cycle gate:** if these 8 amendments land in V1.1, CSO returns APPROVE without a second review cycle. If the implementing agent disagrees with any specific recommendation (especially the 60s coalesce window or the env_clear allowlist scope), surface as cycle-2 dissent — I'll re-review the specific item.

Closing note: this is exactly the right shape of stabilization packet for a local-to-local trust boundary. The temptation in any keychain-touching fix is to over-defend against remote attackers; this packet correctly preserves long-lived local sessions and treats keychain failures as operational signal, not adversarial evidence. The amendments above are calibration, not redirection.
