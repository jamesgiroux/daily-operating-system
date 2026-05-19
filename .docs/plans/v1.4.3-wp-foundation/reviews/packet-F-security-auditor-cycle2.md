# Packet F — Security Auditor Review — Cycle 2

**Reviewer:** security-auditor
**Packet version reviewed:** V1.1 (2026-05-19)
**Cycle:** 2 (cycle-1 verdict: CONDITIONAL APPROVE with 5 findings)
**Date:** 2026-05-19

---

## Verdict

**CONDITIONAL APPROVE**

V1.1 closes the HIGH `wp_user_id` binding finding and the TOCTOU MEDIUM solidly. Three residual conditions require surgical AC-level fixes before the packet locks; none require a V1.2 cycle — they are verifiable from packet text and the substrate code already read.

---

## Summary

V1.1 materially improves the security posture: `wp_user_id` is confirmed bound at issue and cross-checked at verify via `compare_binding_tuple` in the existing substrate; the in-memory Tokio mutex held throughout `verify_and_consume` prevents TOCTOU; the server-reads-action_kind pseudocode in §5.4 is _conceptually_ correct (the server picks `nonce.action` from the stored binding, not an unbounded client field) but the existing substrate also requires the client to re-supply `action` on verify, and the W4 AC does not specify what happens to that redundant client field — a clarification is needed. Two new findings landed from the STRIDE/OWASP probes: (a) the WP-side `action_kind` allowlist in `presence_nonce_payload` at `class-dailyos-plugin.php:907` is hardcoded to the old 4 variants and will reject all 5 new ADR-0123 actions, breaking W4's core acceptance criterion; (b) the §5.5 audit payload extensions for `attempted_wp_user_id` / `attempted_surface_client_id` do not exist in the current substrate and the packet's invariant #4 (grep gate: "must not pass `nonce.payload_json` through") is a grep gate on call-site text, not a runtime assertion — adequate for CI but must be explicitly scoped in the AC.

---

## Cycle-1 finding disposition

### Finding 1 — `wp_user_id` binding HIGH → CLOSED

Confirmed via substrate read:

- `PresenceNonceBindingFields` at `surface_nonce.rs:485` carries `wp_user_id: u64`.
- `issue_nonce` at `surface_nonce.rs:181` populates it from `request.wp_user_id`, which `ensure_session_tuple` at `:1313` already validated against the session-bound value.
- `compare_binding_tuple` at `surface_nonce.rs:1339` rejects on mismatch (`PresenceNonceRejectReason::WrongUser`).
- `surface_runtime/mod.rs:1946–1978` cross-validates all `wp_user_id` channels (body, query, header) against the session before dispatch.

**Assessment:** HIGH finding fully closed. The binding, cross-check, and reject path are all live in the substrate. W4 inherits them.

---

### Finding 2 — TOCTOU race on consume MEDIUM → CLOSED

Confirmed via substrate read:

- `SurfaceNonceStore.inner` is a `tokio::sync::Mutex` (line 567).
- `verify_and_consume` at line 649 calls `self.inner.blocking_lock()` and holds the lock for the entire check-then-act path: binding lookup → `compare_binding_tuple` → invalidation checks → expiry check → DB version reads → `try_mark_consumed`.
- The packet's §5.4 pseudocode wraps `mark_consumed_if_unclaimed` + `record_claim_feedback` in a `with_transaction` block. Since the in-memory store (not a DB table) is the authoritative consumed-state record, the mutex covers the race window. The `with_transaction` wraps the substrate-side DB writes that follow, providing the DB-level atomicity for `record_claim_feedback`.

**Assessment:** MEDIUM finding closed. The mutex is the correct TOCTOU gate for the in-memory nonce store; the `with_transaction` block covers DB writes. No gap.

---

### Finding 3 — `user_intent_text` channel-count claim unverified MEDIUM → CLOSED WITH NOTED RESIDUAL

V1.1 §12 open question #1 correctly defers to cycle-2 to read the registry. Per W6-E L0 packet at `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:252–254`, the baseline is **9 channels** (callouts, prep outputs, MCP responses, Tauri renders, signal payloads, telemetry, eval fixtures, replay, error logs). V1.1 acknowledges this and says W4 either adds a 10th (feedback-render projection path) or documents it is covered by an existing channel. The packet does not resolve this conclusively — AC J ("payload_json user-authored fields never leak to non-originating actors") and test §8.9 (`FeedbackPayloadRedactionTest.php`) address the PHP projection layer but do not name which of the 9 channels (or a 10th) covers the feedback-write path specifically.

