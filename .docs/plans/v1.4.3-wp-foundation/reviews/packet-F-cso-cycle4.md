# CSO Review — Packet F (Feedback Write Infrastructure) — Cycle 4

**Reviewer:** /cso
**Packet version:** V1.3
**Date:** 2026-05-19
**Cycle-3 verdict:** APPROVE — 2 L1 advisories (deployment ordering for orphan retirement; phase-3 record_claim_feedback failure not charged against rate-limit budget)

---

## Verdict

**APPROVE**

Both cycle-3 advisories are faithfully captured in §16 items 4 and 5. The three V1.3-specific security questions (VerifiedNonce payload_json widening, actor format spoofing surface, §5.9 manual-gate adequacy for redaction coverage) resolve cleanly. No new blocking conditions. CSO gate is satisfied; packet contributes to lock.

---

## Task 1 — §16 L1 Implementation Notes: Items 4 and 5

### Item 4 — Deployment ordering for orphan retirement

**ACCURATE.**

§16 item 4 reads: "If the macOS runtime build and the WP plugin build ship at different times in production, a window exists where old WP code POSTs to the dead route → silent feedback loss. For local-to-local dev (the current model) this is moot; for any future remote deployment it's a real coordination concern. L1 notes for ops."

This precisely mirrors the advisory issued in the CSO cycle-3 finding (Probe 1): the path was already dead (no Rust handler wired), so no security regression is introduced; the risk is silent feedback loss during a deployment window where the runtime update lands before the plugin update (or vice versa). The item correctly scopes the concern as moot for the current local-to-local model and flags it as an ops concern for any future remote deployment path. The framing "L1 notes for ops" is the correct disposition — not a packet condition, not a blocker.

The advisory's substance is preserved exactly: the three allowlist entries at `surface_runtime/mod.rs:1243, 4730, 4755` and the `submit_feedback()` deletion at `runtime-client.php:149` are listed in §10 commits 3+4 as the retirement scope. The §9 invariant #12 grep gate prevents reintroduction. Nothing in V1.3 weakens or undermines this advisory.

**Item 4 disposition: accurate; flags the concern correctly for ops.**

### Item 5 — Phase-3 `record_claim_feedback` failure budget charging

**ACCURATE.**

§16 item 5 reads: "Phase-3 (record_claim_feedback failure after a successful consume) is NOT currently charged. An attacker could intentionally exhaust nonces by triggering record_claim_feedback failures (e.g., via malformed payload_json that passes WP shape check but fails Rust validate_feedback_payload). L1 adds the failure charge on the phase-3 Err path."

This precisely mirrors the advisory issued in the CSO cycle-3 finding (Probe 4): `charge_failure_best_effort` at `surface_nonce.rs:273` is called inside `verify_nonce` for nonce-level rejections; it is not called for the W4-added phase-3 path where `record_claim_feedback` fails after a successful consume. The gap exists because `charge_failure_best_effort` is scoped to `verify_nonce`'s Err path, and the W4 pseudocode calls `record_claim_feedback` after `verify_and_consume` returns Ok.

The item also correctly names the file:line reference (`surface_nonce.rs:30-52` for the budget constants, `:273` for the existing call site) — the L1 implementation task is to add a `charge_failure_best_effort` call on the `Err` branch of the `record_claim_feedback` call in the handler. The item does not overstate severity: under local-to-local deployment this is an attacker exhausting their own quota; it is not a service-wide DoS vector because the budget key includes `surface_client_id`.

**Item 5 disposition: accurate; flags the L1 implementation gap at the correct location with correct severity framing.**

---

## Task 2 — V1.3 Security Posture

### Probe A: `VerifiedNonce` extension carrying `payload_json` — does this widen the leak surface?

**CLEAN.**

V1.3 §5.4 extends `VerifiedNonce` (at `surface_nonce.rs:583`) to carry `claim_id`, `action`, `payload_json`, `wp_user_id`, and `session_id` out of `verify_and_consume`. The question is whether this extension creates a new path by which `payload_json` user-authored content could escape to an inappropriate surface.

The model is consistent with the existing sensitivity=User protection:

1. **Source of the data:** `payload_json` is read from the in-memory binding (stored at issue time, never from the verify request body). This is the same store-isolation model established in V1.2 §5.2 and confirmed clean by the security-auditor cycle-3 STRIDE-Tampering probe.

2. **Only consumer of the extended `VerifiedNonce`:** the handler pseudocode at §5.4 Step 2 immediately constructs `ClaimFeedbackInput { ..., payload_json: verified.payload_json.clone() }` and passes it to `record_claim_feedback`. `record_claim_feedback` calls `validate_feedback_payload` at `claims.rs:5150` before writing. The `VerifiedNonce` struct does not escape the handler function body.

