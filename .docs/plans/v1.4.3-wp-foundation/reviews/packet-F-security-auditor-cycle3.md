# Packet F — Security Auditor Review — Cycle 3

**Reviewer:** security-auditor
**Packet version reviewed:** V1.2 (2026-05-19)
**Cycle:** 3 (cycle-2 verdict: CONDITIONAL APPROVE with 4 surgical conditions)
**Date:** 2026-05-19

---

## Verdict

**APPROVE**

All three cycle-2 findings are folded in V1.2. The six net-new STRIDE/OWASP probes produce one advisory observation (anchor 50, not a blocker) and five clean checks. The packet is ready to lock.

---

## Summary

V1.2 correctly closes the cycle-2 HIGH (WP allowlist), the HKDF-binding ambiguity MEDIUM (restated as in-memory store isolation), and the NonceAuditContext slot MEDIUM (slots added by name in §5.5, §9 invariant #10, AC F). The net-new STRIDE/OWASP probes surface one non-blocking observation: the HKDF `ip_hash` derivation helper specified in §5.5 names a purpose label ("ip_hash") but the plan does not confirm that the HKDF `info` parameter also carries a distinct label separate from the existing `PRESENCE_NONCE_KEY_INFO` domain — a cross-purpose key reuse risk if the helper is implemented carelessly. This is advisory at anchor 50 and is addressed by a one-line L1 implementation note, not a packet change.

---

## Cycle-2 finding verification

### Finding 1 — WP allowlist update at `class-dailyos-plugin.php:907` (cycle-2 HIGH) — CONFIRMED FOLDED

Verified from V1.2 text:

- §5.6 step 2 explicitly states: "Replace the hardcoded 4-variant array with the 9 `FeedbackAction` variants (`confirm_current`, `mark_outdated`, `mark_false`, `wrong_subject`, `wrong_source`, `cannot_verify`, `needs_nuance`, `surface_inappropriate`, `not_relevant_here`). Mirrors the runtime-side `PresenceNonceAction::parse` after the §5.1 extension."
- AC G includes: "WP-side `action` allowlist at `class-dailyos-plugin.php:907` updated to 9 variants."
- §9 invariant #9: "WP-side `action` allowlist at `class-dailyos-plugin.php:907` MUST contain all 9 variant names. Grep gate: count entries; fail if ≠ 9 OR if any of the old 4 strings (`correct`, `dismiss`, `corroborate`, `contradict`) appear. Verified by §8.8."

The current code at `:907` reads `['correct', 'dismiss', 'corroborate', 'contradict']` — this is the pre-W4 substrate state. The packet correctly identifies it and specifies the fix with sufficient precision for L1. Finding closed.

### Finding 2 — HKDF binding restated as in-memory store isolation (cycle-2 MEDIUM) — CONFIRMED FOLDED

Verified from V1.2 text:

- §5.2: "tamper-resistant via in-memory store isolation (the verify path reads `payload_json` from the stored binding, NEVER from the verify request body — the JS phase-2 call only supplies `nonce_digest`)"
- AC B: "Presence-nonce binding carries `payload_json: Option<String>` at issue time; **tamper-resistant via in-memory store isolation** (verify reads `payload_json` from the stored binding, NEVER from request body); forwarded to `record_claim_feedback` at verify."
- §5.4 pseudocode confirms: `let action: FeedbackAction = binding.action.into();` and `payload_json: binding.payload_json.clone()` — both read from `binding`, not from the request.

Substrate read confirmed: `PresenceNonceDigestKey::digest` at `surface_nonce.rs:421` takes only `nonce_bytes: &[u8]`; `payload_json` is not and will not be an HMAC input. The store-isolation model is accurate. "HKDF-bound" language is absent from V1.2 AC B. Finding closed.

### Finding 3 — NonceAuditContext slot additions (cycle-2 MEDIUM) — CONFIRMED FOLDED

Verified from V1.2 text:

- §5.5: "V1.2 commit 2 adds slots: `attempted_wp_user_id: Option<u64>`, `attempted_surface_client_id: Option<String>`, `ip_hash: Option<String>`, `user_agent_hash: Option<String>`."
- AC F: "`presence_nonce_rejected` carries `attempted_wp_user_id` + `attempted_surface_client_id` + `rejection_reason` (one of `Replayed` / `WrongUser` / `MalformedRequest`). `NonceAuditContext` struct at `surface_nonce.rs:1235` extended with the new slots."
- §9 invariant #10: "`NonceAuditContext` MUST carry `attempted_wp_user_id`, `attempted_surface_client_id`, `ip_hash`, `user_agent_hash` slots. Grep gate on the struct definition at `surface_nonce.rs:1235`. Verified by §8.10 audit forensic test."

Substrate read confirmed: the current `NonceAuditContext` struct at `surface_nonce.rs:1157-1171` has no `attempted_*`, `ip_hash`, or `user_agent_hash` slots — the struct only carries session + request-routing fields. The packet correctly identifies this as a W4 addition (not claimed as existing substrate). AC F now names the testable fields. Finding closed.

---

## Net-new STRIDE/OWASP probes — cycle-3

### STRIDE-Spoofing: ip_hash HKDF salt + purpose label

**Assessment: FYI — anchor 50**

§5.5 states: "derive via HKDF from the existing pairing root stored in the keychain (same key family as `PresenceNonceDigestKey` at `surface_nonce.rs:421`)." The spec says the helper should be factored as `derive_audit_subkey("ip_hash")`.

The existing `PresenceNonceDigestKey` derivation at `surface_nonce.rs:410-418` uses `PRESENCE_NONCE_KEY_SALT = b"DAILYOS-SURFACE-PRESENCE-NONCE-SALT-V1"` and `PRESENCE_NONCE_KEY_INFO = b"dailyos.surface.presence_nonce.digest.v1"` as the HKDF salt and info parameters. Cross-purpose key reuse is prevented when the `info` parameter differs between derived keys.

The plan specifies a purpose-binding label ("ip_hash" in the helper name) but does not explicitly state that the HKDF `info` bytes for the `ip_hash` subkey will be distinct from `PRESENCE_NONCE_KEY_INFO`. If the L1 implementer passes the same `info` bytes for both derivations, the `ip_hash` key and the nonce-digest key will be identical — defeating purpose isolation.

This is addressable in L1 with one line: define `AUDIT_IP_HASH_KEY_INFO = b"dailyos.surface.audit.ip_hash.v1"` (or equivalent) and use it in `derive_audit_subkey("ip_hash")`. The packet's HKDF model is sound and the same-root derivation is the right approach; only the `info`-parameter binding is left implicit. No packet revision needed — flag as L1 implementation note. Anchor 50.

### STRIDE-Tampering: stored binding immutability post-issue

**Assessment: CLEAN**

The plan at §5.2 asserts: "the verify path reads `payload_json` from the stored binding, NEVER from the verify request body." Substrate read confirms: `SurfaceNonceStore` is `Mutex<HashMap<NonceDigest, PresenceNonceBinding>>` at `surface_nonce.rs:565-575`. There is no API that updates a binding after insertion — `issue()` inserts via `inner.by_digest.insert()` at `:627`; `verify_and_consume` calls `binding.try_mark_consumed()` which sets `lifecycle.consumed_at` only; `invalidate_composition` calls `try_mark_invalidated()` which sets `lifecycle.invalidated_at` only. Neither mutates `fields` (which will carry `payload_json`). The binding fields are structurally immutable post-insert. No gap.

### STRIDE-DoS: malformed payload_json + rate-limit adequacy

**Assessment: CLEAN**

The attack vector: attacker issues a valid nonce with a valid `action_kind`, then sends a malformed `payload_json` variant shape on verify. The verify path calls `try_mark_consumed` (consumes the nonce) then calls `record_claim_feedback` which calls `validate_feedback_payload` at `claims.rs:5150`. Malformed payload causes `validate_feedback_payload` to reject, so the nonce is consumed but feedback is not written — consistent with the §5.4 fail-closed model. The attacker burns one nonce slot per attempt.

Rate-limit bounds: the nonce issue budget at `surface_nonce.rs:27-54` is `DEFAULT_BUDGET_PER_MINUTE = 240` per `(surface_client_id, wp_user_id, claim_id, field_path, action)` tuple. `DEFAULT_MAX_OUTSTANDING_PER_SESSION = 64`. `DEFAULT_MAX_LIVE_BINDINGS = 4096` global. An attacker who can authenticate and pair (required to reach the issue endpoint) can burn at most 240 nonces/minute per tuple, bounded globally at 4096 in-flight. For a local-to-local single-user deployment, reaching this budget requires intentional abuse of the authenticated session — within the accepted threat model. The per-tuple granularity means even exhausting one tuple does not block feedback for other claims. No gap at the plan level.

Note: the WP-side allowlist update in §5.6 step 2 also narrows this attack surface — after W4, only valid 9-variant `action_kind` strings reach the runtime. Malformed `action_kind` is rejected at WP before nonce issue.

### STRIDE-EoP: `/dailyos/v1/nonce/verify` permission callback strictness

**Assessment: CLEAN**

§5.6 step 1 specifies the new `/dailyos/v1/nonce/verify` REST endpoint "reuses `can_issue_presence_nonce`-equivalent logic; derives `wp_user_id` from authenticated session."

Substrate read of `can_issue_presence_nonce` at `class-dailyos-plugin.php:780-798`: checks (1) `is_user_logged_in()`, (2) `current_user_can('edit_post', $post_id)` or `current_user_can('edit_posts')` — same capability required to author content, (3) `DailyOS_Credential_Store::is_paired()`. This is the same three-factor check as the issue endpoint. The verify endpoint is specified as equivalent, not weaker. The plan's AC G and §8.7 PHPUnit both exercise "permission callback rejects unauthenticated" and "requires pairing marker." No EoP vector.

One note for L1: the plan should register the verify route with `can_issue_presence_nonce` (not a new callback) to structurally prevent drift. The packet language "reuses `can_issue_presence_nonce`-equivalent logic" leaves open the possibility of an L1 implementer writing a weaker new callback. Tightening AC G to say "MUST use `can_issue_presence_nonce` directly" would close this — but this is advisory, not a blocking gap, because §8.7 PHPUnit covers the test surface.

### OWASP-A01: `wp_user_id` derivation in WP REST handler (cannot come from request body)

**Assessment: CLEAN**

§5.3 and §5.6 both specify: "WP-side: ensure the `/dailyos/v1/nonce/verify` REST handler derives `wp_user_id` from the authenticated WP session — NEVER from request body."

Substrate read confirmed three layers of enforcement:

1. **WP layer (existing issue handler):** `class-dailyos-plugin.php:911` reads `$current_user_id = (int) get_current_user_id()` — derived from the authenticated WP session, never from `$params`. The `$payload` built at `:936` sets `wp_user_id` from `$current_user_id`. The verify handler is specified to follow the same pattern.

2. **Runtime signed-route layer (existing):** `validate_session_bound_wp_user_id_for_request` at `surface_client.rs:505-523` checks all channels (body, query, headers). If `wp_user_id` appears in the request body AND disagrees with the session-bound value, the request is rejected at `surface_runtime/mod.rs:1957-1977` before dispatch. Critically: the function does NOT reject requests where `wp_user_id` is absent from the body — it only rejects when an asserted value disagrees with the session. This means if the JS were to omit `wp_user_id` from the verify body, the WP handler's session-derived value is the only `wp_user_id` the runtime sees. If the JS were to supply a spoofed `wp_user_id` in the body, the runtime would reject it (because the WP transport layer itself supplies the session-derived value, and any JS-supplied value in the outer HTTP request would be checked against the session).

3. **Nonce binding cross-check (existing substrate):** `compare_binding_tuple` at `surface_nonce.rs:1339` rejects if `binding.fields.wp_user_id != request.wp_user_id` — the `request.wp_user_id` here comes from `VerifyNonceRequest::parse` which reads it from the runtime request body (which was constructed by the WP transport with the session-derived value). No JS-controlled path to supply a different value.

The three-layer enforcement is robust. No gap.

### OWASP-A03 (Injection): `payload_json` variant-shape validation strictness

**Assessment: CLEAN with one L1 note**

§5.6 step 3 specifies: "Extend WP issue handler to accept `payload_json?: object` (validated against the variant-specific shape: `{source_index}` for `wrong_source`, `{corrected_text}` for `needs_nuance` required, `{corrected_to}` for `wrong_subject` optional, `{surface}` for `surface_inappropriate`, `{invocation_id}` for `not_relevant_here`). 500-char cap on user-authored strings."

§12 open question #5 (V1.2 NEW) directly addresses this: "do we duplicate the shape spec in PHP, or call a runtime validation endpoint before mint? Recommended: duplicate in PHP (small surface area; fast feedback for the user) + add a parity test that asserts the PHP allowlist matches the Rust enum."

Two-layer validation (WP + Rust `validate_feedback_payload` at `claims.rs:5150`) is the correct model. The WP layer validates shape for fast rejection; the Rust layer validates before writing. Even if the WP layer were bypassed, `validate_feedback_payload` is the authoritative gate.

Injection risk assessment: `payload_json` user-authored string fields (`corrected_text`, `corrected_to`, etc.) flow from WP → runtime as JSON string values, stored via `rusqlite` parameterized binding (existing substrate, confirmed in cycle-2 OWASP-A03 finding). No string interpolation into SQL. The §6 #12 forward-constraint prevents `corrected_text` from entering LLM prompts without sensitivity=User filtering. No injection path identified.

L1 note: the WP-side validation should reject `payload_json` values that are not plain objects (e.g., arrays, nested objects beyond the expected depth). The 500-char cap covers string-field length; depth limit prevents nested-object escape. This is a standard defensive measure at the WP validation layer, not a plan-level gap.

---

## Findings summary

| # | Finding | Severity | Anchor | Status |
|---|---|---|---|---|
| C2-HIGH-1 | WP allowlist at `:907` hardcoded to 4 old variants | HIGH | 100 | CLOSED — §5.6 step 2 + AC G + §9 inv #9 + §8.8 |
| C2-MED-2 | HKDF binding for `payload_json` ambiguous | MEDIUM | 75 | CLOSED — AC B restated as in-memory store isolation |
| C2-MED-3 | NonceAuditContext slots missing | MEDIUM | 75 | CLOSED — §5.5 + AC F + §9 inv #10 |
| C3-STRIDE-Spoofing | `ip_hash` HKDF `info` parameter not explicitly distinct | Advisory | 50 | L1 note: define `AUDIT_IP_HASH_KEY_INFO` constant distinct from `PRESENCE_NONCE_KEY_INFO` |
| C3-STRIDE-Tampering | Stored binding immutability post-issue | — | — | CLEAN |
| C3-STRIDE-DoS | Malformed payload burns nonce; rate-limit adequacy | — | — | CLEAN |
| C3-STRIDE-EoP | `/nonce/verify` permission callback strictness | — | — | CLEAN |
| C3-OWASP-A01 | `wp_user_id` cannot be supplied via request body | — | — | CLEAN |
| C3-OWASP-A03 | `payload_json` injection / nested-object escape | — | — | CLEAN |

---

## Lock criteria check

- Cycle-2 HIGH closed: YES
- Cycle-2 MEDIUMs closed: YES (both)
- Net-new probes: 1 advisory (anchor 50, L1 note only), 5 clean
- CSO + security-auditor unanimous APPROVE gate: security-auditor APPROVES
- No new design-change conditions introduced

Packet locks on security-auditor dimension. The one advisory (HKDF `info` parameter distinction) is an L1 implementation note, not a plan condition.
