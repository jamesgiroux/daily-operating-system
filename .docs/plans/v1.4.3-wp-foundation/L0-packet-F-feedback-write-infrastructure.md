# L0 Packet F — v1.4.3 W4: Feedback Write Infrastructure

## 1. Header

- **Author:** James Giroux (with Claude)
- **Date:** 2026-05-19
- **Linear:** [DOS-683](https://linear.app/a8c/issue/DOS-683) (v1.4.3 W4 — Feedback Write Infrastructure)
- **Branch:** TBD (`dos-683-w4-feedback-write` off `dev` once V1.x locks; rebases on top of `af8108da` which is dev after W3 PR #315 merged).
- **Wave plan:** `.docs/plans/v1.4.3-waves.md` §W4 (lines 203–218)
- **Upstream gate:** W2 (DOS-682, merged) — all 11 primitive blocks + `account-overview` composite + presence-nonce REST scaffolding live on dev. W3 (DOS-698, merged via PR #315 at `af8108da`) — `dailyos_account` CPT + plugin-owned baseline token shim + magazine theme.

**Intelligence Loop integration check — APPLIES.** W4 wires the **already-locked** `record_claim_feedback` write path into the **already-shipped** presence-nonce lifecycle. Per ADR-0123:

1. **Claim model:** Feedback writes mutate existing claims (state, source weights, agent ledger) and may create user-authored superseding/absence/tombstone claims. Subject attribution preserved via `SubjectRef` (ADR-0105). Temporal scope: feedback rows carry `submitted_at` / `applied_at` timestamps. Sensitivity: `payload_json` may carry user-authored `corrected_text` / `corrected_to` / surface markers — sensitivity=User per ADR-0108. Lifecycle: claim transitions to `superseded` / `withdrawn` / unchanged depending on `FeedbackAction` variant per ADR-0123 §1.
2. **Provenance + trust:** Each feedback row carries actor (`UserActor` for hand-clicked corrections, distinct from `AgentActor` automation). Trust factors mutate per ADR-0123 §4: source-attribution downweight on `WrongSource`, freshness-factor downweight on `MarkOutdated`, no penalty on `CannotVerify`. Trust-band-rendering refreshes on the next projection. **None of this is net-new — it's `record_claim_feedback`'s existing contract.**
3. **Signals + invalidation:** Feedback emits signals via the existing `record_claim_feedback` body at `src-tauri/src/services/claims.rs:6700` (already wired to MutationGuard chokepoint + signal propagation). W4 does not invoke signal infrastructure directly.
4. **Runtime + surfaces:** W4 extends the **existing** `SurfaceNonceService::verify_nonce` (`src-tauri/src/services/surface_nonce.rs:218`) so that on successful nonce verification the path continues into `record_claim_feedback`. The first **WP surface** write path — but **NOT** the first write path overall; the existing `check_mutation_allowed` chokepoint at `claims.rs:6700` and the existing `MutationGuard::reserve` at `claims.rs:6714` already cover mutation discipline.
5. **Feedback loop:** This IS the feedback loop. W4 closes the round-trip: rendered claim → user click → existing presence-nonce mint → user confirm → existing presence-nonce verify → **new** wire-through into `record_claim_feedback` → invalidation → next render reflects the feedback.

**ADR-named contracts:** **ADR-0123** (typed claim feedback semantics — 9-variant `FeedbackAction` enum at `src-tauri/abilities-runtime/src/abilities/feedback.rs:31`), **ADR-0111** (surface-independent ability invocation — writes through `services/claims`, not direct DB writes from WP), **ADR-0126** (memory substrate invariants — all mutations through `services/`), **ADR-0108** (actor/surface-filtered provenance — `payload_json` user-authored fields are sensitivity=User).

## 2. Changelog

- **V1.0 (2026-05-19):** Initial L0 draft. Two convergent BLOCK verdicts (code-reviewer + codex challenge) flagged the same class-pattern: V1.0 proposed a **parallel** substrate (`services/surface_feedback.rs` + `surface_feedback_nonces` table + new `/v1/feedback` REST endpoint + new audit event types `pairing.feedback.*`), but **the substrate already exists**. The K-in obligation per CLAUDE.md mandate was missed: the V1.0 grep used `"surface_feedback"` as the query term, which has zero hits, instead of searching by substrate type (`"nonce"`, `"issue/verify pattern"`, `"feedback service"`) — which would have surfaced `src-tauri/src/services/surface_nonce.rs` with the full 2-phase nonce lifecycle + HKDF + rate limits + live REST routes.

- **V1.1 (2026-05-19):** Folded L0 cycle 1 reviewer findings. Cycle-1 verdicts:
  - **code-reviewer BLOCK** — `surface_nonce.rs` already exists with full 2-phase nonce lifecycle + HKDF digest key + rate-limit budgets + live REST routes; V1.0 proposes a parallel service when the actual gap is small.
  - **codex challenge BLOCK** — 4 of 9 ADR-0123 variants need `payload_json` (`WrongSource`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`); V1.0 only carries `action_kind` + `user_intent_text`; replay atomicity unspecified; `wp_user_id` missing from schema; presence-nonce precedent overclaimed (existing pattern derives session identity, doesn't trust JS-supplied `surface_client_id`); enum lives in `abilities-runtime/abilities/feedback.rs`, not `claims.rs`; redaction-channel baseline is 9 (per v1.4.1 W6-E L0 packet), not 10 as V1.0 claimed.
  - **security-auditor CONDITIONAL APPROVE** — `wp_user_id` binding HIGH (existing scaffold doesn't bind `surface_client_id → wp_user_id`); TOCTOU race on consume MEDIUM; `user_intent_text` channel-count claim unverified MEDIUM; `replay_rejected` audit payload missing attempted-actor identity MEDIUM; `action_kind` should be server-read from row not client-resupplied MEDIUM.
  - **CSO mandatory CONDITIONAL APPROVE** — §1 framing wrong (existing `check_mutation_allowed` chokepoint covers mutations; W4 adds the WP SURFACE, not the chokepoint); `wp_user_id` binding HIGH; audit payload insufficient for forensics MEDIUM; channel-list claim unverified MEDIUM; §5.4 audit-emit-before-record-feedback ordering creates partial-commit risk MEDIUM.
  - **codex consult CONDITIONAL APPROVE** — PR sequencing fix (audit registration must precede service tests; runtime route methods before PHP/REST tests); e2e fixture needs real Rust + DB + projection (in-process runtime/handler fixture OK, no full macOS app); lock single-trigger + reveal-menu (not 9-button menu) for affordance UX; existing `dailyos_nonce_sweep` cron is a no-op + presence nonce is not table-backed — V1.0's "two nonce tables" claim is wrong.

  **Class-pattern (the L0 gate James enforces):** K-in obligation missed. V1.0's grep used the proposed-name as the query term. Per memory `feedback_check_substrate_before_authoring_primitives.md` the K-in search must use **substrate-type** queries before authoring; missing the canonical service triggered the parallel-substrate design. V1.1 grounds every claim against an actually-grepped file path.

  **Major folds (substrate reframe, not surgical):**
  - §3 + §4 K-in rewritten; `surface_nonce.rs` enumerated with file:line.
  - §5 reframed from "parallel service" to "extend existing".
    - Deleted §5.1, §5.2, §5.3 (migrations).
    - §5.4 service → extend `PresenceNonceAction` 4 → 9 + wire `verify_nonce` → `record_claim_feedback`.
    - §5.5 audit events → align with existing `presence_nonce_*` names (NOT `pairing.feedback.*`).
    - §5.6 REST → extend the existing `/v1/surface/nonce/{issue, verify}` shape (NOT new `/v1/feedback`).
  - §6 decisions:
    - #1 keep (services-only writes).
    - #2 narrow CI gate to `record_claim_feedback` body + `FeedbackAction` enum (not all of `claims.rs`).
    - #6 drop 24h TTL claim; keep existing 60s with rationale.
    - #8 add canonical `wp_user_id` locus (schema column + verify check).
    - NEW #9: feedback flow has no migrations.
    - NEW #10: fail-closed replay rejection (explicit).
  - §1 framing: "first WP-SURFACE write path; existing `check_mutation_allowed` chokepoint covers mutation discipline" (NOT "first write path").
  - §9 invariants: drop migration-related; add `wp_user_id` binding gate.
  - §10 PR shape: 6 commits (down from 8); ordered to satisfy codex consult sequencing — start with `PresenceNonceAction` extension, then `surface_nonce.rs` `payload_json` plumbing, then `verify_nonce` → `record_claim_feedback` wire, then WP REST extension, then JS affordance, then end-to-end fixture.
  - §14 AC: drop migration-related; tighten `wp_user_id` + `payload_json` coverage.

- **V1.2 (2026-05-19):** Folded L0 cycle 2 reviewer findings. Cycle-2 verdicts: 5/5 CONDITIONAL APPROVE; 0 BLOCK; 0 class-pattern recurrence. All conditions surgical; cycle-3 review needed per §15 lock criteria (≥3 CONDITIONAL triggers re-review).

  **Convergent finding across 4 of 5 reviewers — §5.4 atomicity rewrite:**
  - The V1.1 pseudocode promised `with_transaction(db, |tx| { mark_consumed_if_unclaimed(tx, ...); record_claim_feedback(ctx, tx, input) })` — but the substrate cannot support this. `SurfaceNonceStore` is `Mutex<HashMap<NonceDigest, PresenceNonceBinding>>` at `surface_nonce.rs:565-575` (not table-backed; no SQL rows to UPDATE). `record_claim_feedback` at `claims.rs:6700` takes `&ActionDb` (not `&Tx`) and opens its own `with_claim_transaction` at `:6717`. Two consistency domains; one cannot be wrapped in the other. Additionally, the existing handler at `surface_runtime/mod.rs:2570` is `app_state.db_read(...)` — calling `record_claim_feedback` (a write) from a reader connection fails at runtime.
  - V1.2 drops the single-tx framing entirely. New §5.4: consume-then-record using existing `try_mark_consumed` primitive at `surface_nonce.rs:518` (Mutex-guarded is the atomicity primitive); call `record_claim_feedback` after consume succeeds; explicit fail-closed for the rare consume-succeeded-record-failed path (user retries with a NEW nonce; the consumed nonce stays consumed in audit trail). Handler upgrade `db_read → db_write` named in §5.4 + §10 commit 3.

  **Cycle-2 net-new findings (10 surgical conditions folded):**
  1. **(security-auditor HIGH)** WP-side `action` allowlist hardcoded to 4 old variants at `class-dailyos-plugin.php:907` — silently rejects all 5 new variant names before the request reaches runtime. V1.2: AC G + §8.8 require updating to 9 variants; §5.6 specifies the change.
  2. **(code-reviewer HIGH)** Invented rejection_reason strings (`"already_consumed"`, `"wp_user_mismatch"`) collide with existing `PresenceNonceRejectReason::{Replayed, WrongUser, MalformedRequest}` enum. V1.2 uses the existing enum names everywhere; AC E/F/§8.3/§8.4 updated.
  3. **(code-reviewer + codex challenge HIGH)** WP-side `/dailyos/v1/nonce/verify` REST handler doesn't exist — only `/dailyos/v1/nonce` (issue) registered at `class-dailyos-plugin.php:573-601`. JS phase-2 has no WP landing zone. Orphan: `/v1/surface/feedback` is on the signed-route allowlist at `surface_runtime/mod.rs:1243` + WP `submit_feedback()` posts to it at `runtime-client.php:163` but no Rust handler is wired. V1.2 §5.6: register the WP-side `/verify` handler + retire the `/v1/surface/feedback` allowlist entry + retire `submit_feedback()`. §10 commit 4 expanded.
  4. **(codex consult MEDIUM)** §12 open question #1 was not just informational — it blocks §8.9 redaction test + §9 invariant #4. Channel registry verified at 9 (`RenderPolicyChannel::ALL` at `src-tauri/src/bridges/types.rs:84-126`). WP block render NOT in `RenderPolicyChannel`. V1.2 adds §5.9: `WpBlockRenders` variant + one ALL entry; closes §12 #1; AC adds compile-time check.
  5. **(code-reviewer + codex challenge MEDIUM)** HKDF binding claim for `payload_json` is mechanically incorrect — `PresenceNonceDigestKey::digest` at `surface_nonce.rs:421` accepts only `nonce_bytes`; binding fields are tuple-compared, not cryptographically mixed. V1.2 restates AC B as "tamper-resistant via in-memory store isolation": the verify path reads `payload_json` from the stored binding (never from request body). Same protection, accurate wording.
  6. **(/cso MEDIUM)** `ip_hash` HMAC key had no provisioning story. **LOCKED in V1.2:** derive via HKDF from existing pairing root in keychain (same key family as nonce digest; survives restart; no new key-management infra). §5.5 specifies.
  7. **(security-auditor MEDIUM)** AC F + §8.4 didn't explicitly assert `attempted_wp_user_id` + `attempted_surface_client_id` in `presence_nonce_rejected` audit payload. `NonceAuditContext` at `surface_nonce.rs:1235` has no such slots. V1.2 §5.5: add slots to the struct; AC F names them as testable assertions.
  8. **(/cso MEDIUM)** `NeedsNuance` `corrected_text` LLM injection path unspecified. V1.2 documents current state: `validate_feedback_payload` at `claims.rs:5150` normalizes but does NOT pass into agent context today. Adds §6 #12 forward-constraint: if a future repair ability consumes `corrected_text` into LLM prompts, sensitivity=User filter applies before prompt construction.
  9. **(code-reviewer MEDIUM)** §5.3 overclaimed `wp_user_id` binding as W4 addition — both binding storage at `:485` and verify-time check via `compare_binding_tuple` at `:1339` already exist. V1.2 §5.3 corrects: the binding storage + verify cross-check are EXISTING substrate; W4 adds only the WP-side derivation (server-derive from session; never trust JS input) + the audit-payload extension.
  10. **(codex consult LOW) — §5.1 reviewer call CLOSED:** extend `PresenceNonceAction` 4 → 9 confirmed. The old 4 variant names have NO callers outside `surface_nonce.rs` itself (production caller at `:1690` is the only one, and it's test-fixture surface). The `From<PresenceNonceAction> for FeedbackAction` compile-time exhaustive match makes extension the minimum-abstraction path. Mapping shim has zero justification.

  **Locked V1.2 (no more reviewer calls):** §5.1 extend-to-9; §12 #1 channels=9 + add WpBlockRenders; §5.2 in-memory-store-isolation framing; ip_hash via pairing-root HKDF.

- **V1.3 (2026-05-19):** Folded L0 cycle 3 reviewer findings. Cycle-3 verdicts: 3 APPROVE (code-reviewer, /cso, security-auditor) + 2 CONDITIONAL APPROVE (codex consult LOW; codex challenge 2 HIGH + 1 MEDIUM + 2 LOW). Per `feedback_reviewer_dissent_is_signal.md`: 1/5 dissent on HIGH is signal — investigate, validate, fold. Codex challenge findings validated against substrate; all real.

  **V1.3 folds:**

  1. **(codex challenge HIGH #1)** §5.4 pseudocode named `try_mark_consumed(&request)` — that signature does NOT exist on the service. The real consume orchestration is `SurfaceNonceStore::verify_and_consume` at `surface_nonce.rs:640` returning `VerifiedNonce { consumed_at, nonce_digest, expected_claim_version, expected_composition_version }` at `:583`. The binding's `claim_id` / `action` / `payload_json` / `wp_user_id` are consumed inside `verify_and_consume` and NOT carried out via `VerifiedNonce`. Plus `actor: "wp_user:{}"` fails `validate_feedback_actor` at `claims.rs:5131`: `actor_class_for_actor` at `:5121` splits on `:`/`/`/`@`; head `"wp_user"` is NOT in the user allowlist (`"user"` or `"human"`). V1.3 §5.4 REWRITES the pseudocode against the real API + extends `VerifiedNonce` to carry the binding fields W4 needs + fixes actor format to `"user:wp:{wp_user_id}"` (head = `"user"`, passes the allowlist).

  2. **(codex challenge HIGH #2)** §6 #11 + §3 substrate table + AC F claimed 3 `PresenceNonceRejectReason` variants cover all W4 paths. **False** — the enum at `surface_nonce.rs:1067-1085` has 17 variants in active use (`MalformedRequest`, `MissingNonce`, `MalformedClaimVersion`, `UnauthenticatedSurface`, `WrongActor`, `ScopeDenied`, `WrongSession`, `WrongUser`, `WrongClaim`, `WrongField`, `MismatchedAction`, `Expired`, `Replayed`, `Invalidated`, `ClaimVersionStale`, `CompositionVersionStale`, `RateLimited`). The W4 path emits subset that includes at minimum `Invalidated`, `Replayed`, `Expired`, `WrongUser`, `WrongClaim`, `ClaimVersionStale`, `CompositionVersionStale`, plus the `MalformedRequest` / `MissingNonce` / `WrongActor` / etc. that come from earlier validation gates. V1.3 §3 substrate table + §6 #11 + AC F enumerate the full enum + note W4 paths use the existing emit set verbatim (no new variants, no renaming).

  3. **(codex challenge MEDIUM #3)** §5.9 + invariant #11 + AC J leaned on a W6-E `#[non_exhaustive]` channel-sweep gate that does **not exist in the tree today**. `RenderPolicyChannel` at `bridges/types.rs:82-94` is plain `#[derive]`, NOT `#[non_exhaustive]`. `ALL` at `:97` is `[Self; 9]` fixed-size; zero non-definition consumers (`grep -r "RenderPolicyChannel::ALL"` returns 0 hits outside the definition site). W6-E packet at `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:189` planned the gate; the gate was never built. V1.3 drops the auto-application claim; §5.9 specifies a manual test in `wp/dailyos/tests/blocks/FeedbackPayloadRedactionTest.php` (§8.9) that asserts `WpBlockRenders` is included in the leak-guard projection sweep; files the W6-E gate backfill as a maintenance follow-up (DOS-### TBD — not in W4 scope).

  4. **(codex challenge LOW #4)** §3 + §5.5 + §10 + inv #10 cited `NonceAuditContext` at `:1235`. Actual location is `:1158`. `:1235` is `audit_event`. V1.3 fixes citations.

  5. **(codex challenge LOW #5)** §3 + AC K + §5.6 step 4 cited `submit_feedback()` at `runtime-client.php:163`. Actual definition is at `:149`; `:163` is an internal path-call line. V1.3 fixes citations to avoid partial-deletion footgun at L1.

  6. **(codex consult LOW)** §5.8 step 5 and §8.10 asserted audit event order `presence_nonce_issued → claim_feedback_recorded → presence_nonce_verified`, but §5.4 (correctly) asserts `presence_nonce_issued → presence_nonce_verified → claim_feedback_recorded`. `presence_nonce_verified` fires inside `verify_and_consume`'s Mutex-guarded HashMap mutation, BEFORE `record_claim_feedback` is called. V1.3 strikes the §5.8 parenthetical + corrects §8.10.

  **L1 implementation notes** (cycle-3 reviewer FYIs that don't change packet text but L1 needs) captured in NEW §16 appendix:

- **V1.4 (2026-05-19):** Folded L0 cycle 4 reviewer findings. Cycle-4 verdicts: 1 APPROVE (code-reviewer) + 1 APPROVE-with-advisory (/cso) + 3 CONDITIONAL APPROVE (codex consult LOW×2 + advisory; security-auditor MEDIUM blocking + LOW advisory; codex challenge 2 structural + 3 consistency). All CONDITIONALs marked "strictly surgical V1.4 patch; no cycle-5 reviewer set needed" or "upgrades to APPROVE once applied."

  **STRUCTURAL folds (codex challenge cycle-4):**

  1. **§5.4 service-layer target, not store-layer.** V1.3 §5.4 step-2 pseudocode targeted `app_state.surface_nonce_store.verify_and_consume(...)` directly. That bypasses critical work in `SurfaceNonceService::verify_nonce` at `surface_nonce.rs:218-286`: `ensure_surface_client(session, fallback_request_id)`, `VerifyNonceRequest::parse(...)`, `NonceAuditContext::from_verify(...)`, `ensure_session_tuple(...)`, budget charging via `charge_budget(NonceBudgetClass::Verify, ...)`, nonce-bytes decode via `decode_presence_nonce`, failure-budget charging on errors, and the `presence_nonce_verified` audit-event construction. V1.4 §5.4 rewrites step 1 + step 2 to target the SERVICE: extend `SurfaceNonceVerify` (the public service return type at `surface_nonce.rs:1008`) to carry the binding fields W4 needs, and call `SurfaceNonceService::verify_nonce` (or a new `pub` sibling) from the handler. The store-layer change (`VerifiedNonce` extension at `:583`) remains but is the internal plumbing — the public surface is the service.

  2. **§5.9 reverses V1.3's wrong substrate read.** V1.3 claimed `#[non_exhaustive]` isn't on `RenderPolicyChannel` and `ALL` has zero consumers. **Both wrong.** `#[non_exhaustive]` IS at `bridges/types.rs:81`. `RenderPolicyChannel::all()` IS consumed at `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs:359, 372, 378, 654` (`assert_eq!(RenderPolicyChannel::all().len(), 9)` + matrix-row coverage + per-channel iteration). The W6-E auto-gate IS in place. V1.4 §5.9 corrects: gate exists; auto-application IS in play; adding `WpBlockRenders` to `ALL` will fail the bundle-17 sweep until the test + 3 fixture JSONs are updated. §10 commit 5 expanded to include the bundle-17 fixture-update task.

  **TEXT-FIX folds (codex consult LOW + security-auditor MEDIUM + /cso advisory + codex challenge consistency):**

  3. §9 inv #10: `surface_nonce.rs:1235` → `:1158` (`NonceAuditContext` struct definition; `:1235` is `audit_event` constructor — wrong target).
  4. §9 inv #6: invented `"already_consumed"` string → `PresenceNonceRejectReason::Replayed` enum variant.
  5. §9 inv #7: invented `"wp_user_mismatch"` string → `PresenceNonceRejectReason::WrongUser` enum variant.
  6. §9 inv #11: W6-E `#[non_exhaustive]` reference reframed per V1.4 §5.9 — gate exists; sweep consumer is `bundle17_*.rs:359, 372, 378, 654`.
  7. §10 commit 3 + AC K: `runtime-client.php:163` → `:149` (`submit_feedback()` definition; `:163` is internal path-call line).
  8. §10 commit 5: "W6-E `#[non_exhaustive]` channel-sweep gate exercise" → "bundle-17 sweep test update (len `9` → `10`) + `bundle-17/metadata.json` and any matrix-row fixture updates".
  9. §16 item 7: SHOULD → MUST (security-auditor advisory — prevents L1 from writing a weaker parallel callback).

  All CONDITIONALs marked themselves as upgrading to APPROVE once V1.4 is applied. Per `feedback_review_loop_l6_policy.md` + codex challenge's "no cycle-5 reviewer set needed," **V1.4 locks the packet**. §15 marks lock.


  - §5.4 noun chain — `verify_and_consume` is on `SurfaceNonceStore` not `SurfaceNonceService` (code-reviewer)
  - §5.9 fixed-size array `[Self; 9]` → `[Self; 10]` size annotation (code-reviewer)
  - `From<PresenceNonceAction> for FeedbackAction` crosses crate boundary (`src-tauri` → `src-tauri/abilities-runtime`); needs `pub` promotion or different impl site (code-reviewer)
  - Deployment ordering window for orphan retirement — runtime + WP retirement should land together or in tight sequence; out-of-sync deploys cause silent feedback loss (/cso)
  - Phase-3 `record_claim_feedback` failure should charge `failure_budget_per_minute` at `surface_nonce.rs:30-52` (/cso)
  - HKDF `info` parameter for `ip_hash` MUST be distinct from `PRESENCE_NONCE_KEY_INFO` (e.g., `AUDIT_IP_HASH_KEY_INFO = b"dailyos.surface.audit.ip_hash.v1"`) to prevent purpose-binding collision (security-auditor)
  - `/dailyos/v1/nonce/verify` permission callback should BE `can_issue_presence_nonce` (literal reuse), not `-equivalent logic` (security-auditor)
  - WP `payload_json` validation should reject non-plain-object values (no arrays, no deep nesting) — defensive measure (security-auditor)

## 3. Status Snapshot

- Linear ticket: DOS-683 (Backlog, v1.4.3 — WordPress Foundation, priority High).
- Substrate **already shipped** (consumed by W4, NOT authored):

| Component | File:line | Status |
|---|---|---|
| `SurfaceNonceService::issue_nonce` | `src-tauri/src/services/surface_nonce.rs:106` | live |
| `SurfaceNonceService::verify_nonce` | `src-tauri/src/services/surface_nonce.rs:218` | live |
| 60s TTL + HKDF digest key + rate-limit budgets | `surface_nonce.rs:27, …` | live |
| `PresenceNonceAction::{Correct, Dismiss, Corroborate, Contradict}` | `surface_nonce.rs:445` (4 variants) | live — **insufficient for ADR-0123's 9; W4 extends** |
| `presence_nonce_issued` / `_verified` / `_rejected` audit events | service emits internally | live |
| Runtime route `POST /v1/surface/nonce/issue` | `bridges/surface_client.rs:19` + `surface_runtime/mod.rs:2016` | live |
| Runtime route `POST /v1/surface/nonce/verify` | `bridges/surface_client.rs:20` + `surface_runtime/mod.rs:2019` | live (handler at `:2570` uses `db_read` — **W4 upgrades to `db_write`**) |
| Runtime route `POST /v1/surface/feedback` (allowlist entry) | `surface_runtime/mod.rs:1243, 4730, 4755` — allowlist + signed-route registration; **no handler match block** | orphan — **W4 retires** (path is nonce-based) |
| WP transport `issue_nonce()` | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:172` | live |
| WP transport `verify_nonce()` | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:188` | live |
| WP transport `submit_feedback()` | `runtime-client.php:149` (def); `:163` is internal path-call | dead path — **W4 retires** |
| WP REST `/dailyos/v1/nonce` (issue side) | `wp/dailyos/includes/class-dailyos-plugin.php:573-601` | live |
| WP REST `/dailyos/v1/nonce/verify` | NOT REGISTERED | **W4 registers** (signs `/v1/surface/nonce/verify` runtime call) |
| WP-side `action` allowlist (presence_nonce_payload) | `class-dailyos-plugin.php:907` hardcoded `['correct', 'dismiss', 'corroborate', 'contradict']` | **W4 expands to 9 variants** |
| `wp_user_id` binding storage | `surface_nonce.rs:485` (`PresenceNonceBindingFields`) | live — **already bound** |
| `wp_user_id` verify cross-check | `surface_nonce.rs:1339` (`compare_binding_tuple`) | live — **already cross-checks** |
| `PresenceNonceRejectReason` enum (17 variants — V1.3 correction) | `surface_nonce.rs:1067-1085` (`MalformedRequest`, `MissingNonce`, `MalformedClaimVersion`, `UnauthenticatedSurface`, `WrongActor`, `ScopeDenied`, `WrongSession`, `WrongUser`, `WrongClaim`, `WrongField`, `MismatchedAction`, `Expired`, `Replayed`, `Invalidated`, `ClaimVersionStale`, `CompositionVersionStale`, `RateLimited`) | live — **W4 reuses verbatim; does NOT invent new reason strings; does NOT add variants** |
| `NonceAuditContext` (audit-event builder) | `surface_nonce.rs:1158` (V1.3 correction; V1.2 cited `:1235` which is `audit_event`) | live — **W4 extends** with `attempted_wp_user_id`, `attempted_surface_client_id`, `ip_hash`, `user_agent_hash` slots |
| `SurfaceNonceStore::verify_and_consume` (consume orchestration) | `surface_nonce.rs:640` — takes `(ctx, db, digest, request, now, audit)`, returns `Result<VerifiedNonce, SurfaceNonceError>` | live — **W4 wraps this in a service-layer method that ALSO returns the binding fields W4 needs** (see §5.4) |
| `try_mark_consumed` (atomicity primitive — called inside `verify_and_consume`) | `surface_nonce.rs:518` | live — internal; W4 reaches it through `verify_and_consume` |
| `VerifiedNonce { consumed_at, nonce_digest, expected_claim_version, expected_composition_version }` | `surface_nonce.rs:583` | live — **W4 extends** with the binding-derived fields needed for `record_claim_feedback` (`claim_id`, `action`, `payload_json`, `wp_user_id`, `session_id`) |
| `validate_feedback_actor` + `actor_class_for_actor` (actor allowlist) | `claims.rs:5131` + `:5111` — head split on `:`/`/`/`@`; user allowlist is `"user"` or `"human"` | live — **W4 actor format = `"user:wp:{wp_user_id}"`** (head = `"user"`, passes allowlist) |
| `RenderPolicyChannel::ALL` (channel registry) | `src-tauri/src/bridges/types.rs:84-126` (9 variants) | live — **W4 adds `WpBlockRenders` as the 10th** |
| `record_claim_feedback` | `src-tauri/src/services/claims.rs:6700` | live — takes `&ActionDb`, opens own tx via `with_claim_transaction:6717` |
| `ClaimFeedbackInput { claim_id, action, actor, actor_id, payload_json }` | `claims.rs:228` | live — **payload_json field exists; W4 wires through** |
| `FeedbackAction` enum (9 variants) | `src-tauri/abilities-runtime/src/abilities/feedback.rs:31` | live — **W4 makes `PresenceNonceAction` match these 9** |
| `validate_feedback_payload` (normalizes payload_json) | `claims.rs:5150` | live — normalizes per-variant; does NOT pass into agent context |
| `MutationGuard` chokepoint reservation | `claims.rs:6714` (inside `record_claim_feedback`) | live |

- **Net-new in W4** (substantially smaller than V1.0 proposed):
  - **Extend `PresenceNonceAction` from 4 → 9 variants** to match `FeedbackAction` 1:1 (or add a mapping shim — see §5.1 cycle-2 reviewer call).
  - **Add `payload_json: Option<String>`** to the presence-nonce binding row + verify-time consumption (substrate-side; the existing nonce store is in-memory with 60s TTL).
  - **Add `wp_user_id: u64`** binding to the presence-nonce row at issue time; `verify_nonce` rejects if the requesting `wp_user_id ≠ stored wp_user_id`.
  - **Wire `verify_nonce` → `record_claim_feedback`** at `src-tauri/src/surface_runtime/mod.rs:2019` (the existing route handler currently consumes the nonce + emits audit; it does NOT call into claims).
  - **Atomic single-transaction**: load nonce + UPDATE consumed_at WHERE consumed_at IS NULL (rows_affected == 0 → replay rejected) + record feedback in same tx, OR `record_claim_feedback`'s row carries `idempotency_key = nonce_digest` for safe retry.
  - **Audit payload extensions**: add `wp_user_id`, `ip_hash` (HMAC-keyed at install), `user_agent_hash` to `presence_nonce_issued` / `_verified` / `_rejected`. `replay_rejected` adds `attempted_wp_user_id` + `attempted_surface_client_id`. `payload_json` user-authored fields **NEVER** in audit payloads (sensitivity=User per ADR-0108).
  - **JS feedback affordance UI** on `dailyos/account-overview` block only for v1.4.3 acceptance; v1.4.4 surface migration adds it to composites. Single-trigger button + reveal menu (NOT 9-button menu) per codex consult cycle-1 call.
  - **WP REST extension**: extend the existing `/v1/surface/nonce/{issue, verify}` shape with the additional `payload_json` field. WP plugin already calls these methods.
- **W4 unlocks:** v1.4.4 surface migration (composite + per-block feedback affordances on briefing/meeting/project surfaces); v1.4.x audit forensic validation (W6) consumes the extended `presence_nonce_*` audit payloads.

## 4. Pre-work — K-in evidence (CLAUDE.md mandate) + substrate reuse audit

**Knowledge-store grep results (`docs/solutions/` + `.docs/decisions/` + actual file:line):**

| Query | Hits | What we reuse |
|---|---|---|
| `record_claim_feedback`, `claim_feedback`, `ClaimFeedbackInput` | ADR-0123 (typed claim feedback semantics) + `services/claims.rs:228` (ClaimFeedbackInput shape) + `claims.rs:6700` (the canonical impl) | The 9-variant `FeedbackAction` enum + the `ClaimFeedbackInput { claim_id, action, actor, actor_id, payload_json }` shape + the trust/lifecycle/agent-ledger triple per variant are LOCKED. W4 does not change them. **`payload_json: Option<String>` already exists** at claims.rs:233 — W4 plumbs it from the WP user input through the nonce row to `record_claim_feedback`. |
| presence nonce, issue/verify pattern, two-phase | `src-tauri/src/services/surface_nonce.rs` (full impl, 2127 LOC) — `pub fn issue_nonce` at :106; `pub fn verify_nonce` at :218; `pub enum PresenceNonceAction` at :445 (4 variants); HKDF digest key, rate-limit budgets, in-memory 60s TTL store | **The substrate exists end-to-end.** W4 is `PresenceNonceAction` extension (4 → 9) + `payload_json` field + `wp_user_id` binding + the wire-through into `record_claim_feedback`. NOT a new service; NOT a new table; NOT a new REST endpoint. |
| HTTP route, surface/nonce, /v1/surface | `src-tauri/src/bridges/surface_client.rs:19-20` defines `SURFACE_NONCE_{ISSUE,VERIFY}_PATH`; `src-tauri/src/surface_runtime/mod.rs:2016, 2019` are the route handlers | **Routes exist.** W4 extends the handler body at :2019 (verify) to consume `payload_json` from the nonce binding + call `record_claim_feedback`. The route URL stays the same. |
| WP transport methods | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:172` (`issue_nonce()`), :188 (`verify_nonce()`), :149 (`submit_feedback()` with `presence_nonce` field) | **WP-side transport exists.** W4 adds `payload_json` to the request payload and updates `submit_feedback()` to call through. |
| `FeedbackAction`, 9-variant enum location | `src-tauri/abilities-runtime/src/abilities/feedback.rs:31` (NOT in `claims.rs`) | Each variant's metadata (claim lifecycle, trust factor change, repair behavior) at `abilities-runtime/src/abilities/feedback.rs`. W4's `PresenceNonceAction` extension mirrors the 9 variant names 1:1. |
| `MutationGuard`, `check_mutation_allowed`, write-path chokepoint | `src-tauri/src/services/claims.rs:6700` (entry) + `:6714` (`MutationGuard::reserve`) | **The chokepoint is already wired** — `record_claim_feedback` calls `ctx.check_mutation_allowed()` on entry and reserves a `MutationGuard` before touching DB. W4's verify path consumes the nonce, then calls `record_claim_feedback` — the chokepoint covers the rest. |
| WP REST scaffold, presence nonce | `wp/dailyos/includes/class-dailyos-plugin.php:520-816` (PresenceNonce REST handlers) | **The auth + permission scaffolding exists.** W4 reuses `can_issue_presence_nonce` permission + adds `payload_json` to the validated request body. |
| audit events, presence_nonce_* | `surface_nonce.rs` emits `presence_nonce_issued` / `_verified` / `_rejected` internally | **The audit infrastructure is wired.** W4 extends the existing event payloads (add `wp_user_id`, `ip_hash`, `user_agent_hash`); does NOT introduce new event types like the V1.0-proposed `pairing.feedback.*`. |
| actor-filtered provenance, sensitivity=User | ADR-0108 | `payload_json` user-authored fields (`corrected_text`, `corrected_to`, `surface` for `SurfaceInappropriate`, `invocation_id` for `NotRelevantHere`) carry sensitivity=User. Audit log NEVER stores raw text. Display projection filters per the existing W2 DOS-477 leak-guards. |
| W2 DOS-477 channel registry | `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:252` | **Baseline is 9 channels** (NOT 10 as V1.0 claimed). W4 either adds the 10th (feedback-render projection path) or documents that feedback-render is already covered by an existing channel. Cycle-1 codex challenge: V1.1 to verify by reading the channel list. |
| memory substrate invariants | ADR-0126 | "All mutations go through `services/`" (CLAUDE.md cardinal rule mirror). W4's path is WP REST → `verify_nonce` → `record_claim_feedback` — all substrate-side. |

**Net K-in conclusion:** W4 is **substrate-extension wiring**, not new-substrate authoring. The 4-to-9 `PresenceNonceAction` extension + `payload_json` plumbing + `wp_user_id` binding + the verify-to-record_claim_feedback wire-through is the entire net-new surface. Every other piece — chokepoint, nonce lifecycle, REST scaffold, audit infra, transport client — already ships.

## 5. What this packet authors

### 5.1 Extend `PresenceNonceAction` from 4 → 9 variants — LOCKED V1.2

File: `src-tauri/src/services/surface_nonce.rs:445`.

Current state (4 variants — informal user-feedback signals):

```rust
pub enum PresenceNonceAction {
    Correct,
    Dismiss,
    Corroborate,
    Contradict,
}
```

Target state (9 variants matching ADR-0123 `FeedbackAction` 1:1):

```rust
pub enum PresenceNonceAction {
    ConfirmCurrent,
    MarkOutdated,
    MarkFalse,
    WrongSubject,
    WrongSource,
    CannotVerify,
    NeedsNuance,
    SurfaceInappropriate,
    NotRelevantHere,
}
```

`as_str` / `parse` match arms updated 1:1 with `FeedbackAction::as_str` at `abilities-runtime/src/abilities/feedback.rs:65`. Plus implement `From<PresenceNonceAction> for FeedbackAction` with a compile-time exhaustive match — this is the structural gate that prevents the two enums drifting apart.

**Caller audit (cycle-2 codex consult finding):** the existing 4 variant names have ZERO callers outside `surface_nonce.rs` itself. The only production touch-point is the test-fixture surface at `surface_nonce.rs:1690`. The extension is mechanically safe; commit-1 of §10 also updates that fixture.

**In-flight nonce handling:** in-memory store, 60s TTL. Any nonce issued before the upgrade fails to parse on verify + emits `presence_nonce_rejected` with `MalformedRequest` — acceptable transient behavior; the TTL bounds the window.

### 5.2 Add `payload_json` field to the presence-nonce binding

The existing `PresenceNonceBindingFields` struct (at `surface_nonce.rs:481`) carries:

```rust
pub struct PresenceNonceBindingFields {
    surface_client_id: String,
    session_id: String,
    wp_user_id: u64,  // already present per cycle-1 K-in
    // …
}
```

Add `payload_json: Option<String>`. Issued at `issue_nonce` time from the request body; **tamper-resistant via in-memory store isolation** (the verify path reads `payload_json` from the stored binding, NEVER from the verify request body — the JS phase-2 call only supplies `nonce_digest`); forwarded to `record_claim_feedback` as `ClaimFeedbackInput::payload_json`.

**Cycle-2 cryptographic-binding clarification (folded):** V1.1 said "HKDF-bound" — that was mechanically incorrect. `PresenceNonceDigestKey::digest` at `surface_nonce.rs:421` accepts only `nonce_bytes`; binding fields are matched via tuple comparison at `compare_binding_tuple:1339`, not cryptographically mixed into the digest. The protection model is **store isolation** — the digest authenticates the nonce; the store-by-digest lookup retrieves the immutable binding (including `payload_json`); the verify path never trusts client-supplied payload. Same defense, accurate framing.

**Mandatory for these variants** (per ADR-0123 + `validate_feedback_payload` at `claims.rs:5150`):
- `WrongSource`: `payload_json` carries `{ "source_index": <int> }` — which source index is wrong.
- `NeedsNuance`: `payload_json` carries `{ "corrected_text": <string> }` — required.
- `SurfaceInappropriate`: `payload_json` carries `{ "surface": <string> }` — which surface.
- `NotRelevantHere`: `payload_json` carries `{ "invocation_id": <string> }` — which invocation.
- `WrongSubject` (optional): `payload_json` may carry `{ "corrected_to": <subject_ref> }`.

For variants that don't need a payload (`ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `CannotVerify`) — `payload_json` is `None`.

### 5.3 `wp_user_id` binding — canonical locus

`wp_user_id` is the durable WP-user identifier the binding ties to. **Cycle-2 correction:** V1.1 overclaimed W4 additions here — the binding storage AND the verify cross-check **already exist** in substrate:

- `surface_pairing.rs:189` derives `wp_user_id` from the session on issue.
- `surface_runtime/mod.rs:1946` cross-validates body/query/header `wp_user_id` against the session.
- `PresenceNonceBindingFields` at `surface_nonce.rs:485` already stores `wp_user_id`.
- `compare_binding_tuple` at `surface_nonce.rs:1339` already cross-checks `wp_user_id` on verify.
- Mismatch already emits `presence_nonce_rejected` with `PresenceNonceRejectReason::WrongUser`.

**Actual W4 additions** (much smaller scope):
- WP-side: ensure the `/dailyos/v1/nonce/verify` REST handler (registered in V1.2 §5.6) derives `wp_user_id` from the authenticated WP session — NEVER from request body.
- `NonceAuditContext` slot for `attempted_wp_user_id` so the existing `WrongUser` rejection emits forensic detail (per cycle-1 security-auditor finding #4 + cycle-2 security-auditor finding #3).

Canonical locus declaration: the binding is the source of truth; the WP layer must derive `wp_user_id` from the WP session at REST entry; the runtime cross-check already enforces the same constraint at `:1339`.

### 5.4 Wire `verify_nonce` → `record_claim_feedback` — V1.2 REWRITE

File: `src-tauri/src/surface_runtime/mod.rs:2019` (the existing route handler) and `src-tauri/src/services/surface_nonce.rs:218` (`verify_nonce` service).

**Cycle-2 convergent finding (4 reviewers):** V1.1's pseudocode framed this as a single SQL transaction wrapping `try_mark_consumed` + `record_claim_feedback`. That is structurally impossible:
- `SurfaceNonceStore` is `Mutex<HashMap<NonceDigest, PresenceNonceBinding>>` at `surface_nonce.rs:565-575` — not table-backed; no SQL row to UPDATE.
- `record_claim_feedback` at `claims.rs:6700` takes `&ActionDb` (not `&Tx`) and opens its own `with_claim_transaction` at `:6717`. Cannot be wrapped.
- The existing handler at `surface_runtime/mod.rs:2570` uses `app_state.db_read(...)` — incompatible with `record_claim_feedback`'s write requirement.

**V1.3 atomicity model — consume-then-record with fail-closed semantics, grounded against the real substrate:**

The atomicity primitive is the **Mutex on the in-memory nonce store** (not a SQL transaction). V1.3 grounds the pseudocode against the actual `verify_and_consume` API at `surface_nonce.rs:640` (V1.2 named a non-existent `try_mark_consumed(&request)` signature — cycle-3 codex challenge finding):

**Step 1 — extend BOTH `VerifiedNonce` (store) AND `SurfaceNonceVerify` (service-layer public return) to carry the binding fields W4 needs.**

V1.4 correction: V1.3 only extended `VerifiedNonce` and §16 #1 punted on the service-vs-store call-site choice. Codex challenge cycle-4 found that calling the store directly bypasses critical work in `SurfaceNonceService::verify_nonce` (ensure_surface_client, request parse, session-tuple validation, budget charging, nonce decode, audit emit at `surface_nonce.rs:218-286`). V1.4 locks the call site at the SERVICE; the store-layer struct extension is the internal plumbing.

```rust
// surface_nonce.rs:583 — store-internal extension (called inside verify_and_consume).
struct VerifiedNonce {
    // Existing fields:
    consumed_at: DateTime<Utc>,
    nonce_digest: NonceDigest,
    expected_claim_version: u64,
    expected_composition_version: u64,
    // V1.3/V1.4 additions (captured from binding before consume completes):
    claim_id: String,
    action: PresenceNonceAction,
    payload_json: Option<String>,
    wp_user_id: u64,
    session_id: String,
}

// surface_nonce.rs:1008 — public service return; V1.4 extends to expose the
// same binding-derived fields so handlers don't have to reach into the store.
pub struct SurfaceNonceVerify {
    pub consumed_at: DateTime<Utc>,
    pub request_id: String,
    pub expected_claim_version: u64,
    pub expected_composition_version: u64,
    pub audit_events: Vec<SurfaceNonceAuditEvent>,
    // V1.4 additions (mapped from VerifiedNonce by the service wrapper):
    pub claim_id: String,
    pub action: PresenceNonceAction,
    pub payload_json: Option<String>,
    pub wp_user_id: u64,
    pub session_id: String,
}
```

`verify_and_consume` at `:640` captures the binding fields into `VerifiedNonce` before `binding.try_mark_consumed(now)` is called at `:725`. The service wrapper at `:218-286` then maps `VerifiedNonce` → `SurfaceNonceVerify` (existing code at `:251-264` extended to copy the new fields). No new lock acquisition; no new function on the service surface.

**Step 2 — handler-level orchestration via the SERVICE wrapper:**

```rust
// Pseudocode for the surface_runtime/mod.rs:2570 handler — V1.4 grounded
// at the SERVICE layer (NOT the store layer; the service wrapper does
// ensure_surface_client + request parse + session-tuple validation +
// budget charging + nonce decode + audit-event construction that the
// store method skips).
fn surface_nonce_verify_response(
    app_state: &AppState,
    session: &ValidatedSurfaceSession,
    payload: Value,
    fallback_request_id: &str,
) -> Result<VerifyNonceResponse, SurfaceNonceError> {
    // V1.3 §10 commit 3 change: db_read → db_write (record_claim_feedback
    // needs a write connection).
    app_state.db_write(|db, ctx| {
        // SurfaceNonceService::verify_nonce at surface_nonce.rs:218 wraps
        // SurfaceNonceStore::verify_and_consume with:
        //   - ensure_surface_client (caller pairing + scope)
        //   - VerifyNonceRequest::parse (envelope shape + action_kind
        //     allowlist)
        //   - NonceAuditContext::from_verify (audit-context construction)
        //   - ensure_session_tuple (session_id + wp_user_id cross-check)
        //   - charge_budget(NonceBudgetClass::Verify, ...) (rate limit)
        //   - decode_presence_nonce (nonce-bytes → digest)
        //   - calls store.verify_and_consume (Mutex-guarded HashMap mutation
        //     — the atomicity primitive)
        //   - on success: builds presence_nonce_verified audit event,
        //     captures binding fields into SurfaceNonceVerify
        //   - on error: charge_failure_best_effort + presence_nonce_rejected
        // V1.4 EXTENDS SurfaceNonceVerify (at :1008) to carry the binding
        // fields needed for record_claim_feedback.
        let verified = app_state
            .surface_nonce_service
            .verify_nonce(ctx, db, session, payload, fallback_request_id)?;

        // W4 NEW: translate to ClaimFeedbackInput using fields exposed on
        // the extended SurfaceNonceVerify.
        let action: FeedbackAction = verified.action.into();
        let input = ClaimFeedbackInput {
            claim_id: verified.claim_id.clone(),
            action,
            // Actor format MUST pass validate_feedback_actor at claims.rs:5131.
            // actor_class_for_actor at :5111 splits on `:` / `/` / `@`; head
            // must be "user" or "human" for the user class. V1.3: head=user.
            actor: format!("user:wp:{}", verified.wp_user_id),
            actor_id: Some(verified.session_id.clone()),
            payload_json: verified.payload_json.clone(),
        };

        // record_claim_feedback at claims.rs:6700 — takes &ActionDb, opens
        // its own with_claim_transaction at :6717, reserves MutationGuard at
        // :6714.
        let outcome = record_claim_feedback(ctx, db, input)
            .map_err(SurfaceNonceError::Mutation)?;

        Ok(VerifyNonceResponse {
            feedback_id: outcome.feedback_id,
            new_verification_state: outcome.new_verification_state,
        })
    })
}
```

**Mutex IS the atomicity boundary.** Concurrent verify requests for the same nonce race on `verify_and_consume`'s `blocking_lock()` at `:649`; exactly one wins (returns `Ok(VerifiedNonce)`), the rest return `Err(SurfaceNonceError::rejected(PresenceNonceRejectReason::Replayed, ...))` per the existing logic at `:670`.

**Fail-closed for the consume-succeeded-record-failed path:**
- If `try_mark_consumed` succeeds but `record_claim_feedback` fails (DB error, MutationGuard rejection, validation rejection), the nonce stays consumed. Audit trail records: `presence_nonce_verified` (from `try_mark_consumed`) followed by a `record_claim_feedback` error.
- The user-facing affordance receives an error and is instructed to click again — which mints a NEW nonce. The old nonce is consumed (no replay), and the failed feedback was never written.
- Decision §6 #10 (NEW): this is the locked fail-closed semantics. NO rollback of the consume step; NO retry of the same nonce. Per `feedback_no_deferrals_period.md`, this is a real choice with a real trade-off, not a TODO.

**Handler upgrade — `db_read → db_write` at `surface_runtime/mod.rs:2570`:**
The existing verify handler wraps work in `app_state.db_read(...)`. W4 commit 3 changes that to `db_write(...)` so `record_claim_feedback` can call `with_claim_transaction`. The handler signature change is in the §10 PR shape commit 3 scope.

**Audit ordering** (existing substrate behavior, called out explicitly):
- `presence_nonce_verified` emits from inside `try_mark_consumed` after the HashMap mutation commits in-memory.
- `presence_nonce_rejected` (with `PresenceNonceRejectReason::{Replayed, WrongUser, MalformedRequest}`) emits from inside `try_mark_consumed` before the function returns.
- `claim_feedback_recorded` (emitted inside `record_claim_feedback`) lands inside the claims tx.
- Audit order on the happy path: `presence_nonce_issued` (phase 1) → `presence_nonce_verified` (phase 2 consume) → `claim_feedback_recorded` (phase 3 substrate write). On the consume-succeeded-record-failed path: `presence_nonce_issued` → `presence_nonce_verified` → `record_claim_feedback` error log (no `claim_feedback_recorded`).

**Rejection reason names — use the existing `PresenceNonceRejectReason` enum verbatim** (cycle-3 codex challenge HIGH #2 correction; V1.2's "3 variants cover all paths" was wrong — the enum has 17). W4 paths emit the existing subset; no new variants, no renaming.

| Failure mode (W4-emit subset) | `PresenceNonceRejectReason` variant | Where emitted (existing) |
|---|---|---|
| Digest not found in store (nonce invalidated) | `Invalidated` | `surface_nonce.rs:652` |
| Binding tuple mismatch (`compare_binding_tuple`) — wrong session, wrong claim, wrong field, wrong actor, action mismatch | `WrongSession`, `WrongClaim`, `WrongField`, `WrongActor`, `MismatchedAction` | inside `compare_binding_tuple` at `:1313`/`:1339`/`:1341`/etc. |
| Nonce already consumed | `Replayed` | `surface_nonce.rs:670` + `:728` |
| Nonce past TTL | `Expired` | `surface_nonce.rs:680` + `:736` |
| `wp_user_id` mismatch | `WrongUser` | inside `compare_binding_tuple` |
| Composition version stale | `CompositionVersionStale` | `surface_nonce.rs:712` + `:744` |
| Claim version stale | `ClaimVersionStale` | `surface_nonce.rs:696` |
| Malformed request envelope (bad action_kind parse, missing nonce digest, bad claim version JSON, unauthenticated, scope denied, rate-limited) | `MalformedRequest`, `MissingNonce`, `MalformedClaimVersion`, `UnauthenticatedSurface`, `ScopeDenied`, `RateLimited` | upstream of `verify_and_consume` in the route handler |

**Full enum, for AC F + §9 inv #10 enumeration:** `MalformedRequest`, `MissingNonce`, `MalformedClaimVersion`, `UnauthenticatedSurface`, `WrongActor`, `ScopeDenied`, `WrongSession`, `WrongUser`, `WrongClaim`, `WrongField`, `MismatchedAction`, `Expired`, `Replayed`, `Invalidated`, `ClaimVersionStale`, `CompositionVersionStale`, `RateLimited`. Defined at `surface_nonce.rs:1067-1085`.

W4 does NOT add variants; does NOT remove variants; does NOT rename variants. The "rejection reason naming alignment" guarantee is "use the existing names verbatim, with the full enum surface exposed in the audit event payload."

### 5.5 Audit events — align with existing `presence_nonce_*` names

**Existing events** (emitted by `surface_nonce.rs` today):
- `presence_nonce_issued` — on `issue_nonce` success.
- `presence_nonce_verified` — on `verify_nonce` success.
- `presence_nonce_rejected` — on `verify_nonce` rejection (any reason).

**W4 extensions** (payload-only — no new event types):

| Event | Payload fields added | Why |
|---|---|---|
| `presence_nonce_issued` | `wp_user_id`, `ip_hash`, `user_agent_hash`, `action_kind` (one of the 9) | Forensic trail per cycle-1 CSO finding #3. |
| `presence_nonce_verified` | `wp_user_id`, `ip_hash`, `user_agent_hash`, `claim_id`, `action_kind`, `claim_feedback_id` (from `record_claim_feedback` outcome) | Correlate the verify → record_feedback chain. |
| `presence_nonce_rejected` | `wp_user_id`, `attempted_wp_user_id`, `attempted_surface_client_id`, `rejection_reason` (one of `Replayed`, `WrongUser`, `MalformedRequest` — the existing `PresenceNonceRejectReason` enum variants) | Replay-rejection forensic completeness per cycle-1 security-auditor finding #4. |

**`NonceAuditContext` extension** (cycle-2 security-auditor finding #3): the existing struct at `surface_nonce.rs:1158` (V1.3 correction — V1.2 cited `:1235` which is `audit_event`, the event constructor) plumbs only the issued-nonce fields. V1.2 commit 2 adds slots: `attempted_wp_user_id: Option<u64>`, `attempted_surface_client_id: Option<String>`, `ip_hash: Option<String>`, `user_agent_hash: Option<String>`. The handler at `surface_runtime/mod.rs:2016/2019` reads `ip_hash` + `user_agent_hash` from request headers; the `try_mark_consumed` path captures `attempted_*` from the verify request before rejecting.

**`ip_hash` HMAC key — LOCKED V1.2** (cycle-2 CSO finding #2): derive via HKDF from the existing pairing root stored in the keychain (same key family as `PresenceNonceDigestKey` at `surface_nonce.rs:421`). Survives process restart (per CSO finding's requirement for forensic correlation across sessions). No new key-management infrastructure. L1 task: factor the HKDF derivation into a helper (e.g., `derive_audit_subkey("ip_hash")`) so future audit fields can reuse the same root without duplicating key plumbing.

**Sensitivity=User content NEVER in audit payloads** — `payload_json` user-authored fields (`corrected_text`, `corrected_to`, `surface`, `invocation_id`) stay in the `claim_feedback` row, surfaced only through actor-filtered projection per ADR-0108.

### 5.6 WP REST surface — V1.2 REWRITE

**Cycle-2 net-new findings (code-reviewer F-cycle2-3 + codex challenge C2):** V1.1 said "extend existing routes" but the actual WP-side state has gaps:
- `/dailyos/v1/nonce` (issue side) IS registered at `class-dailyos-plugin.php:573-601`.
- `/dailyos/v1/nonce/verify` is NOT registered (JS phase-2 has no WP landing zone).
- `/v1/surface/feedback` is on the runtime signed-route allowlist (`surface_runtime/mod.rs:1243, 4730, 4755`) and `runtime-client.php:163` (`submit_feedback()`) posts to it — but no Rust handler is wired. Orphan, dead path.
- WP-side `action` allowlist (`presence_nonce_payload` at `class-dailyos-plugin.php:907`) is hardcoded to the OLD 4 variants (`'correct', 'dismiss', 'corroborate', 'contradict'`) — silently rejects every new variant at the WP layer.

**V1.2 W4 changes** (commit 4 of §10):

1. **Register WP-side `/dailyos/v1/nonce/verify` REST handler.** Same auth shape as `/dailyos/v1/nonce` (permission callback reuses `can_issue_presence_nonce`-equivalent logic; derives `wp_user_id` from authenticated session; signs the runtime call via `DailyOS_Runtime_Client::verify_nonce()` at `runtime-client.php:188`). Request body: `{ nonce_digest }`. Response: `{ feedback_id, new_verification_state }` on success; `{ error_code, rejection_reason }` (using `PresenceNonceRejectReason` variant names) on failure.

2. **Update WP-side `action` allowlist at `class-dailyos-plugin.php:907`.** Replace the hardcoded 4-variant array with the 9 `FeedbackAction` variants (`confirm_current`, `mark_outdated`, `mark_false`, `wrong_subject`, `wrong_source`, `cannot_verify`, `needs_nuance`, `surface_inappropriate`, `not_relevant_here`). Mirrors the runtime-side `PresenceNonceAction::parse` after the §5.1 extension. **Without this change, AC A is structurally unreachable.** Cycle-2 security-auditor HIGH.

3. **Extend WP issue handler to accept `payload_json`.** `POST /wp-json/dailyos/v1/nonce` body adds `payload_json?: object` (validated against the variant-specific shape: `{source_index}` for `wrong_source`, `{corrected_text}` for `needs_nuance` required, `{corrected_to}` for `wrong_subject` optional, `{surface}` for `surface_inappropriate`, `{invocation_id}` for `not_relevant_here`). 500-char cap on user-authored strings.

4. **Retire the orphan `/v1/surface/feedback` runtime route + WP `submit_feedback()` transport method.** Three actions:
   - Remove `/v1/surface/feedback` from `surface_runtime/mod.rs:1243, 4730, 4755` allowlist entries.
   - Remove `submit_feedback()` from `runtime-client.php:149` (V1.3 correction — V1.2 cited `:163` which is an internal path-call line, not the def; L1 partial-deletion footgun avoided).
   - Add a CI grep gate against `/v1/surface/feedback` to prevent reintroduction (§9 invariant #12 NEW).

**JS affordance call sequence** (unchanged shape; corrected routes):
1. Phase 1 (issue): JS POSTs `{ claim_id, action_kind, payload_json? }` to `/wp-json/dailyos/v1/nonce`. WP derives `surface_client_id` + `session_id` + `wp_user_id` from the authenticated session; signs the runtime call via `DailyOS_Runtime_Client::issue_nonce()`; runtime mints the nonce + stores the binding + emits `presence_nonce_issued`. Response: `{ nonce_digest, expires_at }`.
2. Phase 2 (verify): JS POSTs `{ nonce_digest }` to `/wp-json/dailyos/v1/nonce/verify`. WP derives `wp_user_id` from session; signs the runtime call via `DailyOS_Runtime_Client::verify_nonce()`; runtime executes §5.4. Response: `{ feedback_id, new_verification_state }` on success; error code + `PresenceNonceRejectReason` on failure.

Why not a separate `/v1/feedback`: the substrate path IS the nonce path. The orphan `/v1/surface/feedback` allowlist entry is removed (item 4 above) so the dead path can't be confused for an active surface. (Closes V1.0 invented-parallel finding + V1.2 reviewer findings.)

### 5.7 JS feedback affordance — per-block UI

Added as a shared React component used by `dailyos/account-overview` only for v1.4.3 acceptance. v1.4.4 surface migration adds it to other composites.

**Component shape (per codex consult cycle-1 call — lock single-trigger):**
- Trigger button: "Feedback" (single button, NOT 9 separate buttons).
- Reveal menu: 9 entries mapping to `PresenceNonceAction` (and 1:1 with `FeedbackAction`).
- Conditional textarea: appears for `NeedsNuance` (required, 500 char cap), `WrongSubject` (optional, 500 char cap), `WrongSource` (with a source-index picker derived from the rendered claim's source citations).
- Conditional surface picker: appears for `SurfaceInappropriate` (current rendering surface auto-detected; user confirms).
- Conditional invocation picker: appears for `NotRelevantHere` (current `invocation_id` auto-detected).
- Confirm / Cancel buttons.
- Loading state during the issue → verify roundtrip.
- Success state: claim re-renders with the updated trust band / lifecycle / withdrawn marker.
- Error state: shows the rejection reason from the substrate (`already_consumed`, `expired`, `wp_user_mismatch`, etc.).

CSS: per-block `style.css` + plugin-owned baseline tokens (no inline CSS per memory `feedback_no_inline_css.md`).

### 5.8 Wire-up — end-to-end fixture

**Test sequencing per codex consult cycle-1:** in-process runtime + handler fixture (NOT a full Tauri macOS app boot). PHP REST tests can mock the runtime client; Rust tests use the actual `record_claim_feedback` path with a real DB.

`src-tauri/tests/dos683_w4_feedback_end_to_end_test.rs`:

1. Stand up an in-process runtime instance with a real SQLite DB seeded with one `LikelyCurrent` claim.
2. Simulate JS phase 1: HTTP POST `/v1/surface/nonce/issue` with `{ surface_client_id, session_id, claim_id, action_kind: "mark_outdated" }`. Receive `nonce_digest`.
3. Simulate JS phase 2: HTTP POST `/v1/surface/nonce/verify` with `{ nonce_digest }`. Runtime executes §5.4.
4. Re-render: claim now shows `superseded` lifecycle + `UseWithCaution` (or `NeedsVerification`) trust band per ADR-0123 §1.
5. Audit log contains: `presence_nonce_issued → presence_nonce_verified → claim_feedback_recorded` (V1.3 correction — V1.2 had the order reversed; `presence_nonce_verified` fires inside `verify_and_consume`'s Mutex-guarded HashMap mutation BEFORE `record_claim_feedback` is called).
6. Replay: HTTP POST `/wp-json/dailyos/v1/nonce/verify` with the same `nonce_digest`. Runtime rejects + emits `presence_nonce_rejected` with `PresenceNonceRejectReason::Replayed` + `attempted_wp_user_id` + `attempted_surface_client_id`. Re-render unchanged.

### 5.9 `RenderPolicyChannel` extension — add `WpBlockRenders` — V1.2 NEW

File: `src-tauri/src/bridges/types.rs:84-126`.

**Cycle-2 codex consult finding #2:** `RenderPolicyChannel::ALL` has exactly 9 variants today. None covers WP block rendering. §8.9 (`FeedbackPayloadRedactionTest.php`) and §9 invariant #4 (no raw `payload_json` in audit) cannot be tested to a correct policy contract until WP block rendering is classified.

**V1.2 change** (1 enum entry + 1 ALL element):

```rust
// src-tauri/src/bridges/types.rs
pub enum RenderPolicyChannel {
    // … existing 9 variants …
    WpBlockRenders,
}

impl RenderPolicyChannel {
    pub const ALL: &'static [RenderPolicyChannel] = &[
        // … existing 9 entries …
        RenderPolicyChannel::WpBlockRenders,
    ];
}
```

**V1.4 substrate re-verification (REVERSES V1.3's wrong read).** Codex challenge cycle-4 found that V1.3 mis-read the substrate state:

- `#[non_exhaustive]` IS on `RenderPolicyChannel` at `bridges/types.rs:81`. V1.3 said it isn't. Wrong.
- `RenderPolicyChannel::all()` IS consumed by `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs:359, 372, 378, 654` — the bundle-17 test asserts `RenderPolicyChannel::all().len() == 9`, iterates all variants for matrix-row coverage, and pins each channel name in fixtures. V1.3 said zero consumers. Wrong.
- The W6-E gate IS in place; auto-application IS real.

**W4 §5.9 actual disposition (V1.4):**

Adding `WpBlockRenders` to `RenderPolicyChannel::ALL` will fail the bundle-17 sweep test at `:359` (`len == 9` assertion) and the matrix-row coverage at `:372, :378, :654` until both the test and the bundle-17 fixtures are updated. §10 commit 5 (V1.4-expanded) must include:

1. `bundle-17` sweep test update: `len == 9` → `len == 10`.
2. `bundle-17/metadata.json` channel-row addition for `wp_block_renders`.
3. Any matrix-row fixtures (`expected_output.json`, `expected_state.json`, `expected_provenance.json`) that enumerate channels — add the new row with appropriate W4-suitable expected values.
4. §8.9 PHPUnit (`FeedbackPayloadRedactionTest.php`) remains in scope as the WP-side asserted redaction; the bundle-17 sweep is the substrate-side compile-time + fixture-time gate.

The combination provides BOTH compile-time exhaustiveness (via `#[non_exhaustive]` + the bundle-17 sweep) AND runtime redaction assertion (via §8.9 PHPUnit). V1.3's "manual gate only + W6-E backfill maintenance" disposition was wrong — V1.4 corrects.

Closes V1.1 §12 open question #1 (channel count = 9 today; `WpBlockRenders` becomes the 10th in W4 commit 5).

## 6. Decisions to lock at L0

1. **All write paths go through `services::claims::record_claim_feedback`.** The WP REST handler is a thin envelope: validates request, signs the runtime call, relays the response. No DB writes from WP. (Locks ADR-0111 + ADR-0126 mirror.)
2. **`record_claim_feedback` is consumed unchanged.** W4 does not modify the function body or the `FeedbackAction` enum. **CI gate (narrowed):** PR must not edit `src-tauri/src/services/claims.rs` lines containing `fn record_claim_feedback` through its closing brace, AND must not edit `src-tauri/abilities-runtime/src/abilities/feedback.rs` `enum FeedbackAction` block. (V1.0 had a blanket "no edits to `claims.rs` non-test" which is too broad — `claims.rs` has 100+ legitimate edit paths from W2+; the gate must target the specific function + enum.)
3. **Two-phase nonce.** No single-shot path. Every feedback write requires `issue` THEN `verify` from the same actor on the same `surface_client_id` AND `wp_user_id`.
4. **Replay-rejection is non-negotiable.** Consumed nonce can never be consumed again; rejection is recorded as `presence_nonce_rejected` audit event with `rejection_reason = PresenceNonceRejectReason::Replayed` (V1.4 correction — V1.1 used the invented string `"already_consumed"`).
5. **`payload_json` user-authored fields sensitivity=User.** NEVER appear in audit event payloads. Redaction enforced at the projection layer per W2 DOS-477 leak guards (V1.1 verifies the channel list at cycle 2 — see §12).
6. **60s default TTL on un-verified nonces.** Existing presence-nonce behavior; W4 does NOT introduce a longer TTL. Rationale: feedback affordances are click-bound; the user is right there. Longer TTL widens the replay window without UX benefit. (V1.0 proposed 24h TTL; cycle-1 rejected because the presence nonce is in-memory not table-backed.)
7. **W4 ships affordance UI ONLY on `dailyos/account-overview`** for v1.4.3 acceptance. Other W2 primitive blocks + composites get affordance UI in v1.4.4 surface migration.
8. **`wp_user_id` is canonical and server-derived.** Stored in the presence-nonce binding at issue; cross-checked at verify; rejected with `PresenceNonceRejectReason::WrongUser` on any disagreement (V1.4 correction — V1.1 used the invented string `"wp_user_mismatch"`). Never trust JS-supplied `wp_user_id`.
9. **NEW: feedback flow has no migrations.** The existing nonce store is in-memory with 60s TTL. If a feedback flow ever needs durability beyond 60s (offline retry, audit forensics), that's an ADR amendment requiring explicit `/cso` sign-off — NOT a packet decision. (V1.0 proposed migrations v181/v182/v183 — DELETED in V1.1.)
10. **NEW: replay rejection is fail-closed.** Any ambiguous state (concurrent verify race, partial-commit mid-flight, stored nonce row corruption) MUST reject with `presence_nonce_rejected` rather than silently succeed.
11. **V1.2 NEW + V1.3 corrected: rejection_reason names use the existing `PresenceNonceRejectReason` enum verbatim.** No invented strings; no new variants; no renames. V1.2 incorrectly claimed only 3 variants cover all W4 paths — the enum has 17 (per §3 substrate table + §5.4 table). W4 emits the existing subset that `verify_and_consume` already produces (`Invalidated`, `Replayed`, `Expired`, `WrongUser`, `WrongClaim`, `WrongField`, `MismatchedAction`, `ClaimVersionStale`, `CompositionVersionStale`, plus upstream `MalformedRequest`, `MissingNonce`, `MalformedClaimVersion`, `UnauthenticatedSurface`, `ScopeDenied`, `WrongActor`, `WrongSession`, `RateLimited`). AC E, F, §8.3, §8.4 reference the enum surface, not a 3-variant subset.
12. **V1.2 NEW: forward-constraint for LLM-injection on `corrected_text`.** Current state: `validate_feedback_payload` at `claims.rs:5150` normalizes `payload_json` but does NOT pass into agent context. **Constraint for future work** (any v1.4.4+ repair ability consuming `corrected_text` into an LLM prompt): the sensitivity=User filter (per ADR-0108) MUST apply BEFORE prompt construction; the filter implementation lives at the projection layer that already exists for W2 DOS-477 channels. This is a constraint, not an implementation; the constraint goes in §9 invariants as a regression-prevention test scaffold.
13. **V1.2 NEW: consume-succeeded-record-failed is a real path, not a TODO.** The fail-closed atomicity model (§5.4) means the nonce stays consumed even if `record_claim_feedback` rejects. The user-facing affordance receives an error; the user retries with a NEW nonce. No rollback of the consume. No retry of the same nonce. Per `feedback_no_deferrals_period.md`.

## 7. Visual parity matrix

L4 captures: 9 affordance states (one per `FeedbackAction` variant) + 3 chrome states (loading, success, replay-rejected) on the `dailyos/account-overview` block. ~12 screenshot pairs total. Parked end-of-batch per the W2/W3 L4 batching protocol.

## 8. Test/fixture spec

### 8.1 Rust unit: PresenceNonceAction parses all 9 variants
`src-tauri/src/services/surface_nonce_action_test.rs` — round-trip `as_str` ↔ `parse` for all 9 variants; rejects unknown.

### 8.2 Rust integration: nonce lifecycle with payload_json
`src-tauri/abilities-runtime/tests/surface_nonce_payload_json_lifecycle.rs` — issue with payload → verify → ClaimFeedback row carries payload_json end-to-end.

### 8.3 Rust integration: replay rejection (atomicity)
`src-tauri/abilities-runtime/tests/surface_nonce_replay_atomic.rs` — issue → verify → verify-again rejected; audit emits `presence_nonce_rejected` with `rejection_reason: "already_consumed"` + `attempted_wp_user_id`.

### 8.4 Rust integration: wp_user_id binding rejection
`src-tauri/abilities-runtime/tests/surface_nonce_wp_user_mismatch.rs` — issue from `wp_user_id=A` → verify with `wp_user_id=B` rejected; audit emits `presence_nonce_rejected` with `rejection_reason: "wp_user_mismatch"`.

### 8.5 Rust integration: TTL expiry rejection
`src-tauri/abilities-runtime/tests/surface_nonce_expired.rs` — issue → wait past 60s → verify rejected; audit emits `presence_nonce_rejected` with `rejection_reason: "expired"`.

### 8.6 Rust integration: full feedback round-trip
`src-tauri/tests/dos683_w4_feedback_end_to_end_test.rs` — §5.8 end-to-end fixture exercising 4 representative variants (`MarkOutdated`, `MarkFalse`, `WrongSubject` with `corrected_to`, `NeedsNuance` with `corrected_text`).

### 8.7 PHPUnit: REST endpoint permissions
`wp/dailyos/tests/SurfaceNonceFeedbackEndpointTest.php` — permission callback rejects unauthenticated; requires pairing marker; `surface_client_id` must match active session.

### 8.8 PHPUnit: REST endpoint input validation
`wp/dailyos/tests/SurfaceNonceFeedbackInputTest.php` — `action_kind` allowlist (rejects unknown); `payload_json` shape per variant (rejects malformed); 500-char cap on user-authored text fields.

### 8.9 PHPUnit: payload_json redaction at projection
`wp/dailyos/tests/FeedbackPayloadRedactionTest.php` — user-authored fields in `payload_json` NEVER returned to a non-originating actor. Cross-references the W2 DOS-477 channel list (V1.1 cycle-2 verifies the channel-count baseline).

### 8.10 Audit log forensic test
`src-tauri/abilities-runtime/tests/surface_nonce_audit_correlation.rs` — for a happy-path feedback, verify the chain `presence_nonce_issued → presence_nonce_verified → claim_feedback_recorded` (V1.3 correction — `presence_nonce_verified` fires inside `verify_and_consume` BEFORE `record_claim_feedback`) lands with consistent `request_id` correlation + the audit payload extensions from §5.5 are present.

## 9. Invariants (CI-enforced)

1. **No DB writes from WP outside `services::claims::record_claim_feedback`.** Already enforced by existing `raw_wpdb_outside_services` gate.
2. **No `record_claim_feedback` body modifications in W4 PR.** Diff gate: PR must not edit the lines spanning `pub fn record_claim_feedback` through its closing brace at `src-tauri/src/services/claims.rs:6700+`. Also must not edit `pub enum FeedbackAction` block at `src-tauri/abilities-runtime/src/abilities/feedback.rs:31`.
3. **`verify_nonce` MUST pass through `record_claim_feedback`** (which enters `ctx.check_mutation_allowed()`). Grep gate on `services/surface_nonce.rs` verify path: must contain `record_claim_feedback` call.
4. **No raw user-authored `payload_json` content in audit event payloads.** Grep gate: audit-event writer callers in `surface_nonce.rs` must not pass `nonce.payload_json` through.
5. **REST endpoint MUST use `DailyOS_Runtime_Client` for runtime calls.** No raw `wp_remote_post` to the runtime sentinel from feedback handler. (Already enforced by existing `wp_remote_post_body_array` gate.)
6. **Replay-rejection MUST emit `presence_nonce_rejected` with `rejection_reason = PresenceNonceRejectReason::Replayed`** (V1.4 correction — V1.1/V1.2 used invented string `"already_consumed"`). Verified by §8.3 integration test.
7. **`wp_user_id` mismatch MUST emit `presence_nonce_rejected` with `rejection_reason = PresenceNonceRejectReason::WrongUser`** (V1.4 correction — V1.1/V1.2 used invented string `"wp_user_mismatch"`). Verified by §8.4 integration test.
8. **`PresenceNonceAction` MUST have exactly 9 variants matching `FeedbackAction` 1:1.** Compile-time check via `From<PresenceNonceAction> for FeedbackAction` impl + exhaustive match.
9. **WP-side `action` allowlist at `class-dailyos-plugin.php:907` MUST contain all 9 variant names.** Grep gate: count entries; fail if ≠ 9 OR if any of the old 4 strings (`correct`, `dismiss`, `corroborate`, `contradict`) appear. Verified by §8.8.
10. **`NonceAuditContext` MUST carry `attempted_wp_user_id`, `attempted_surface_client_id`, `ip_hash`, `user_agent_hash` slots.** Grep gate on the struct definition at `surface_nonce.rs:1158` (V1.4 correction — V1.2 cited `:1235` which is `audit_event` constructor). Verified by §8.10 audit forensic test.
11. **`RenderPolicyChannel::ALL` MUST include `WpBlockRenders`.** Compile-time check via the `#[non_exhaustive]` attribute at `bridges/types.rs:81` + the bundle-17 sweep consumer at `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs:359` (asserts `RenderPolicyChannel::all().len() == 10` after V1.4 commit 5) + matrix-row iteration at `:372, :378, :654`. (V1.3 wrongly claimed this gate doesn't exist; V1.4 reverses.)
12. **NEW: orphan `/v1/surface/feedback` MUST NOT reappear.** Grep gate: fail if `/v1/surface/feedback` appears in `src-tauri/src/surface_runtime/` or `wp/dailyos/includes/`. Prevents reintroduction of the dead path after V1.2 retirement.
13. **No customer-specific data in test fixtures.** CLAUDE.md rule. Use `acct-test-001`, `claim-test-001` generic IDs.
14. **No PII in commit messages.** CLAUDE.md rule.
15. **L2-status on every code commit.** CLAUDE.md rule + commit-msg hook.

## 10. PR shape

W4 ships as one PR (PR-F1) to `dev`, multi-commit for L2 reviewability, squash-merge at landing. Ordered per codex consult cycle-1 sequencing + V1.2 cycle-2 additions:

1. **`PresenceNonceAction` extension (4 → 9)** + `From<PresenceNonceAction> for FeedbackAction` compile-time exhaustive match + update test fixture caller at `surface_nonce.rs:1690` + Rust unit test (§8.1).
2. **`surface_nonce.rs` `payload_json` field on `PresenceNonceBindingFields`** + `NonceAuditContext` slot additions (`attempted_wp_user_id`, `attempted_surface_client_id`, `ip_hash`, `user_agent_hash`) + `ip_hash` HKDF subkey derivation from pairing root + audit payload extensions per §5.5 + Rust integration tests (§8.2, §8.4, §8.5).
3. **`verify_nonce` → `record_claim_feedback` wire-through** at `surface_runtime/mod.rs:2019` (`db_read → db_write` upgrade at `:2570`) + consume-then-record fail-closed atomicity model (§5.4) + replay-rejection test (§8.3) + audit correlation test (§8.10) + retire orphan `/v1/surface/feedback` allowlist entries at `mod.rs:1243, 4730, 4755` + retire `runtime-client.php:149` `submit_feedback()` (V1.4 correction — V1.2 cited `:163` which is internal path-call).
4. **WP REST endpoint changes**: register `/dailyos/v1/nonce/verify` handler at `class-dailyos-plugin.php` (signs `verify_nonce()` transport call) + update WP-side `action` allowlist at `:907` to 9 variants (cycle-2 security-auditor HIGH) + extend issue handler with `payload_json` variant-shape validation + PHPUnit (§8.7, §8.8, §8.9).
5. **`RenderPolicyChannel::WpBlockRenders` extension** at `bridges/types.rs` (add 1 enum variant + bump `ALL` from `[Self; 9]` to `[Self; 10]` with the new entry) + bundle-17 sweep test update at `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs:359` (`len == 9` → `len == 10`) + bundle-17 fixture updates (`metadata.json` channel-row, plus any `expected_output.json` / `expected_state.json` / `expected_provenance.json` matrix-row additions). V1.4 correction — V1.3 wrongly thought no consumer existed.
6. **JS feedback affordance component** + style.css + integration into `account-overview` block.
7. **End-to-end fixture** + L4 visual parity matrix capture (§8.6).

## 11. Reviewer matrix

Per wave-plan §W4 + CLAUDE.md L0 Amendment 3 (nonce lifecycle + write path + trust-boundary):

- `/codex challenge` (adversarial probe — L0 + L2)
- `code-reviewer` (Rust + PHP correctness)
- `/codex consult` (sequencing + cross-layer integration)
- `/cso` — **MANDATORY** (nonce lifecycle + write path + new trust boundary surface)
- `security-auditor` — **MANDATORY** (per Amendment 3 — claim/provenance/write-path)

**K-in obligation per CLAUDE.md:** reviewers grep `docs/solutions/` + `.docs/decisions/` AND inspect actual file:line paths in §3 / §4 BEFORE scoring. V1.1 K-in evidence in §4 is grounded by file:line — reviewers verify the paths.

## 12. Open questions

1. ~~**Channel-count baseline for the leak-guard registry.**~~ **CLOSED V1.2** — codex challenge + codex consult verified at `bridges/types.rs:84-126`: `RenderPolicyChannel::ALL` has 9 variants today. WP block rendering NOT covered. V1.2 §5.9 adds `WpBlockRenders` (1 enum entry + 1 ALL element).

2. ~~**`PresenceNonceAction` extension vs mapping shim.**~~ **CLOSED V1.2** — codex consult cycle-2 grep confirmed: the existing 4 variant names have ZERO callers outside `surface_nonce.rs` itself (only fixture touch at `:1690`). Mapping shim has zero justification. V1.2 §5.1 locks extend-to-9.

3. **`SurfaceInappropriate` + `NotRelevantHere` — surface payload derivation.** Both variants take an environmental marker (which surface / which invocation). Should the JS auto-detect the value from the rendering context, or ask the user to select? **Recommended:** auto-detect; show the detected value as confirmation; user can override before confirm. **Cycle-3 lockable.**

4. **JS bundle size impact.** Adding the 9-action menu + textarea + variant-conditional editors (surface picker, invocation picker, source-index picker) to the account-overview block adds ~8-12 KB. Stays under the existing per-block budget. **Recommended:** ship; revisit at L4 if bundle audit catches a regression.

5. **V1.2 NEW: `validate_feedback_payload` shape mirroring.** The substrate validator at `claims.rs:5150` enforces variant-specific `payload_json` shapes. The WP-side validation at `class-dailyos-plugin.php` MUST mirror the same shapes (cycle-2 security-auditor finding implication). Open question: do we duplicate the shape spec in PHP, or call a runtime validation endpoint before mint? **Recommended:** duplicate in PHP (small surface area; fast feedback for the user) + add a parity test that asserts the PHP allowlist matches the Rust enum. Cycle-3 lockable.

6. **V1.2 NEW: handler-vs-service `db_read → db_write` upgrade granularity.** §10 commit 3 changes the verify handler at `mod.rs:2570` from `db_read` to `db_write`. Question: does the existing `surface_nonce_verify_response` codepath have OTHER read-only invocations that lose their `db_read` optimization when promoted to `db_write`? L1 task to grep callers + audit. **Recommended:** if the codepath is only the verify route, promote in place; if shared, split. Cycle-3 lockable.

## 13. v1.4.4+ + backlog lineage

| Item | Where it lives now | What v1.4.x / backlog does |
|---|---|---|
| Feedback affordance on briefing/meeting/project composites | W4 ships on `account-overview` only | v1.4.4 surface migration (DOS-702 entity sync architecture lands first; affordances follow per composite). |
| Per-claim action filtering (some claim types support fewer variants) | W4 ships all 9 variants for every claim | v1.4.4 surface migration — adds per-claim-type action allowlist. |
| Plugin admin: presence-nonce sweep health panel | not in W4 | v1.4.4 admin UX. |
| MCP surface: feedback writes via ability invocation (not just click-bound) | not in W4 | v1.5.x — feedback-as-ability requires re-thinking actor-attribution per ADR-0111. |
| Causal lineage feedback signals → claim trust | partial via `record_claim_feedback`'s existing signal emission | v1.5.x — full causal lineage substrate (per memory `project_causal_lineage_deferred.md`). |
| Recommendations layer driven by feedback signals | not in W4 | v1.5.x (per memory `project_recommendations_layer_vision.md`). |
| Persistent / durable feedback queue (offline replay, audit forensics) | not in W4 (60s in-memory TTL) | ADR-amendment territory; not in v1.4.x scope without explicit `/cso` sign-off. |

## 14. Acceptance criteria

A. **`PresenceNonceAction` has exactly 9 variants matching `FeedbackAction` 1:1.** `as_str` ↔ `parse` round-trip for all 9. Compile-time exhaustive match in `From<PresenceNonceAction> for FeedbackAction`.

B. **Presence-nonce binding carries `payload_json: Option<String>`** at issue time; **tamper-resistant via in-memory store isolation** (verify reads `payload_json` from the stored binding, NEVER from request body); forwarded to `record_claim_feedback` at verify.

C. **`wp_user_id` binding + cross-check work end-to-end.** WP REST handlers derive `wp_user_id` from the authenticated WP session at entry (never from request body). The existing binding storage (`surface_nonce.rs:485`) + verify check (`:1339`) reject mismatch as `PresenceNonceRejectReason::WrongUser`.

D. **`verify_nonce` → `record_claim_feedback` wire-through.** Successful verify creates one `claim_feedback` row + transitions the claim per ADR-0123 §1. Mutex on `SurfaceNonceStore` is the consume-step atomicity primitive; `MutationGuard::reserve` inside `record_claim_feedback` is the substrate write chokepoint. Handler at `mod.rs:2570` uses `db_write` (NOT `db_read`).

E. **Replay rejection atomic via Mutex.** Two concurrent verify calls with the same nonce: exactly one's `try_mark_consumed` returns `Ok(binding)`, the rest return `PresenceNonceRejectReason::Replayed`. Verified by §8.3.

F. **Audit payload extensions land.** `presence_nonce_issued` / `_verified` / `_rejected` carry `wp_user_id` + `ip_hash` + `user_agent_hash` (+ correlation-specific fields per §5.5). `presence_nonce_rejected` carries `attempted_wp_user_id` + `attempted_surface_client_id` + `rejection_reason` — accepts ANY `PresenceNonceRejectReason` variant from the full enum at `surface_nonce.rs:1067-1085` (17 variants; V1.3 correction). `NonceAuditContext` struct at `surface_nonce.rs:1158` (V1.3 correction) extended with the new slots. User-authored `payload_json` content NEVER in audit payloads.

G. **WP REST works for both phases.** `/dailyos/v1/nonce` (issue) + `/dailyos/v1/nonce/verify` (NEW, registered in V1.2 §5.6) both work. WP-side `action` allowlist at `class-dailyos-plugin.php:907` updated to 9 variants. `payload_json` validates per variant.

H. **JS feedback affordance renders on `dailyos/account-overview`.** All 9 actions selectable; conditional textarea / surface-picker / invocation-picker per variant; single-trigger + reveal menu.

I. **End-to-end fixture passes for 4 representative variants** (`MarkOutdated`, `MarkFalse`, `WrongSubject` with payload, `NeedsNuance` with payload).

J. **`payload_json` user-authored fields never leak to non-originating actors.** PHPUnit at §8.9 asserts redaction is applied on the WP block-render surface (`WpBlockRenders` channel — V1.2 §5.9). V1.3 correction: the W6-E `#[non_exhaustive]` channel-sweep auto-gate doesn't exist today (W6-E planned but not built); §8.9 is the manual coverage gate for v1.4.3. W6-E backfill filed as maintenance.

K. **`/v1/surface/feedback` orphan is retired.** Removed from `surface_runtime/mod.rs:1243, 4730, 4755` allowlist. `runtime-client.php:149` `submit_feedback()` definition deleted (V1.4 correction — V1.2 cited `:163` which is internal path-call line). §9 invariant #12 grep gate prevents reintroduction.

L. **CI gates pass.** §9 invariants 1–15 enforced. Critically: `record_claim_feedback` body + `FeedbackAction` enum unchanged; WP action allowlist count = 9; `NonceAuditContext` slots present; `RenderPolicyChannel::WpBlockRenders` in `ALL`; orphan `/v1/surface/feedback` absent.

M. **L2 unanimous APPROVE** per §11 reviewer matrix, bounded by AC A–L.

N. **L4 hands-on** — 12 screenshot pairs captured (parked end-of-batch).

O. **CSO + security-auditor APPROVE** (mandatory L0 + L2 review per Amendment 3).

P. **No regressions in W2 primitive blocks, W3 magazine theme, or any existing presence-nonce caller.** The orphan `submit_feedback()` retirement (AC K) requires confirming no production caller exists — V1.2 grep confirmed no callers outside the dead path itself.

## 15. Lock criteria

Cycle 1 reviewer outputs (V1.0): 2 BLOCK (code-reviewer, codex challenge) + 3 CONDITIONAL APPROVE (codex consult, CSO, security-auditor). V1.1 folded all 14 convergent findings.

Cycle 2 reviewer outputs (V1.1): **5/5 CONDITIONAL APPROVE; 0 BLOCK; 0 class-pattern recurrence.** Cycle-2 verdicts captured in `.docs/plans/v1.4.3-wp-foundation/reviews/packet-F-{code-reviewer, codex-challenge, codex-consult, cso, security-auditor}-cycle2.md`. V1.2 folds 10 surgical conditions (convergent §5.4 atomicity rewrite + 9 net-new findings) + 2 reviewer calls closed (extend-to-9 + channels=9+add-WpBlockRenders).

Cycle 3 reviewer outputs (V1.2): 3 APPROVE (code-reviewer, /cso, security-auditor) + 2 CONDITIONAL APPROVE (codex consult LOW; codex challenge 2 HIGH + 1 MEDIUM + 2 LOW). V1.3 folded the 6 conditions. Output at `.docs/plans/v1.4.3-wp-foundation/reviews/packet-F-*-cycle3.md`.

Cycle 4 reviewer outputs (V1.3): 1 APPROVE (code-reviewer) + 1 APPROVE-with-advisory (/cso) + 3 CONDITIONAL APPROVE (codex consult LOW×2 + advisory; security-auditor MEDIUM blocking + LOW advisory; codex challenge 2 structural + 3 consistency). All marked "strictly surgical V1.4 patch; no cycle-5 reviewer set needed." V1.4 folds the 11 conditions (2 structural + 9 text). Output at `.docs/plans/v1.4.3-wp-foundation/reviews/packet-F-*-cycle4.md`.

**V1.4 LOCKED 2026-05-19.** Per codex challenge cycle-4 ("strictly surgical V1.4 patch; no cycle-5 reviewer set needed") + codex consult cycle-4 ("does not block lock if folded in V1.4") + security-auditor cycle-4 ("Once applied, upgrades to APPROVE"). All cycle-4 CONDITIONAL conditions folded; cycle-5 reviewer dispatch not required.

Pack locks when:
- Cycle 2 verdict set is unanimous APPROVE *or* unanimous-equivalent (≤2 reviewers CONDITIONAL with strictly surgical conditions folded into V1.x; 0 BLOCK; 0 active class-pattern).
- All folded conditions verifiable from packet text alone.
- No reviewer dissent on critical findings (per memory `feedback_reviewer_dissent_is_signal.md`).
- CSO + security-auditor unanimous APPROVE (mandatory gates).
- Cycle-2 confirms the §5.1 reviewer call (extend `PresenceNonceAction` 4 → 9 vs mapping shim) — V1.1 specifies extend; reviewers confirm or surface a stronger mapping-shim argument.
- §12 open question #1 resolved with a verified channel count (read the registry, don't claim).

Per `feedback_review_loop_l6_policy.md`: 15-cycle hard cap; class-pattern sweeps at 2-similar-findings; convergence rule allows surgical CONDITIONALs to lock.

## 16. L1 implementation notes (V1.3 appendix)

Captured from cycle-3 reviewer FYIs that don't change packet text but are load-bearing for L1 authoring:

1. **§5.4 noun chain.** `verify_and_consume` is on `SurfaceNonceStore` (private struct) at `surface_nonce.rs:640`, not directly on `SurfaceNonceService`. L1 reaches it through `app_state.surface_nonce_store` or a service-level wrapper. The W4 changes are in commit 3 scope. (code-reviewer cycle-3 FYI #1)

2. **§5.9 fixed-size array.** Existing `ALL: [Self; 9]` is `[Self; N]` with explicit `N = 9`. Adding `WpBlockRenders` requires `[Self; 10]` (or a refactor to a slice). L1 picks the smallest change; keep fixed-size for `len()` const evaluation. (code-reviewer cycle-3 FYI #2)

3. **`From<PresenceNonceAction> for FeedbackAction` crate boundary.** `PresenceNonceAction` is crate-private in `src-tauri`; `FeedbackAction` is in `src-tauri/abilities-runtime` (a separate crate). The `From` impl needs either a `pub` promotion of `PresenceNonceAction` OR the impl lives in a third crate that depends on both OR the impl lives in `surface_nonce.rs` using a re-exported type. L1 picks the smallest visibility change. (code-reviewer cycle-3 FYI #3)

4. **Deployment ordering for orphan retirement.** §5.6 step 4 retires `/v1/surface/feedback` allowlist + `submit_feedback()` in the same commit (commit 3 + commit 4 atomic-ish). If the macOS runtime build and the WP plugin build ship at different times in production, a window exists where old WP code POSTs to the dead route → silent feedback loss. For local-to-local dev (the current model) this is moot; for any future remote deployment it's a real coordination concern. L1 notes for ops. (/cso cycle-3 advisory)

5. **Phase-3 `record_claim_feedback` failure charging.** The existing rate-limit budget `failure_budget_per_minute: 240` at `surface_nonce.rs:30-52` is charged by `charge_failure_best_effort` at `:273` when nonce-issue or verify fails. Phase-3 (record_claim_feedback failure after a successful consume) is NOT currently charged. An attacker could intentionally exhaust nonces by triggering record_claim_feedback failures (e.g., via malformed `payload_json` that passes WP shape check but fails Rust `validate_feedback_payload`). L1 adds the failure charge on the phase-3 Err path. (/cso cycle-3 advisory)

6. **HKDF `info` parameter for `ip_hash` MUST be purpose-distinct.** The existing `PRESENCE_NONCE_KEY_INFO = b"dailyos.surface.presence_nonce.digest.v1"` is the info bytes for the nonce-digest key. The `ip_hash` derivation MUST use a different info string (suggested: `AUDIT_IP_HASH_KEY_INFO = b"dailyos.surface.audit.ip_hash.v1"`) so the two keys are cryptographically isolated. Single-constant decision; L1. (security-auditor cycle-3 advisory)

7. **`/dailyos/v1/nonce/verify` permission callback — literal reuse, not "equivalent".** §5.6 step 1 says "reuses `can_issue_presence_nonce`-equivalent logic". L1 **MUST** use the existing `can_issue_presence_nonce` callback directly (V1.4 — security-auditor cycle-4 upgraded "SHOULD" to "MUST" to prevent L1 from writing a weaker parallel callback). The callback checks `is_user_logged_in()`, `current_user_can('edit_posts')`, and `is_paired()`; the verify endpoint requires all three.

8. **WP `payload_json` validation — reject non-plain-object values.** §5.6 step 3 says variant-specific shape validation. L1 should additionally reject arrays, deeply-nested objects, or values that aren't plain key-value maps before forwarding to runtime. Defensive measure against JSON injection / shape-confusion attacks. (security-auditor cycle-3 advisory)