3. **Audit path — payload_json never in audit payloads:** §5.5 and §6 decision #5 explicitly assert: "Sensitivity=User content NEVER in audit event payloads — `payload_json` user-authored fields stay in the `claim_feedback` row." §9 invariant #4 names a grep gate: "audit-event writer callers in `surface_nonce.rs` must not pass `nonce.payload_json` through." The `VerifiedNonce` extension adds `payload_json` as a field, but the audit-event construction paths (`presence_nonce_issued`, `_verified`, `_rejected`) are called inside `verify_and_consume` BEFORE the `VerifiedNonce` is returned — those paths read from `NonceAuditContext` (which §5.5 explicitly excludes `payload_json` from). No new audit-side exposure vector.

4. **Projection redaction — WpBlockRenders channel:** the `WpBlockRenders` channel addition (§5.9) ensures the existing W2 DOS-477 leak-guard machinery covers WP block render outputs. §8.9 `FeedbackPayloadRedactionTest.php` is the manual coverage gate. The V1.3 correction that the W6-E `#[non_exhaustive]` gate was never built does not weaken the protection: the PHPUnit test explicitly asserts redaction for `WpBlockRenders`. The gap — that the compile-time exhaustiveness check does not exist — is filed as a maintenance backfill (correct disposition).

**No new `payload_json` leak surface introduced by the `VerifiedNonce` extension. The V1.3 model is consistent.**

### Probe B: `actor: "user:wp:{wp_user_id}"` format — spoofing surface via wp_user_id control?

**CLEAN.**

The concern is whether an attacker controlling the `wp_user_id` portion of the actor string could subvert actor attribution or pass a crafted value through `validate_feedback_actor`.

The V1.3 model closes this at three layers:

1. **wp_user_id is server-derived, not client-supplied.** §5.3 states: "WP-side: ensure the `/dailyos/v1/nonce/verify` REST handler derives `wp_user_id` from the authenticated WP session — NEVER from request body." §5.6 phase-2 call sequence confirms: "WP derives `wp_user_id` from session." The existing issue-side substrate at `class-dailyos-plugin.php:911` reads `$current_user_id = (int) get_current_user_id()` from the WP session — the verify handler is specified to follow the same pattern. The security-auditor cycle-3 OWASP-A01 probe confirmed three layers of enforcement on this constraint (WP session derivation, runtime signed-route validation at `mod.rs:1946`, and nonce binding cross-check at `surface_nonce.rs:1339`).

2. **wp_user_id is cross-checked at the binding level.** The `compare_binding_tuple` call at `surface_nonce.rs:1339` rejects if `binding.fields.wp_user_id != request.wp_user_id`. The `request.wp_user_id` at this point is the value the WP transport layer derived from the session (not from the outer HTTP request body). An attacker who supplies a mismatched `wp_user_id` at issue time would produce a binding mismatch at verify, triggering `PresenceNonceRejectReason::WrongUser`.

3. **Actor format passes the existing allowlist without any new surface.** `actor_class_for_actor` at `claims.rs:5111` splits on `:`/`/`/`@` and checks the head token against `["user", "human"]`. The `"user:wp:{wp_user_id}"` format — head = `"user"` — passes cleanly. This is a correction of the V1.2 bug (`"wp_user:{}"` would have failed; V1.3 fixes the head token). The `wp_user_id` integer value appears as the tail of the actor string; it does not affect the actor-class allowlist check. The actor string is stored as a provenance attribution in the `claim_feedback` row — any injection attempt in the `wp_user_id` value would be bounded by the `u64` integer type (the WP transport casts `get_current_user_id()` to `int`).

**The V1.3 actor format is sound. No spoofing surface introduced.**

### Probe C: §5.9 manual PHPUnit gate — redaction coverage vs auto-gate?

**ADVISORY OBSERVATION (anchor 50) — not a blocker; consistent with V1.3 disposition.**

The V1.3 correction (§5.9) drops the claim that the W6-E `#[non_exhaustive]` channel-sweep auto-gate applies — that gate was planned but never built. The packet substitutes a manual PHPUnit test (§8.9 `FeedbackPayloadRedactionTest.php`) as the coverage gate for v1.4.3.

The coverage difference is real:

- **Auto-gate** (W6-E planned, not built): compile-time exhaustiveness check across all callsites that iterate `RenderPolicyChannel::ALL`. When `WpBlockRenders` is added, any callsite that projects without handling the new channel would fail at compile time. Coverage is structural and prevents drift.
- **Manual PHPUnit test** (§8.9): explicitly asserts that the WP block-render output for a feedback projection applies the correct redactions for `payload_json` user-authored fields. Coverage is behaviorally correct for the specific test path but does not enforce structure at the type level.

The manual test is adequate for v1.4.3 because:
- `RenderPolicyChannel::ALL` has zero non-definition consumers today (confirmed in V1.3 §5.9: "zero non-definition consumers"). Adding `WpBlockRenders` to `ALL` without a consumer is safe in the current substrate state.
- §8.9 explicitly names `WpBlockRenders` and asserts the redaction. The test will catch a regression where the projection path skips the new channel.
- The W6-E backfill is filed as a maintenance ticket, not deferred indefinitely.

