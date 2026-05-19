# CSO Review — Packet F (Feedback Write Infrastructure) — Cycle 2

**Reviewer:** /cso  
**Packet version:** V1.1  
**Date:** 2026-05-19  
**Cycle-1 verdict:** CONDITIONAL APPROVE (5 findings)

---

## Verdict

**CONDITIONAL APPROVE — 1 HIGH surgical condition + 2 MEDIUM notes**

V1.1 resolves all 5 cycle-1 findings at the framing and specification level. The design is sound. One implementation-critical gap was surfaced by codebase verification that the packet text does not acknowledge: the existing `surface_nonce_verify_response` handler at `surface_runtime/mod.rs:2571` uses `db_read`, and `record_claim_feedback` is a write operation. W4 MUST change this call site to `db_write` — failure to do so will either silently fail or panic at the `MutationGuard::reserve` step. This is surgical and non-architectural; it does not require a new review cycle. The two MEDIUM observations are carry-forwards from new cycle-2 probes; both are advisories, not blockers.

---

## Cycle-1 Finding Resolution

### 1. §1 Framing (was: BLOCK-equivalent)

RESOLVED. §1 now reads: "The first **WP surface** write path — but **NOT** the first write path overall; the existing `check_mutation_allowed` chokepoint at `claims.rs:6700` and the existing `MutationGuard::reserve` at `claims.rs:6714` already cover mutation discipline." This is the correct framing. Confirmed at packet lines 17–18.

### 2. `wp_user_id` canonical locus (was: HIGH)

RESOLVED. §5.3 specifies:  
- `wp_user_id` stored in `PresenceNonceBindingFields` at issue (confirmed live at `surface_nonce.rs:485`).  
- `verify_nonce` rejects on mismatch with `wp_user_mismatch` (confirmed live at `surface_nonce.rs:1339–1343` via `compare_binding_tuple`).  
- Server-derived from WP session; never trusts JS-supplied value (confirmed live: `ensure_session_tuple` at `surface_nonce.rs:1301–1319` checks `session.wp_user_id != Some(request_wp_user_id)`).  
- K-in citation to `surface_runtime/mod.rs:1946` verified: this is the `signed_route_response` function which calls `validate_session_bound_wp_user_id_for_request` across body + query + headers before dispatch. The packet's citation is accurate.  

### 3. Audit payload extensions (was: MEDIUM)

RESOLVED. §5.5 specifies all required payload extensions per the cycle-1 finding:  
- `presence_nonce_issued` adds `wp_user_id`, `ip_hash`, `user_agent_hash`, `action_kind`.  
- `presence_nonce_verified` adds same + `claim_id`, `action_kind`, `claim_feedback_id`.  
- `presence_nonce_rejected` adds `attempted_wp_user_id`, `attempted_surface_client_id`, `rejection_reason`.  
- User-authored `payload_json` content explicitly excluded from all audit payloads (§5.5 final paragraph + §9 invariant 4 + CI gate).  

### 4. Channel-count baseline (was: MEDIUM)

RESOLVED AT SPECIFICATION LEVEL. §12 open question #1 correctly identifies the §5.5 open question and defers resolution to cycle-2 verification. This review performed that verification: the W6-E L0 packet at `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:252–254` confirms the baseline is the **9-channel ADR-0108 sweep** (callouts, prep outputs, MCP responses, Tauri renders, signal payloads, telemetry, eval fixtures, replay, error logs). The feedback-render projection path from W4 is not currently enumerated as a named channel in that list. The packet correctly states W4 either adds a 10th channel or documents that feedback-render is covered by an existing channel. The V1.1 AC J and §8.9 PHPUnit cover this at the test level; the formal channel-list update must occur at L1 implementation. This is an implementation obligation, not a V1.1 text gap.

### 5. §5.4 audit ordering (was: MEDIUM)

RESOLVED. §5.4 specifies:  
- `presence_nonce_verified` emits AFTER the transaction commits (post-tx).  
- `presence_nonce_rejected` emits BEFORE returning the error.  
- Race rejection uses `UPDATE WHERE consumed_at IS NULL` with `rows_affected` check.  
This is confirmed in the pseudocode at packet lines 228–238; `emit_audit` is called after `with_transaction(db, ...)` returns `Ok`.

