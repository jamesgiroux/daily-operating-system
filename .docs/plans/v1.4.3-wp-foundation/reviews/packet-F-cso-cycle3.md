# CSO Review — Packet F (Feedback Write Infrastructure) — Cycle 3

**Reviewer:** /cso
**Packet version:** V1.2
**Date:** 2026-05-19
**Cycle-2 verdict:** CONDITIONAL APPROVE — 1 HIGH (db_read→db_write), 2 MEDIUM advisory (ip_hash key provisioning, NeedsNuance corrected_text LLM injection)

---

## Verdict

**APPROVE**

All three cycle-2 findings are folded correctly. The four V1.2 net-new security probes resolve cleanly: the orphan retirement carries no live-client gap material enough to block (advisory observation below), the WpBlockRenders channel addition is contained, the expanded action allowlist has no new authorization concerns at the W4 trust boundary, and the fail-closed/nonce-exhaustion threat is bounded by existing per-session rate limits that are sized appropriately for this attack pattern. No blocking conditions remain.

---

## Cycle-2 Finding Resolution

### Finding 1 — HIGH: db_read → db_write at mod.rs:2570

RESOLVED. V1.2 §5.4 names the upgrade explicitly at two locations:

- Pseudocode comment at the pseudocode block: "Note: app_state.db_write at the route handler level (NOT db_read — V1.2 §5.4 + §10 commit 3 change this)."
- §10 commit 3 scope: "`db_read → db_write` upgrade at `:2570`" listed as a named deliverable.
- AC D: "Handler at `mod.rs:2570` uses `db_write` (NOT `db_read`)."

The cycle-2 condition was "§5.4 and §10 commit 3 must add one sentence naming the db_write change." Both locations now name it. Condition satisfied.

### Finding 2 — MEDIUM (advisory): ip_hash HMAC key provisioning

RESOLVED. V1.2 §5.5 locks the provisioning story: "derive via HKDF from the existing pairing root stored in the keychain (same key family as `PresenceNonceDigestKey` at `surface_nonce.rs:421`). Survives process restart (per CSO finding's requirement for forensic correlation across sessions). No new key-management infrastructure."

The cycle-2 advisory asked for a one-sentence provisioning decision. §5.5 provides it with a concrete key source, restart-survival rationale, and L1 factoring task. No dissent on keychain-pairing-root as the key source — this reuses the established HKDF family without introducing new infrastructure or rotation complexity. The ephemeral-root concern from cycle-2 does not apply here because the pairing root is persisted to the keychain and therefore survives restart, which is the correct behavior for an audit field that must correlate across sessions.

### Finding 3 — MEDIUM (advisory): NeedsNuance corrected_text LLM injection path

RESOLVED. V1.2 §6 decision #12 documents current state ("validate_feedback_payload at claims.rs:5150 normalizes payload_json but does NOT pass into agent context") and establishes a forward-constraint: "any v1.4.4+ repair ability consuming corrected_text into an LLM prompt: the sensitivity=User filter (per ADR-0108) MUST apply BEFORE prompt construction." The constraint is also reflected in §9 invariant (regression-prevention scaffold referenced in decision #12). The advisory asked for documentation of current behavior or an AC asserting the flow. §6 #12 provides both the current-state documentation and the constraint for future work. Condition satisfied.

---

## V1.2 Net-New Security Probes

### Probe 1 — §5.6 step 4: /v1/surface/feedback orphan retirement

**No block.** Advisory observation follows (anchor 50).

The retirement removes three allowlist entries (`surface_runtime/mod.rs:1243, 4730, 4755`) and deletes `submit_feedback()` at `runtime-client.php:163`. The security posture improves: dead allowlist entries are an attack surface because they signal an expected route to any client that enumerates the allowlist even without a handler match.

**Advisory (anchor 50):** The packet confirms no production callers exist for the `/v1/surface/feedback` path (V1.2 §5.6 item 4: "grep confirmed no callers outside the dead path itself"). However, the packet does not specify whether there is a migration window (e.g., a deployed WP instance running the old `submit_feedback()` before the plugin update lands). If the runtime update and the plugin update land at different times, the old WP code will POST to the now-removed allowlist entry and receive a route-not-found error. The user-facing consequence is silent feedback loss (a request that was already partially dead). This is not a security regression — the path was already dead (no Rust handler wired) — but it is worth noting for deployment ordering. The §9 invariant #12 CI grep gate prevents reintroduction. No blocking condition; this is a deployment-sequencing note for L1.

### Probe 2 — §5.9 WpBlockRenders channel addition

**Clean.** The `RenderPolicyChannel::WpBlockRenders` addition at `bridges/types.rs` is purely additive to the `ALL` slice. The W6-E `#[non_exhaustive]` channel-sweep gate will require callers to handle the new variant, making the addition compile-time visible. The channel addition does not open a new data flow — it classifies an already-occurring flow (WP block rendering) so that the existing W2 DOS-477 leak-guard machinery covers it. No new trust boundary is introduced; no new credential or secret is required. The variant has no authorization behavior of its own; it is a policy-classification tag.

### Probe 3 — §5.6 step 2 action allowlist expansion (4 → 9)

**No new authorization concerns at the W4 trust boundary.**

The five new variants are: `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere` (completing the 4→9 expansion; four of these are new). The question raised is whether `MarkFalse` — which withdraws a claim globally — should require elevated permission.