**Assessment:** The channel-count question is adequately bounded for a plan-level review: the packet commits to verifying coverage in §8.9 + §9 invariant #4. The implementation-time resolution (does W4 add channel 10 or classify under an existing channel?) is an L1 obligation, not a plan-lock blocker. Lowering to FYI.

---

### Finding 4 — `replay_rejected` audit payload missing attempted-actor identity MEDIUM → PARTIALLY CLOSED

§5.5 states `presence_nonce_rejected` will add `attempted_wp_user_id` + `attempted_surface_client_id` + `rejection_reason`. The packet AC F confirms this. However, the **current substrate audit event builder** (`audit_event` at `surface_nonce.rs:1235`) does not include these fields — `NonceAuditContext` has no `attempted_wp_user_id` or `attempted_surface_client_id` slots. W4 must add these, and the packet should explicitly name the struct extension as a W4 deliverable (it currently only appears in §5.5 prose). AC F says "User-authored `payload_json` content never in audit payloads" but does not name the `attempted_wp_user_id` field addition as a testable AC. §8.4 (`surface_nonce_wp_user_mismatch`) and §8.10 (audit correlation) together cover this, but neither test is explicitly scoped to assert `attempted_wp_user_id` is present in the `presence_nonce_rejected` payload.

**Assessment:** Finding is addressed by §5.5 prose + §8.4 + §8.10 together. The AC wording is imprecise — AC F should explicitly name `attempted_wp_user_id` as a testable field in the rejected event. **Surgical condition #1 (below).**

---

### Finding 5 — `action_kind` should be server-read from row not client-resupplied MEDIUM → PARTIALLY CLOSED

The §5.4 pseudocode correctly shows `let action: FeedbackAction = nonce.action.into();` — the server derives action from the stored binding. This is the right design intent.

However, the existing substrate's `VerifyNonceRequest` at `surface_nonce.rs:884` includes `action: PresenceNonceAction` parsed from the client request body (line 914), and `compare_binding_tuple` at line 1357 checks `binding.fields.action != request.action`. This means the client must also supply `action` on the verify call, and a mismatch rejects. The packet's §5.6 says the verify phase carries only `{ nonce_digest }` — but the existing substrate requires the full binding tuple on verify.

The security posture is sound: the server takes `action` from `nonce.action` (the stored binding), not from the verify request. But the current `VerifyNonceRequest.action` field is also checked against the binding. If W4 extends to 9 variants, the JS must supply the correct `action` on verify as well. The packet's §5.6 describe the verify request as only `{ nonce_digest }`, which contradicts the existing substrate shape.

This is not a security vulnerability (the stored binding value is authoritative; the client re-supply only enables binding-mismatch rejection). But the packet's claim that verify sends only `{ nonce_digest }` is incorrect per the real code, and this discrepancy could mislead L1 implementation into omitting required fields. **Surgical condition #2 (below).**

---

## New STRIDE/OWASP findings (cycle-2 probes)

---

### STRIDE-Spoofing: HKDF seed attack surface

**Assessment: FYI (anchor 50)**

The `surface_nonce_w2b_root` at `hmac.rs:131` is generated via `random_secret32()` at runtime construction. It is never persisted, never logged, and never exposed over any surface per the comment at `hmac.rs:110–118`. The HKDF-SHA256 expansion over this root with the `PRESENCE_NONCE_KEY_SALT` + `PRESENCE_NONCE_KEY_INFO` domain separators produces a 32-byte HMAC-SHA256 key. The nonce itself is a 32-byte random value; `digest = HMAC-SHA256(key, nonce_bytes)`. To forge a digest an attacker would need both the key (only in process memory, not persisted) AND the nonce bytes (also never transmitted — the client sees only the base64 of the nonce bytes, from which the server recomputes the digest). Since nonce bytes are 256 bits of system randomness, preimage attacks are infeasible.

The only realistic attack surface is process memory compromise, which is out of scope for this threat model (local-to-local runtime). No additional mitigation needed at plan level.

---

### STRIDE-Tampering: does the HKDF digest cover `payload_json`?

**Assessment: MEDIUM — Surgical condition #3 (below)**

The packet at §5.2 states: "`payload_json` HKDF-bound (so the digest covers the payload and any tamper invalidates the nonce)."

The existing substrate's digest computation at `surface_nonce.rs:421–427` computes `HMAC-SHA256(key, nonce_bytes)`. The `nonce_bytes` are 32 bytes of randomness; `payload_json` is NOT currently an input to the digest. This is expected — `payload_json` does not exist in the current substrate; W4 adds it.