---

## Cycle-2 Probes — New Findings

### Finding 1 — HIGH: `surface_nonce_verify_response` uses `db_read`; W4 wire-through requires `db_write`

**Evidence:** `src-tauri/src/surface_runtime/mod.rs:2570–2578`. The current handler body:

```rust
let result = app_state
    .db_read(move |db| {
        ...
        Ok(service.verify_nonce(&ctx, db, &session_for_work, payload, &request_id_for_work))
    })
    .await;
```

`db_read` routes to `svc.reader().call(...)` (a read connection). `record_claim_feedback` at `claims.rs:6714` calls `MutationGuard::reserve(db, ...)`, which performs a write. A read connection will either panic, return a `SQLITE_READONLY` error, or silently no-op depending on the connection type — none of which is acceptable.

The packet's §5.4 pseudocode is correct in semantics but does not call out that the call site in `mod.rs` must change from `db_read` to `db_write`. The §10 PR shape (commit 3: "verify_nonce → record_claim_feedback wire-through at `surface_runtime/mod.rs:2019`") implicitly covers this, but the packet text does not name `db_write` explicitly.

**Required:** V1.1 must add a sentence to §5.4 and to commit 3 of §10 specifying that the handler is changed from `db_read` to `db_write`. No new review cycle needed — surgical text addition only.

### Finding 2 — MEDIUM (advisory): HKDF seed is ephemeral per process restart; key-management story is absent from the packet but is benign by design

**Evidence:** `src-tauri/src/surface_runtime/hmac.rs:131`: `surface_nonce_w2b_root: SecretBytes32(random_secret32())`. The comment at lines 110–118 states: "Pinned for the runtime lifetime; rotates only on full process restart, which discards all live nonce bindings (no live-rotation window). This root is never persisted, logged, or returned over any surface."

The packet §6 decision #9 (no migrations, no persistence beyond 60s TTL) and §6 decision #3 (two-phase; 60s TTL) are consistent with an ephemeral key. There is no rotation attack surface because all in-flight nonces are invalidated on restart anyway.

The `ip_hash` field specified in §5.5 audit payloads ("HMAC keyed at install") does NOT yet exist in the codebase — the current `SurfaceNonceAuditEvent` struct at `surface_nonce.rs:1136–1143` carries `wp_user_id`, `wp_user_hash`, and `detail`, but no `ip_hash` or `user_agent_hash`. The packet describes these as W4 additions, which is correct, but the phrase "HMAC keyed at install" in the table implies a stable per-install key distinct from the ephemeral HKDF root. That key needs a provisioning story (keychain? derived from the pairing key? random at install and stored?). This is an implementation decision that must be made at L1 before the audit field is wired.

**Advisory:** Add a sentence to §5.5 or §6 clarifying where the `ip_hash` HMAC key lives (proposed: derive from the existing pairing root key in the keychain, not a new independent key). Not a V1.1 rewrite — a one-sentence decision.

### Finding 3 — MEDIUM (advisory): No prompt-injection path from `payload_json` into LLM context in the current targeted-repair flow

**Evidence reviewed:**  
- `validate_feedback_payload` at `claims.rs:5149–5178` parses and validates JSON shape but does not log user-authored text.  
- `targeted_repair_enqueue_invalidation` at `claims.rs:7308–7376` puts `claim_id`, `feedback_id`, and `repair_action` into the invalidation job payload — not raw `corrected_text`.  
- `targeted_repair_policy_repair_coalescing_surface` at `claims.rs:7460–7478` reads `surface` from `SurfaceInappropriate` payload but passes it through `normalize_claim_surface` which maps to a `ClaimDismissalSurface` enum (closed allowlist), not forwarded raw.  
- The `NeedsNuance` `corrected_text` field: how it flows into the targeted-repair LLM prompt is not traceable from the packet or from the current `record_claim_feedback` body (lines 6700–6878 reviewed). The `corrected_text` is stored in the `claim_feedback` row but the packet does not specify whether the targeted-repair ability reads it back and injects it into the repair prompt.