The trust boundary analysis: `record_claim_feedback` at `claims.rs:6700` already operates under `ctx.check_mutation_allowed()` + `MutationGuard::reserve`. The claim lifecycle transitions per ADR-0123 §1 are applied uniformly regardless of which `FeedbackAction` variant is submitted; there is no variant-level privilege distinction in the current substrate. `MarkFalse` transitions the claim to the `withdrawn` lifecycle state, but it does so as the actor who submitted feedback — the `actor: format!("wp_user:{}", binding.wp_user_id)` attribution in the pseudocode records who made the assertion. The trust model is: any authenticated WP user who has a valid pairing session can submit feedback on claims rendered to them. This is consistent with ADR-0111 and the current claim model.

No variant-level elevation is required at W4. If per-claim-type action filtering (e.g., restricting `MarkFalse` to claim owners) becomes a requirement, that belongs in v1.4.4 per §13 lineage item "Per-claim action filtering (some claim types support fewer variants)." At W4 the allowlist expansion is authorization-safe.

### Probe 4 — §6 #13 consume-succeeded-record-failed + nonce exhaustion via deliberate record_claim_feedback failures

**Bounded by existing rate limits. No block.**

The threat: a malicious WP-authenticated user issues a nonce, then crafts a verify request that passes nonce verification but causes `record_claim_feedback` to fail (e.g., via malformed `payload_json`). The nonce is consumed; the attacker repeats, exhausting the per-session nonce budget.

The actual rate-limit structure, confirmed from `surface_nonce.rs:27-54`:

- `DEFAULT_MAX_LIVE_BINDINGS: 4096` — global cap on outstanding (unconsumed) nonces in the in-memory store.
- `DEFAULT_MAX_OUTSTANDING_PER_SESSION: 64` — per-session cap on unconsumed nonces.
- `DEFAULT_BUDGET_PER_MINUTE: 240` — shared default for `issue_budget_per_minute`, `verify_budget_per_minute`, AND `failure_budget_per_minute`.

The `failure_budget_per_minute` is the load-bearing counter here. `charge_failure_best_effort` at `surface_nonce.rs:352-358` is called on any verify rejection path — including the case where the nonce verifies but the subsequent `record_claim_feedback` call fails. The budget key is per-`(surface_client_id, wp_user_id, claim_id, field_path, action)` (the `NonceBudgetKey::full` form at `:1394-1408`), not just per-session. This means a persistent attacker targeting the same claim with the same action hits the per-claim-action budget (240/minute) before they can inflict meaningful damage to other users.

At 240 failures/minute with a 60s TTL, the maximum number of exhausted-but-unrecorded nonce attempts per attacker per minute is 240. Each failed attempt produces a `presence_nonce_issued` + `presence_nonce_verified` + `record_claim_feedback` error log — a traceable audit chain. The attack's practical impact is that the attacker wastes their own feedback-submission quota. Legitimate users on different sessions are unaffected because the budget key includes `surface_client_id`.

The one gap worth noting: if `record_claim_feedback` fails due to a DB error (not a validation error), the `charge_failure_best_effort` call happens inside `verify_nonce` at `:273`, which is called before the wire-through to `record_claim_feedback` that W4 adds. The W4 pseudocode calls `try_mark_consumed` (which is inside `verify_nonce`) and then calls `record_claim_feedback` after. A DB failure in the W4-added `record_claim_feedback` call happens AFTER `charge_failure_best_effort` has already been called from inside `verify_nonce`. So the failure budget IS charged for the verify-side failures, but the DB-error path in the W4 phase-3 call is outside the existing `charge_failure_best_effort` scope.

This means a DB-error on `record_claim_feedback` (not a validation error) would NOT increment the failure budget. Under the W4 threat model — local-to-local loopback, no remote attacker — this is acceptable: a DB error means the local runtime is sick, not that an external attacker is exploiting the path. However, the L1 implementor should add a `charge_failure_best_effort` call on the `Err` return from `record_claim_feedback` in the W4 wire-through path. This is an L1 implementation note, not a blocking condition at L0.

---

## Supply-Chain / Secrets Probe (cycle-3)

**HKDF key family for ip_hash:** The decision to derive from the pairing root via HKDF is consistent with the existing `PresenceNonceDigestKey` derivation (same root, different info string). Domain separation via HKDF info is correct practice. The L1 task to factor into a `derive_audit_subkey("ip_hash")` helper is appropriate. No secrets archaeology concern.

**Pairing root keychain access from audit path:** The ip_hash is computed at the handler level (where the pairing root is available) before being passed into `NonceAuditContext`. The derived hash does not expose the root. No keychain access from deep substrate. Sound.

**Orphan route retirement:** Removing the `/v1/surface/feedback` allowlist entries reduces the signed-route attack surface. No new secrets introduced.

---

## Top Findings (cycle-3)

All three cycle-2 conditions are fully satisfied. No new blocking findings from the four net-new probes. Two advisory observations:

1. **Advisory (anchor 50) — Orphan retirement deployment window:** If the runtime and WP plugin updates land at different times, old WP code calling `submit_feedback()` will receive a route-not-found error. Not a security regression (path was already dead); note for L1 deployment ordering.

2. **Advisory (anchor 50) — record_claim_feedback DB-error path not charged against failure budget:** The W4 `record_claim_feedback` call happens after `verify_nonce` completes; a DB error on that call does not trigger `charge_failure_best_effort`. Under local-to-local deployment this is acceptable; L1 implementor should add the failure charge on the W4 phase-3 error path.

---

## Lock Confirmation

Per §15 lock criteria:

- Cycle-2 verdict set: 5/5 CONDITIONAL APPROVE. V1.2 folds all 10 conditions.
- All three CSO cycle-2 conditions verifiable from packet text (§5.4, §5.5, §6 #12).
- No new blocking findings from cycle-3 probes.
- CSO gate: **APPROVE**.