The critical question for W4 is: **how exactly will `payload_json` be incorporated into the HKDF binding?** The packet asserts it will be "HKDF-bound" but the `PresenceNonceDigestKey::digest` method takes `nonce_bytes: &[u8]` and produces the HMAC of those bytes alone. If W4 adds `payload_json` to `PresenceNonceBindingFields` but does NOT change the digest computation to cover it, the assertion in §5.2 is false — an attacker who can intercept the nonce response could issue a nonce with `payload_json = None`, receive the digest, then replay with a different `payload_json` in the verify request (if the verify request echoes `payload_json` and the server uses the echoed value rather than the stored value). Per §5.4's pseudocode the server reads `payload_json` from `nonce.payload_json` (stored), not from the verify request. If that is correctly implemented, `payload_json` tamper resistance is provided by the in-memory store isolation, not by the HKDF binding. The HKDF-bound claim in §5.2 is then either imprecise (the store provides tamper resistance, not the digest) or incorrect. The AC at criterion B ("HKDF-bound (tamper invalidates the nonce)") creates an implementation obligation whose mechanism is unspecified in the packet, creating a risk that L1 satisfies the AC by relying on store isolation but documents it as "HKDF-bound" without actually incorporating `payload_json` into the digest.

The packet MUST clarify the tamper-resistance mechanism: either (a) extend `PresenceNonceDigestKey::digest` to cover `payload_json` bytes in the HMAC input, making the digest genuinely dependent on the payload, or (b) restate the AC as "tamper-resistant via in-memory store isolation — the verify path reads `payload_json` from the stored binding only" and remove the "HKDF-bound" language that implies cryptographic coverage.

---

### STRIDE-Repudiation: audit payload sufficiency for incident response

**Assessment: FYI (anchor 50)**

The §5.5 payload extensions add `wp_user_id`, `ip_hash`, `user_agent_hash`, `claim_id`, `action_kind`, and `claim_feedback_id` to the issued/verified events, plus `attempted_wp_user_id` + `attempted_surface_client_id` for rejected events. For a local-to-local DailyOS deployment where the user IS the operator, this is adequate for forensic reconstruction. No session-rotation vector, no multi-tenant trust boundary. The `ip_hash` and `user_agent_hash` add linkability metadata beyond what a strictly local runtime requires, but they do not create a privacy problem in the single-user deployment.

If DailyOS ever becomes multi-tenant, the `ip_hash` keyed at install per §5.5 may not provide sufficient cross-session linkability for incident response. This is outside the current deployment model.

No action needed at this plan level.

---

### STRIDE-InfoDisclosure: `payload_json` redaction mechanism

**Assessment: MEDIUM — see Surgical condition #3**

The packet at §5.5 states "Sensitivity=User content NEVER in audit event payloads — payload_json user-authored fields (corrected_text, corrected_to, surface, invocation_id) stay in the claim_feedback row." Invariant #4 in §9 specifies a grep gate: "audit-event writer callers in `surface_nonce.rs` must not pass `nonce.payload_json` through."

The current `audit_event` builder at `surface_nonce.rs:1235` does not accept a `payload_json` parameter and does not have access to binding fields at all — it accepts only `NonceAuditContext` (which has no `payload_json` slot). This means the current code structurally cannot leak `payload_json` into audit events. W4 will add `payload_json` to `PresenceNonceBindingFields`. As long as the `audit_event` builder signature is not extended to accept binding fields, the structural protection holds.

The grep gate in §9 invariant #4 is "callers must not pass `nonce.payload_json` through." This is a correct CI gate for the call-site check. The gap is: if a future implementer adds `payload_json` to `NonceAuditContext` (a seemingly reasonable extension), the grep gate would not catch that path unless it also covers `audit.payload_json`. The gate should be specified as: grep that `NonceAuditContext` struct does NOT have a `payload_json` field, not just that call-sites don't pass it. This is a defense-in-depth hardening, folded into Surgical condition #3.

---

### STRIDE-DoS: rate-limit budget sizing

**Assessment: FYI (anchor 50)**

Rate limits at `surface_nonce.rs:27–54`: `DEFAULT_BUDGET_PER_MINUTE = 240` for issue, verify, and failure separately. `DEFAULT_MAX_OUTSTANDING_PER_SESSION = 64`. `DEFAULT_MAX_LIVE_BINDINGS = 4096`. The budget is per `(surface_client_id, wp_user_id, claim_id, field_path, action)` tuple (full key in `NonceBudgetKey::full`). A single user issuing 240 nonces/minute across different (claim_id, action) tuples would hit the narrow budget, but the global `DEFAULT_MAX_LIVE_BINDINGS = 4096` also caps total in-flight bindings across all sessions.