**Advisory:** The packet should document (or add an AC asserting) that `corrected_text` from `NeedsNuance` flows into the repair prompt only through the existing sensitivity-filtered `build_intelligence_context` path, not as a raw string interpolation. If the repair ability currently does not consume `corrected_text`, document that as the intended behavior. This is a low-probability injection path but the `NeedsNuance` case is the one where user text is intentionally provided to influence the AI output, making it the highest-risk variant.

---

## Supply-Chain / Secrets Probe (cycle-2 mandate)

**HKDF derivation keyed against stable seed:** The HKDF salt and info constants (`PRESENCE_NONCE_KEY_SALT`, `PRESENCE_NONCE_KEY_INFO`) are hardcoded at `surface_nonce.rs:31–32`. The root entropy is the ephemeral `surface_nonce_w2b_root` (random per process, never persisted). This is sound: domain separation is correct, the derivation is per `ring::hkdf` (HKDF-SHA256), and the key material is zeroized after derivation (`secret_material.fill(0)` at `surface_nonce.rs:94`). No rotation gap — nonces don't survive restart.

**Secrets archaeology — `ip_hash` HMAC key:** As noted in Finding 2, this key does not exist yet. It needs a provisioning story before W4 lands. Derived-from-pairing-root is the cleanest option because it reuses existing keychain infrastructure.

**No hardcoded secret material in tests:** Confirmed. Tests use `[7_u8; 32]` as test vectors (e.g., `mod.rs:4044`), which is clearly not production material and is acceptable for test fixtures. The CLAUDE.md rule on no customer-specific data in fixtures is satisfied.

---

## §12 Open Question Resolution

**Question #1 (channel-count baseline):** See Finding resolution §4 above. The 9-channel baseline is confirmed. Feedback-render is not currently a named channel. The implementation must either enumerate it as channel 10 or cite the existing channel that covers it (most likely "Tauri renders" covers the WP block render by analogy, but this needs explicit documentation at L1). This is not a V1.1 text gap — it is an L1 obligation.

**Question #2 (extend vs. mapping shim):** Extend to 9 confirmed as the correct call. The mapping shim alternative adds indirection without benefit; the `From<PresenceNonceAction> for FeedbackAction` compile-time exhaustive match enforces the 1:1 correspondence. No dissent.

---

## Top-3 Findings (severity order)

1. **HIGH — `db_read` → `db_write` at `mod.rs:2570`:** W4 wires `record_claim_feedback` into the verify handler, which currently runs on a read connection. This will break at runtime. Packet §5.4 and §10 commit 3 must name the `db_write` change explicitly. Surgical; no new review cycle.

2. **MEDIUM (advisory) — `ip_hash` HMAC key has no provisioning story:** §5.5 specifies `ip_hash (HMAC keyed at install)` but the key does not exist in the codebase and the packet does not specify its origin or storage. An implementation decision must be made at L1; suggest deriving from the pairing root in the existing keychain.

3. **MEDIUM (advisory) — `NeedsNuance` `corrected_text` LLM injection path unspecified:** `corrected_text` is user-authored free text stored in `claim_feedback.payload_json`. The packet does not assert how (or whether) the targeted-repair ability reads it back into the LLM prompt. If it does, the sensitivity=User gate must apply. If it does not yet, document that explicitly in AC.

---

## Conditions for Lock

**Blocking (must fold into V1.1 text before L0 locks):**

- §5.4 and §10 commit 3 must add one sentence: the handler at `src-tauri/src/surface_runtime/mod.rs:2570` changes from `db_read` to `db_write` for the wire-through.

**Advisory (implementation obligations, not V1.1 text gates):**

- L1: Specify `ip_hash` HMAC key provisioning source (§5.5 one-sentence addition or §6 new decision #11).
- L1: Assert or document `corrected_text` → repair prompt flow (AC addition or §13 lineage note).