The gap vs the auto-gate: if between v1.4.3 and the W6-E backfill, a new consumer of `RenderPolicyChannel::ALL` is added that does not handle `WpBlockRenders`, there is no compile-time catch — only test-time coverage. This is within the accepted risk envelope for v1.4.3. The §15 lock criteria do not require the auto-gate to be built in W4.

**This advisory observation is equivalent in substance to the V1.3 packet's own disposition of the finding.** The packet is self-aware of the gap. No new finding; confirming consistency.

---

## Task 3 — §9 Invariants: Security-Critical Gates

The §9 invariant set covers the security-critical gates adequately. Spot-check against the key security contracts:

| Security contract | Invariant(s) covering it | Status |
|---|---|---|
| wp_user_id binding (server-derived, never trust client) | §6 decision #8 locked at L0; AC C; §8.4 test; invariant #7 (wp_user_id mismatch audit) | Covered. Three-layer enforcement (WP session, runtime signed-route, binding cross-check). |
| Audit payload completeness — replay attempts recorded with attempted identity | §5.5 NonceAuditContext extension; AC F; invariant #10 (struct slots); §8.10 test | Covered. |
| No PII/payload_json in audit | §6 decision #5; §5.5 explicit exclusion; invariant #4 (grep gate on audit-event callers); §8.9 test | Covered. |
| No raw payload_json in audit event payloads | invariant #4 (grep gate: audit-event writer callers in surface_nonce.rs must not pass nonce.payload_json) | Covered. |
| Orphan reintroduction prevention | invariant #12 (grep gate: /v1/surface/feedback must not reappear); AC K | Covered. |
| Replay-rejection fail-closed | §6 decision #10; invariant #6; §8.3 test | Covered. |
| PresenceNonceAction / FeedbackAction structural parity (drift prevention) | invariant #8 (compile-time exhaustive match via From impl) | Covered. |
| All mutations through services/ | invariant #1 (raw_wpdb_outside_services CI gate); invariant #3 (verify_nonce must call record_claim_feedback via the services path) | Covered. |
| record_claim_feedback body unchanged by W4 | invariant #2 (diff gate on function body + FeedbackAction enum) | Covered. |

One residual observation (anchor 50, not a new finding): §9 invariant #10 still cites `NonceAuditContext` at `surface_nonce.rs:1235`, but V1.3 corrected the authoritative citation to `:1158` (`:1235` is `audit_event`, the constructor). The grep gate text in invariant #10 points to `:1235`. If the gate is implemented as a grep on the line number, it will scan the wrong location. This is a low-severity L1 precision note — the struct name `NonceAuditContext` is in both lines so a name-based grep remains effective — but the citation should say `:1158` to match the rest of V1.3's corrections.

**Finding: Advisory (anchor 50).** Invariant #10 cites the wrong line number (`:1235` instead of `:1158`, inconsistent with V1.3's own correction in §3, §5.5, and AC F). A name-based grep is still effective; a line-number-based gate would target the wrong location. L1 implementor should use `NonceAuditContext` as the grep term, not line `:1235`.

---

## Top Findings

| # | Finding | Severity | Anchor | Disposition |
|---|---|---|---|---|
| §16 item 4 | Deployment ordering advisory — accurate, flags runtime + WP coordination concern correctly | — | — | Verified accurate |
| §16 item 5 | Phase-3 failure budget gap — accurate, names `charge_failure_best_effort` at `:273` and the L1 addition required | — | — | Verified accurate |
| V1.3 VerifiedNonce payload_json extension | No new leak surface; store-isolation model preserved; audit exclusion and WpBlockRenders channel coverage consistent | — | — | CLEAN |
| V1.3 actor format `"user:wp:{wp_user_id}"` | wp_user_id server-derived (three-layer enforcement); actor-class split is well-understood; no spoofing surface | — | — | CLEAN |
| §5.9 manual PHPUnit vs auto-gate | Coverage gap is real but within accepted risk envelope for v1.4.3; packet self-aware; W6-E backfill filed | Advisory | 50 | Consistent with V1.3 disposition |
| §9 invariant #10 line-number citation | Cites `:1235` (audit_event constructor) not `:1158` (NonceAuditContext struct def); inconsistent with V1.3's own correction in §3/§5.5/AC F; name-based grep still effective | Advisory | 50 | L1 note: use struct name as grep term, not line number |

---

## Lock Confirmation

Per §15 lock criteria:

- Cycle-3 CSO verdict: APPROVE (no conditions).
- Cycle-4 task: verify §16 items 4+5 accuracy + V1.3 security posture — all checks pass or resolve as advisory-only.
- No new blocking conditions.
- No class-pattern recurrence.
- CSO gate across all cycles: APPROVE (cycle-1 CONDITIONAL → cycle-2 CONDITIONAL → cycle-3 APPROVE → cycle-4 APPROVE).

**CSO gate: APPROVE. Contributes to lock.**