For a single-user local deployment, 240 issue requests per minute is well above any realistic feedback affordance rate (user clicks). The budget is adequate. If the W4 feedback affordance adds 9 actions per claim visible in the UI, and a user stress-tests all of them, the per-tuple granularity prevents a single claim from exhausting the budget for others.

No action needed at plan level.

---

### STRIDE-EoP: non-paired WP user or stale-pairing invocation

**Assessment: FYI (anchor 50)**

`can_issue_presence_nonce` at `class-dailyos-plugin.php:780` checks: (1) `is_user_logged_in()`, (2) `current_user_can('edit_post', $post_id)` or `edit_posts`, (3) `DailyOS_Credential_Store::is_paired()`. The pairing check gates on the presence of a stored credential marker — a revoked pairing where the marker is stale-present but the runtime has the session as revoked would pass the WP check but fail at the runtime `ensure_surface_client` + `ensure_session_tuple` level (wrong actor or wrong session). So revoked-session rejection is runtime-side, not WP-side.

This is acceptable for the local-to-local threat model: if the runtime is running and paired, the pairing check is live. If the Tauri process restarted and a new session was issued, the old `presence_nonce` tokens are gone from the in-memory store (60s TTL; process restart clears all). The stale-marker attack surface is an already-accepted risk in the pairing model.

No additional hardening needed at plan level.

---

### OWASP-A01: WP REST permission callback — stale-session and scope-denied

**Assessment: HIGH — NEW FINDING**

`can_issue_presence_nonce` checks `is_paired()` via `DailyOS_Credential_Store::is_paired()`. What `is_paired()` tests is the presence of a stored marker (runtime credential), not the liveness of the runtime session. A revoked or scope-denied WP user (e.g., a user whose `edit_posts` capability was removed after the pairing credential was stored) would pass the `is_paired()` check but fail at `current_user_can('edit_posts')`. That path is correctly rejected.

However, the **bigger gap** is what W4 introduces at `class-dailyos-plugin.php:907`: the existing allowlist for `action`:

```php
if ( ! in_array( $action, [ 'correct', 'dismiss', 'corroborate', 'contradict' ], true ) ) {
    return self::nonce_payload_error( 'malformed_request', 400 );
}
```

This hardcoded 4-variant allowlist is the WP-side gate for `action_kind`. W4 extends `PresenceNonceAction` to 9 variants. If this allowlist is not updated, every one of the 5 new variants (`ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`) will be rejected at the WP layer before reaching the runtime. The V1.1 §5.6 section says "Permission callback unchanged; uses the existing `can_issue_presence_nonce`" but does not mention this allowlist which is inside the separate `presence_nonce_payload` method — distinct from the permission callback.

The packet's AC G says "Permission callback unchanged (reuses existing). `payload_json` validates per variant." AC G is silent on the `action` allowlist update. AC A requires 9 variants. These two ACs are in conflict unless `presence_nonce_payload` is updated to accept the 9-variant set.

This is a clear acceptance-criterion gap: AC A is unreachable without also updating the WP-side allowlist. This is a **HIGH finding** because without the allowlist update, the entire W4 feature is non-functional for any feedback action beyond the original 4 variants.

**Required fix:** AC G must explicitly require the WP `presence_nonce_payload` method to update the `action` allowlist from the old 4 variants to all 9. **Surgical condition #4 (below).**

---

### OWASP-A03: SQL injection and path traversal in `record_claim_feedback`

**Assessment: FYI (anchor 50)**

`record_claim_feedback` at `claims.rs:6700` uses the existing Rust `rusqlite` parameterized query substrate. `payload_json` is a `Option<String>` stored as a JSON column via parameterized binding — no string interpolation into SQL. Path traversal in the audit-event writer: the audit log writer uses structured `AuditFields` + `serde_json` serialization, not file paths derived from user input. No traversal surface identified. Rust's type system eliminates the classic C injection paths.

No action needed.

---

### OWASP-A05: `payload_json` redaction — CI grep vs. runtime assertion

**Assessment: MEDIUM — folded into Surgical condition #3**

See STRIDE-InfoDisclosure above. The grep gate is adequate as a CI enforcement mechanism for the call-site; the additional hardening is to ensure `NonceAuditContext` itself cannot be extended to carry `payload_json` without triggering the gate. Addressed in Surgical condition #3.

---

## Surgical conditions (must be resolved before packet locks)

