# Packet F — Security Auditor Review — Cycle 4

**Reviewer:** security-auditor
**Packet version reviewed:** V1.3 (2026-05-19)
**Cycle:** 4 (cycle-3 verdict: APPROVE with 3 L1 advisories)
**Date:** 2026-05-19

---

## Verdict

**CONDITIONAL APPROVE**

One blocking condition before L1 authoring begins (§9 invariant #10 stale line reference). One non-blocking advisory (§16 item 7 SHOULD weakening). The four V1.3-reframe STRIDE probes are clean or advisory-only. The three §16 items are substantively addressed in packet text; item 7 has a wording gap. The invariants are structurally sound with the one citation fix.

---

## Summary

V1.3's `VerifiedNonce` extension and actor-format fix introduce no new exploitable surfaces — the three-layer `wp_user_id` enforcement and the audit redaction model are intact. The sole concrete implementation risk is §9 invariant #10 pointing a grep gate at `:1235` (`audit_event` constructor) instead of `:1158` (`NonceAuditContext` struct definition) — the same class of citation footgun the V1.3 changelog explicitly fixed for `submit_feedback()`, left unresolved in the invariant block.

---

## §16 L1 implementation notes — items 6, 7, 8

### Item 6 — HKDF `info` parameter purpose-binding

**Status: CONFIRMED PRESENT**

§16 line 670 specifies verbatim: `AUDIT_IP_HASH_KEY_INFO = b"dailyos.surface.audit.ip_hash.v1"` distinct from the existing `PRESENCE_NONCE_KEY_INFO = b"dailyos.surface.presence_nonce.digest.v1"`. The two constants differ in every byte after the shared `dailyos.surface.` prefix. The purpose-binding collision risk from the cycle-3 advisory is addressed at the constant-naming level. Confirmed.

### Item 7 — Permission callback literal reuse

**Status: WEAKENED — advisory condition**

The cycle-3 advisory stated: "L1 SHOULD use the existing `can_issue_presence_nonce` callback directly to prevent drift." §16 line 672 preserves the word "SHOULD." The cycle-3 advisory text that this captures explicitly called out that "reuses `can_issue_presence_nonce`-equivalent logic" leaves open the possibility of an L1 implementer writing a weaker new callback.

The downgrade from the cycle-3 recommendation ("tightening AC G to say 'MUST use `can_issue_presence_nonce` directly' would close this") to SHOULD in §16 is a regression from the advisory. §8.7 PHPUnit covers "permission callback rejects unauthenticated" and "requires pairing marker" — these two sub-checks are tested. The third sub-check in `can_issue_presence_nonce` at `class-dailyos-plugin.php:780-798` is `DailyOS_Credential_Store::is_paired()`. If an L1 implementer writes a parallel callback that passes the first two sub-checks but omits `is_paired()`, §8.7 as written does not catch it because §8.7 names only the first two checks.

**Recommended fix:** change "L1 SHOULD use" to "L1 MUST use" in §16 item 7, or add `is_paired()` as an explicit assertion in §8.7. Either closes the gap. This is advisory (anchor 50) — the realistic attack surface is authenticated-but-unpaired users, which the existing infra would catch at the runtime signed-route layer — but the SHOULD wording contradicts the cycle-3 advisory intent.

### Item 8 — WP `payload_json` non-plain-object rejection

**Status: CONFIRMED PRESENT**

§16 line 674 specifies: "reject arrays, deeply-nested objects, or values that aren't plain key-value maps before forwarding to runtime." This matches the cycle-3 advisory. The defense-in-depth measure is captured at the L1 implementation note level; §5.6 step 3 and §8.8 back it up with test coverage. Confirmed.

---

## V1.3 security-critical reframe probes

### 1. STRIDE-InfoDisclosure: `VerifiedNonce` extension + audit-event writer redaction

**Assessment: CLEAN**

V1.3 extends `VerifiedNonce` at `surface_nonce.rs:583` to carry `claim_id`, `action`, `payload_json`, `wp_user_id`, `session_id` out of `verify_and_consume`. The new concern: does carrying `payload_json` in `VerifiedNonce` create a path by which user-authored content reaches the audit-event writer?

The audit-event writer (`presence_nonce_verified`) fires inside `verify_and_consume` BEFORE returning the `VerifiedNonce` struct (V1.3 §5.4 + §8.10 both confirm ordering: `presence_nonce_verified` fires inside `verify_and_consume`'s Mutex-guarded HashMap mutation; the `VerifiedNonce` is returned after that fires; `record_claim_feedback` is called subsequently). The `payload_json` field in `VerifiedNonce` is therefore not in scope at audit-emit time. §5.5 explicitly states user-authored `payload_json` fields stay in the `claim_feedback` row and are "surfaced only through actor-filtered projection per ADR-0108." §9 invariant #4 gates this with a grep check on audit-event writer callers.

The V1.2 §5.5 + §9 inv #4 coverage established in cycle-3 remains intact through V1.3. No new path opened by `VerifiedNonce` extension. Clean.

### 2. STRIDE-Spoofing: `"user:wp:{wp_user_id}"` actor format

**Assessment: CLEAN**

The actor string is assembled from `verified.wp_user_id` at §5.4 pseudocode line `actor: format!("user:wp:{}", verified.wp_user_id)`. `verified.wp_user_id` comes from the `VerifiedNonce` return of `verify_and_consume`, which captures the binding's `wp_user_id` — the same field that `compare_binding_tuple` at `surface_nonce.rs:1339` cross-checked at verify time. The three-layer enforcement confirmed clean in cycle-3 (WP session derivation + signed-route validate + binding cross-check) ensures no JS-controlled path exists to inject a different `wp_user_id` into the binding. The head `"user"` passes `actor_class_for_actor`'s allowlist; the tail is always the session-bound WP user. No spoofing path identified. Clean.

### 3. STRIDE-InfoDisclosure: full `PresenceNonceRejectReason` enum in HTTP response

**Assessment: FYI — anchor 50**

V1.3 §5.6 step 1 specifies the `/dailyos/v1/nonce/verify` REST response on failure: `{ error_code, rejection_reason }` using `PresenceNonceRejectReason` variant names. V1.3 corrects the variant set to the full 17-variant enum. Variants such as `WrongSession` disclose to the caller that the nonce was issued under a different session.

The realistic attacker surface: the `/dailyos/v1/nonce/verify` endpoint requires `can_issue_presence_nonce` — authenticated and paired. Unauthenticated probing is blocked before any rejection reason is returned. Within an authenticated session, `WrongSession` tells the authenticated user their nonce crossed sessions — this is information the user can already infer from the UX flow (they would see a stale nonce state). No cross-user or cross-account session topology is disclosed beyond what the authenticated session principal already knows.

This is advisory. If a future deployment model allows multiple authenticated users to share a pairing (multi-user install), `WrongSession` would disclose inter-user session presence to a sibling user. Current model is single-user local-to-local per §3 framing. No packet change warranted.

### 4. OWASP-A04: manual §8.9 PHPUnit vs compile-time channel-sweep gate

**Assessment: FYI — anchor 50**

V1.3 §5.9 correctly acknowledges the W6-E `#[non_exhaustive]` gate was planned but never built. The fallback is §8.9 PHPUnit: a manual test asserting `WpBlockRenders` is included in the redaction sweep for WP block renders. The structural gap: if a future `RenderPolicyChannel` variant is added and its redaction path is incomplete, the §8.9 test would not catch it unless simultaneously updated. A compile-time exhaustive match (W6-E intent) would.

However, the W6-E backfill is correctly filed as a maintenance ticket. §9 invariant #11 requires `WpBlockRenders` in `ALL`. The W4 test covers the current enumeration. The residual risk is future-variant-addition omission, not a W4 implementation gap. No W4 packet change warranted; the maintenance ticket is the appropriate vehicle.

---

## §9 invariants — lockability check

| Invariant | Status | Notes |
|---|---|---|
| #4 — no raw `payload_json` in audit | LOCKED + TESTABLE | §5.5 explicit prohibition + grep gate. `VerifiedNonce` extension does not open a new path (audit fires before `VerifiedNonce` is returned). |
| #9 — WP allowlist count = 9 | LOCKED + TESTABLE | Grep gate on `:907` count + old-4-string absence check. |
| #10 — `NonceAuditContext` slots | LOCKED but **CITATION WRONG** | See finding below. |
| #12 — orphan prevention | LOCKED + TESTABLE | Grep gate on `/v1/surface/feedback` absence. |

---

## Findings

### Finding C4-1 — §9 invariant #10 stale line reference (MEDIUM — implementation risk)

**Anchor: 75**

§9 invariant #10 reads: "Grep gate on the struct definition at `surface_nonce.rs:1235`."

`:1235` is `audit_event` (the event constructor), not `NonceAuditContext` (the struct definition). The correct location, per V1.3 §2 changelog, §3 substrate table, and §5.3, is `:1158`. This is the same class of citation footgun the V1.3 changelog explicitly fixed for `submit_feedback()` (`:163` → `:149`) — that fix was applied in §5.6 step 4 and AC K, but the parallel instance in §9 invariant #10 was not corrected.

A CI grep gate targeting `:1235` will either hit the wrong structure (the `audit_event` constructor body rather than the `NonceAuditContext` struct field list) or — if the grep is pattern-matching for the slot names rather than line number — will be writing a grep against a different context than the struct definition. In either case the gate is fragile. At L1, an implementer following the invariant literally would verify the wrong location.

**Required fix:** change `surface_nonce.rs:1235` to `surface_nonce.rs:1158` in §9 invariant #10. One-word change; consistent with the V1.3 corrections already applied everywhere else in the packet.

### Finding C4-2 — §16 item 7 "SHOULD" weakens cycle-3 advisory intent (LOW — advisory)

**Anchor: 50**

As detailed in the §16 item 7 section above, the SHOULD wording in the L1 implementation note opens a narrow gap: an L1 implementer could write a parallel `can_verify_presence_nonce` callback that passes §8.7's tested sub-checks but omits `is_paired()`. The gap is bounded by the runtime signed-route layer catching unpaired callers; it is not zero-risk.

Recommended: change "SHOULD" to "MUST" in §16 item 7, or add `is_paired()` as an explicit assertion in §8.7. Either is sufficient.

---

## Lock criteria check

| Gate | Status |
|---|---|
| Cycle-3 APPROVE maintained | YES — all three cycle-3 L1 advisories are substantively in §16 |
| §16 items 6 + 8 confirmed present | YES |
| §16 item 7 wording | WEAKENED — advisory condition |
| V1.3 STRIDE probes — `VerifiedNonce` + audit redaction | CLEAN |
| V1.3 STRIDE-Spoofing — actor format | CLEAN |
| V1.3 STRIDE-InfoDisclosure — rejection reason enum | FYI anchor 50 |
| V1.3 OWASP-A04 — manual gate vs compile-time | FYI anchor 50 |
| §9 inv #4 lockable | YES |
| §9 inv #9 lockable | YES |
| §9 inv #10 lockable | CONDITIONAL — citation must be corrected to `:1158` |
| §9 inv #12 lockable | YES |
| CSO + security-auditor APPROVE gate | security-auditor: CONDITIONAL APPROVE pending C4-1 fix |

**Condition to clear:** correct `surface_nonce.rs:1235` → `surface_nonce.rs:1158` in §9 invariant #10. Single line change, verifiable from §2 V1.3 changelog. Once applied, security-auditor upgrades to APPROVE.