**Condition 1** — AC F must explicitly name `attempted_wp_user_id` as a testable assertion.

> Current AC F: "Audit payload extensions land. `presence_nonce_issued` / `_verified` / `_rejected` carry `wp_user_id` + `ip_hash` + `user_agent_hash` (+ correlation-specific fields per §5.5). User-authored `payload_json` content never in audit payloads."
>
> Required addition: "The `presence_nonce_rejected` event for `wp_user_mismatch` rejection carries `attempted_wp_user_id` and `attempted_surface_client_id`. Verified by §8.4."
>
> This closes cycle-1 finding #4 at the AC level.

**Condition 2** — §5.6 verify request shape must be corrected.

> Current §5.6: "Phase 2 (verify): JS POSTs `{ nonce_digest }` to `/verify`."
>
> The existing substrate requires the full binding tuple on verify (see `VerifyNonceRequest` at `surface_nonce.rs:884`). The packet should state that verify carries the full binding tuple PLUS `nonce_digest`, consistent with the existing substrate, and that `action_kind` in the verify request is checked against the stored binding (mismatch → `MismatchedAction` reject) — the server uses the binding's `action` for the `record_claim_feedback` call, not the verify request's `action`. This is a prose correction, not a design change.

**Condition 3** — AC B must clarify the `payload_json` tamper-resistance mechanism.

> Current AC B: "Presence-nonce binding carries `payload_json: Option<String>` at issue time; HKDF-bound (tamper invalidates the nonce); forwarded to `record_claim_feedback` at verify."
>
> The "HKDF-bound" claim is ambiguous — the existing `PresenceNonceDigestKey::digest` takes only `nonce_bytes` as input, not `payload_json`. The packet must specify whether W4 extends `digest` to cover `payload_json` bytes (true cryptographic binding) or relies on in-memory store isolation (the verify path reads from `nonce.payload_json` stored at issue time, making client-side tamper ineffective without a race against the store). Either is acceptable; the mechanism must be named. Additionally, §9 invariant #4's grep gate should scope to also assert that `NonceAuditContext` does not gain a `payload_json` field.

**Condition 4** — AC G must require updating the WP `action` allowlist from 4 to 9 variants.

> Current AC G: "WP REST extension works for both phases. Permission callback unchanged (reuses existing). `payload_json` validates per variant."
>
> Must add: "`presence_nonce_payload` in `class-dailyos-plugin.php` updates the `action` allowlist (currently `['correct', 'dismiss', 'corroborate', 'contradict']` at line 907) to the 9-variant set matching `PresenceNonceAction` in V1.1 §5.1. PHPUnit §8.8 asserts all 9 are accepted and unknown values are rejected."
>
> Without this fix, AC A is unreachable. This is the most critical of the four conditions.

---

## Top-3 findings summary

| # | Finding | Severity | Anchor |
|---|---|---|---|
| 1 | WP-side `action` allowlist at `class-dailyos-plugin.php:907` is hardcoded to 4 old variants; all 5 new ADR-0123 actions will be rejected before reaching the runtime, making AC A unreachable | HIGH | 100 |
| 2 | AC B's "HKDF-bound" claim for `payload_json` is ambiguous — `PresenceNonceDigestKey::digest` only covers `nonce_bytes`; tamper-resistance mechanism for `payload_json` is unspecified, creating L1 implementation risk | MEDIUM | 75 |
| 3 | AC F and AC G do not explicitly name `attempted_wp_user_id` / `attempted_surface_client_id` as testable assertions on `presence_nonce_rejected` events; §8.4 covers the rejection but not the payload content | MEDIUM | 75 |

---

## Cycle-1 recall: findings disposition table

| Cycle-1 finding | Cycle-2 disposition |
|---|---|
| `wp_user_id` binding HIGH | CLOSED — substrate confirms binding + cross-check + reject path |
| TOCTOU race on consume MEDIUM | CLOSED — `blocking_lock()` held throughout `verify_and_consume` |
| `user_intent_text` channel-count unverified MEDIUM | FYI — packet defers to L1; AC J + §8.9 bound it adequately |
| `replay_rejected` audit payload missing actor identity MEDIUM | SURGICAL CONDITION #1 — AC F must name `attempted_wp_user_id` |
| `action_kind` client-resupplied MEDIUM | SURGICAL CONDITION #2 — prose correction; security posture sound |

---

## Lock criteria

Packet locks when conditions 1–4 are incorporated into packet text (AC A, AC B, AC F, AC G). All four are verifiable from packet text without requiring a new substrate grep. Cycle-3 not required — conditions are prose/AC fixes, not design changes.

