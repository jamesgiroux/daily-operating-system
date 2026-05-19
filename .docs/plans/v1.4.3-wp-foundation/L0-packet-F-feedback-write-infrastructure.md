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
| HTTP route `POST /v1/surface/nonce/issue` | `bridges/surface_client.rs:19` + `surface_runtime/mod.rs:2016` | live |
| HTTP route `POST /v1/surface/nonce/verify` | `bridges/surface_client.rs:20` + `surface_runtime/mod.rs:2019` | live |
| WP transport `issue_nonce()` | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:172` | live |
| WP transport `verify_nonce()` | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:188` | live |
| WP REST scaffold (PresenceNonce) | `wp/dailyos/includes/class-dailyos-plugin.php:520-816` | live |
| `record_claim_feedback` | `src-tauri/src/services/claims.rs:6700` | live |
| `ClaimFeedbackInput { claim_id, action, actor, actor_id, payload_json }` | `claims.rs:228` | live — **payload_json field exists; W4 wires through** |
| `FeedbackAction` enum (9 variants) | `src-tauri/abilities-runtime/src/abilities/feedback.rs:31` | live — **W4 makes `PresenceNonceAction` match these 9** |
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

> **Cycle-2 reviewer call:** The biggest single design decision is **§5.1 below** — extend `PresenceNonceAction` 4 → 9 with `payload_json` per variant **versus** add a mapping shim at the `verify_nonce` boundary that translates the existing 4 actions into the 9 substrate actions. James's lean: **extend to 9** (the substrate enum is in `abilities-runtime` already and used by `record_claim_feedback` directly; mapping adds an extra layer for cosmetic-only "presence-nonce keeps its 4 names" reason). V1.1 specifies the extend-to-9 path; cycle-2 confirms.

### 5.1 Extend `PresenceNonceAction` from 4 → 9 variants

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

`as_str` / `parse` match arms updated 1:1 with `FeedbackAction::as_str` at `abilities-runtime/src/abilities/feedback.rs:65`.

**Migration impact:** in-memory store; no DB schema change. Any in-flight nonce issued before the upgrade fails to parse + emits `presence_nonce_rejected` with reason `action_kind_invalid` — acceptable transient behavior, mirrors the 60s TTL.

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

Add `payload_json: Option<String>`. Issued at `issue_nonce` time from the request body; HKDF-bound (so the digest covers the payload and any tamper invalidates the nonce); consumed at `verify_nonce` time and forwarded to `record_claim_feedback` as `ClaimFeedbackInput::payload_json`.

**Mandatory for these variants** (per ADR-0123 + `validate_feedback_payload` at `claims.rs:5150`):
- `WrongSource`: `payload_json` carries `{ "source_index": <int> }` — which source index is wrong.
- `NeedsNuance`: `payload_json` carries `{ "corrected_text": <string> }` — required.
- `SurfaceInappropriate`: `payload_json` carries `{ "surface": <string> }` — which surface.
- `NotRelevantHere`: `payload_json` carries `{ "invocation_id": <string> }` — which invocation.
- `WrongSubject` (optional): `payload_json` may carry `{ "corrected_to": <subject_ref> }`.

For variants that don't need a payload (`ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `CannotVerify`) — `payload_json` is `None`.

### 5.3 `wp_user_id` binding — canonical locus

`wp_user_id` is the durable WP-user identifier the binding ties to. **Existing state** (confirmed cycle-1 K-in):
- `surface_pairing.rs:189` derives `wp_user_id` from the session on issue.
- `surface_runtime/mod.rs:1946` cross-validates body/query/header `wp_user_id` against the session.

**W4 additions:**
- Store `wp_user_id` in the presence-nonce binding (already present per `PresenceNonceBindingFields` — confirmed at :481).
- `verify_nonce` MUST reject when `requesting wp_user_id ≠ stored wp_user_id` with reason `wp_user_mismatch`.
- Audit payload on `presence_nonce_rejected` carries `attempted_wp_user_id` for forensic trail.

Canonical locus declaration: the binding is the source of truth; never trust JS-supplied `wp_user_id` on the verify request — always derive from the WP session and cross-check against the binding.

### 5.4 Wire `verify_nonce` → `record_claim_feedback`

File: `src-tauri/src/surface_runtime/mod.rs:2019` (the existing `(Method::POST, "/v1/surface/nonce/verify")` route handler).

Current behavior: route handler calls `SurfaceNonceService::verify_nonce`, which:
1. Looks up the nonce by digest.
2. Validates: not consumed, not expired, `wp_user_id` matches (per §5.3), `action_kind` matches.
3. Marks consumed + emits `presence_nonce_verified` audit event.
4. Returns to caller.

**W4 extension** — atomic single-transaction wire-through:

```rust
// Pseudocode for the extended verify_nonce body (consolidating
// nonce consumption + record_claim_feedback into one tx).
fn verify_nonce_and_record_feedback(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    request: VerifyNonceRequest,
) -> Result<VerifyNonceOutcome, SurfaceNonceError> {
    let nonce = load_nonce_by_digest(db, &request.digest)?;
    if nonce.consumed_at.is_some() {
        emit_audit(db, presence_nonce_rejected("already_consumed", &nonce, &request))?;
        return Err(SurfaceNonceError::AlreadyConsumed);
    }
    if nonce.expires_at < ctx.clock.now() {
        emit_audit(db, presence_nonce_rejected("expired", &nonce, &request))?;
        return Err(SurfaceNonceError::Expired);
    }
    if nonce.wp_user_id != request.wp_user_id {
        emit_audit(db, presence_nonce_rejected("wp_user_mismatch", &nonce, &request))?;
        return Err(SurfaceNonceError::WpUserMismatch);
    }
    // Server reads action_kind from the stored row (NOT the client request).
    let action: FeedbackAction = nonce.action.into();
    let input = ClaimFeedbackInput {
        claim_id: nonce.claim_id.clone(),
        action,
        actor: format!("wp_user:{}", nonce.wp_user_id),
        actor_id: Some(nonce.session_id.clone()),
        payload_json: nonce.payload_json.clone(),
    };
    // Atomic: mark consumed AND record feedback in one tx. UPDATE WHERE
    // consumed_at IS NULL returning rows_affected; rows_affected == 0 ⇒
    // someone else consumed first ⇒ replay rejected.
    let outcome = with_transaction(db, |tx| {
        let rows_affected = mark_consumed_if_unclaimed(tx, &nonce.digest)?;
        if rows_affected == 0 {
            return Err(SurfaceNonceError::AlreadyConsumed);
        }
        // record_claim_feedback enters MutationGuard + check_mutation_allowed
        // and emits its own claim_feedback_recorded audit row.
        record_claim_feedback(ctx, tx, input)
    })?;
    emit_audit(db, presence_nonce_verified(&nonce, &request))?;
    Ok(VerifyNonceOutcome::from(outcome))
}
```

**Atomicity contract** (closes security-auditor TOCTOU finding):
- Mark-consumed + record-feedback live in one `with_transaction` block.
- Mark-consumed uses `UPDATE ... WHERE consumed_at IS NULL` with `rows_affected` check — zero rows means another verify won the race; reject as replay.
- `record_claim_feedback` itself enters `MutationGuard::reserve` for the claim — second-level chokepoint contract.

**Audit ordering contract** (closes CSO partial-commit finding):
- `presence_nonce_verified` emits AFTER the transaction commits (post-tx, not inside).
- `presence_nonce_rejected` emits BEFORE returning the error (so audit trail catches the rejection regardless of caller handling).
- `claim_feedback_recorded` (emitted inside `record_claim_feedback`) lands inside the tx — that's existing substrate behavior.

### 5.5 Audit events — align with existing `presence_nonce_*` names

**Existing events** (emitted by `surface_nonce.rs` today):
- `presence_nonce_issued` — on `issue_nonce` success.
- `presence_nonce_verified` — on `verify_nonce` success.
- `presence_nonce_rejected` — on `verify_nonce` rejection (any reason).

**W4 extensions** (payload-only — no new event types):

| Event | Payload fields added | Why |
|---|---|---|
| `presence_nonce_issued` | `wp_user_id`, `ip_hash` (HMAC keyed at install), `user_agent_hash`, `action_kind` (one of the 9) | Forensic trail per CSO finding. |
| `presence_nonce_verified` | `wp_user_id`, `ip_hash`, `user_agent_hash`, `claim_id`, `action_kind`, `claim_feedback_id` (from `record_claim_feedback` outcome) | Correlate the verify → record_feedback chain. |
| `presence_nonce_rejected` | `wp_user_id`, `attempted_wp_user_id`, `attempted_surface_client_id`, `rejection_reason` (one of `already_consumed` / `expired` / `wp_user_mismatch` / `action_kind_invalid`) | Replay-rejection forensic completeness. |

**Sensitivity=User content NEVER in audit payloads** — `payload_json` user-authored fields (corrected_text, corrected_to, surface, invocation_id) stay in the claim_feedback row, surfaced only through actor-filtered projection per ADR-0108.

### 5.6 WP REST extension — extend `/v1/surface/nonce/{issue, verify}`

**No new endpoint.** W4 extends the existing routes at `wp/dailyos/includes/class-dailyos-plugin.php:520-816` to carry the `payload_json` field. Specifically:

- `POST /wp-json/dailyos/v1/surface/nonce/issue` — request body adds `payload_json?: object` (JSON-serialized to string before forwarding to runtime). Permission callback unchanged; uses the existing `can_issue_presence_nonce`.
- `POST /wp-json/dailyos/v1/surface/nonce/verify` — request body adds `payload_json?: object` (only present for the same-payload echo path; the canonical payload comes from the stored binding, NOT this echo). Permission callback unchanged.

**JS affordance call sequence** (no API surface change beyond the field addition):
1. Phase 1 (issue): JS POSTs `{ surface_client_id, session_id, claim_id, action_kind, payload_json? }` to `/issue`. WP signs the runtime call; runtime mints the nonce + stores the binding + emits `presence_nonce_issued`. Response: `{ nonce_digest, expires_at }`.
2. Phase 2 (verify): JS POSTs `{ nonce_digest }` to `/verify`. WP signs the runtime call; runtime executes §5.4. Response: `{ feedback_id, new_verification_state }` on success; error code + reason on failure.

Why not a separate `/v1/feedback`: the substrate path **is** the nonce path. Adding a parallel endpoint with the same auth + transport shape adds surface area + an MCP-allowlist sibling without proportional value. (Closes V1.0 invented-parallel finding.)

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
5. Audit log contains: `presence_nonce_issued → claim_feedback_recorded → presence_nonce_verified` (in that order — the claim_feedback row commits inside the tx, the verified event emits post-commit).
6. Replay: HTTP POST `/v1/surface/nonce/verify` with the same `nonce_digest`. Runtime rejects + emits `presence_nonce_rejected` with `rejection_reason: "already_consumed"` + `attempted_wp_user_id`. Re-render unchanged.

## 6. Decisions to lock at L0

1. **All write paths go through `services::claims::record_claim_feedback`.** The WP REST handler is a thin envelope: validates request, signs the runtime call, relays the response. No DB writes from WP. (Locks ADR-0111 + ADR-0126 mirror.)
2. **`record_claim_feedback` is consumed unchanged.** W4 does not modify the function body or the `FeedbackAction` enum. **CI gate (narrowed):** PR must not edit `src-tauri/src/services/claims.rs` lines containing `fn record_claim_feedback` through its closing brace, AND must not edit `src-tauri/abilities-runtime/src/abilities/feedback.rs` `enum FeedbackAction` block. (V1.0 had a blanket "no edits to `claims.rs` non-test" which is too broad — `claims.rs` has 100+ legitimate edit paths from W2+; the gate must target the specific function + enum.)
3. **Two-phase nonce.** No single-shot path. Every feedback write requires `issue` THEN `verify` from the same actor on the same `surface_client_id` AND `wp_user_id`.
4. **Replay-rejection is non-negotiable.** Consumed nonce can never be consumed again; rejection is recorded as `presence_nonce_rejected` audit event with `rejection_reason: "already_consumed"`.
5. **`payload_json` user-authored fields sensitivity=User.** NEVER appear in audit event payloads. Redaction enforced at the projection layer per W2 DOS-477 leak guards (V1.1 verifies the channel list at cycle 2 — see §12).
6. **60s default TTL on un-verified nonces.** Existing presence-nonce behavior; W4 does NOT introduce a longer TTL. Rationale: feedback affordances are click-bound; the user is right there. Longer TTL widens the replay window without UX benefit. (V1.0 proposed 24h TTL; cycle-1 rejected because the presence nonce is in-memory not table-backed.)
7. **W4 ships affordance UI ONLY on `dailyos/account-overview`** for v1.4.3 acceptance. Other W2 primitive blocks + composites get affordance UI in v1.4.4 surface migration.
8. **`wp_user_id` is canonical and server-derived.** Stored in the presence-nonce binding at issue; cross-checked at verify; rejected with `wp_user_mismatch` on any disagreement. Never trust JS-supplied `wp_user_id`.
9. **NEW: feedback flow has no migrations.** The existing nonce store is in-memory with 60s TTL. If a feedback flow ever needs durability beyond 60s (offline retry, audit forensics), that's an ADR amendment requiring explicit `/cso` sign-off — NOT a packet decision. (V1.0 proposed migrations v181/v182/v183 — DELETED in V1.1.)
10. **NEW: replay rejection is fail-closed.** Any ambiguous state (concurrent verify race, partial-commit mid-flight, stored nonce row corruption) MUST reject with `presence_nonce_rejected` rather than silently succeed.

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
`src-tauri/abilities-runtime/tests/surface_nonce_audit_correlation.rs` — for a happy-path feedback, verify the chain `presence_nonce_issued → claim_feedback_recorded → presence_nonce_verified` lands with consistent `request_id` correlation + the audit payload extensions from §5.5 are present.

## 9. Invariants (CI-enforced)

1. **No DB writes from WP outside `services::claims::record_claim_feedback`.** Already enforced by existing `raw_wpdb_outside_services` gate.
2. **No `record_claim_feedback` body modifications in W4 PR.** Diff gate: PR must not edit the lines spanning `pub fn record_claim_feedback` through its closing brace at `src-tauri/src/services/claims.rs:6700+`. Also must not edit `pub enum FeedbackAction` block at `src-tauri/abilities-runtime/src/abilities/feedback.rs:31`.
3. **`verify_nonce` MUST pass through `record_claim_feedback`** (which enters `ctx.check_mutation_allowed()`). Grep gate on `services/surface_nonce.rs` verify path: must contain `record_claim_feedback` call.
4. **No raw user-authored `payload_json` content in audit event payloads.** Grep gate: audit-event writer callers in `surface_nonce.rs` must not pass `nonce.payload_json` through.
5. **REST endpoint MUST use `DailyOS_Runtime_Client` for runtime calls.** No raw `wp_remote_post` to the runtime sentinel from feedback handler. (Already enforced by existing `wp_remote_post_body_array` gate.)
6. **Replay-rejection MUST emit `presence_nonce_rejected` with `rejection_reason: "already_consumed"`.** Verified by §8.3 integration test.
7. **`wp_user_id` mismatch MUST emit `presence_nonce_rejected` with `rejection_reason: "wp_user_mismatch"`.** Verified by §8.4 integration test.
8. **`PresenceNonceAction` MUST have exactly 9 variants matching `FeedbackAction` 1:1.** Compile-time check via `From<PresenceNonceAction> for FeedbackAction` impl + exhaustive match.
9. **No customer-specific data in test fixtures.** CLAUDE.md rule. Use `acct-test-001`, `claim-test-001` generic IDs.
10. **No PII in commit messages.** CLAUDE.md rule.
11. **L2-status on every code commit.** CLAUDE.md rule + commit-msg hook.

## 10. PR shape

W4 ships as one PR (PR-F1) to `dev`, multi-commit for L2 reviewability, squash-merge at landing. Ordered per codex consult cycle-1 sequencing:

1. **`PresenceNonceAction` extension (4 → 9)** + `From<PresenceNonceAction> for FeedbackAction` + Rust unit test (§8.1).
2. **`surface_nonce.rs` `payload_json` field + `wp_user_id` binding additions** + audit payload extensions per §5.5 + Rust integration tests (§8.2, §8.4, §8.5).
3. **`verify_nonce` → `record_claim_feedback` wire-through** at `surface_runtime/mod.rs:2019` + atomic-tx + replay-rejection test (§8.3) + audit correlation test (§8.10).
4. **WP REST endpoint extension** (request body validation; `payload_json` shape per variant) + PHPUnit (§8.7, §8.8, §8.9).
5. **JS feedback affordance component** + style.css + integration into `account-overview` block.
6. **End-to-end fixture** + L4 visual parity matrix capture (§8.6).

## 11. Reviewer matrix

Per wave-plan §W4 + CLAUDE.md L0 Amendment 3 (nonce lifecycle + write path + trust-boundary):

- `/codex challenge` (adversarial probe — L0 + L2)
- `code-reviewer` (Rust + PHP correctness)
- `/codex consult` (sequencing + cross-layer integration)
- `/cso` — **MANDATORY** (nonce lifecycle + write path + new trust boundary surface)
- `security-auditor` — **MANDATORY** (per Amendment 3 — claim/provenance/write-path)

**K-in obligation per CLAUDE.md:** reviewers grep `docs/solutions/` + `.docs/decisions/` AND inspect actual file:line paths in §3 / §4 BEFORE scoring. V1.1 K-in evidence in §4 is grounded by file:line — reviewers verify the paths.

## 12. Open questions

1. **Channel-count baseline for the leak-guard registry.** V1.0 claimed 10 channels; cycle-1 codex challenge says baseline is 9 (per `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:252`). V1.1 cycle-2: **read the channel registry directly** and either (a) confirm 9 → W4 adds a 10th for the feedback-render projection, or (b) document that feedback-render is already covered by an existing channel and the count stays 9.
2. **`PresenceNonceAction` extension (V1.1 recommendation) vs mapping shim.** V1.1 specifies the extend-to-9 path. The mapping shim alternative would translate the existing 4 variants at the verify boundary into the 9 substrate actions — keeps presence-nonce-public-shape as 4 names but adds a translation layer. **V1.1 recommendation:** extend to 9. Substrate enum already in `abilities-runtime`; mapping adds an extra layer for cosmetic-only reasons. Cycle-2 confirms.
3. **`SurfaceInappropriate` + `NotRelevantHere` — surface payload derivation.** Both variants take an environmental marker (which surface / which invocation). Should the JS auto-detect the value from the rendering context, or ask the user to select? **Recommended:** auto-detect; show the detected value as confirmation; user can override before confirm.
4. **JS bundle size impact.** Adding the 9-action menu + textarea + state machine to the account-overview block adds ~5-10 KB. Stays under the existing per-block budget. **Recommended:** ship; revisit at L4 if bundle audit catches a regression.
5. **`payload_json` redaction policy at audit-event boundary.** Confirmed in §5.5 — user-authored fields never in audit payloads. Cycle-2: verify by reading the audit-event writer body and confirming the projection layer's redaction policy aligns.

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

B. **Presence-nonce binding carries `payload_json: Option<String>`** at issue time; HKDF-bound (tamper invalidates the nonce); forwarded to `record_claim_feedback` at verify.

C. **Presence-nonce binding carries `wp_user_id: u64`** at issue; `verify_nonce` rejects on mismatch with `rejection_reason: "wp_user_mismatch"`.

D. **`verify_nonce` → `record_claim_feedback` wire-through.** Successful verify creates one `claim_feedback` row + transitions the claim per ADR-0123 §1. Single transaction. `MutationGuard::reserve` reached.

E. **Replay rejection atomic.** Two concurrent verify calls with the same nonce: exactly one succeeds, exactly one rejects with `rejection_reason: "already_consumed"`. Verified by §8.3.

F. **Audit payload extensions land.** `presence_nonce_issued` / `_verified` / `_rejected` carry `wp_user_id` + `ip_hash` + `user_agent_hash` (+ correlation-specific fields per §5.5). User-authored `payload_json` content never in audit payloads.

G. **WP REST extension works for both phases.** Permission callback unchanged (reuses existing). `payload_json` validates per variant.

H. **JS feedback affordance renders on `dailyos/account-overview`.** All 9 actions selectable; conditional textarea / surface-picker / invocation-picker per variant; single-trigger + reveal menu.

I. **End-to-end fixture passes for 4 representative variants** (`MarkOutdated`, `MarkFalse`, `WrongSubject` with payload, `NeedsNuance` with payload).

J. **`payload_json` user-authored fields never leak to non-originating actors.** PHPUnit at §8.9.

K. **CI gates pass.** §9 invariants enforced. Critically: `record_claim_feedback` body + `FeedbackAction` enum unchanged.

L. **L2 unanimous APPROVE** per §11 reviewer matrix, bounded by AC A–K.

M. **L4 hands-on** — 12 screenshot pairs captured (parked end-of-batch).

N. **CSO + security-auditor APPROVE** (mandatory L0 + L2 review per Amendment 3).

O. **No regressions in W2 primitive blocks, W3 magazine theme, or any existing presence-nonce caller** (e.g., W2 `submit_feedback` transport path at `runtime-client.php:149`).

## 15. Lock criteria

Cycle 1 reviewer outputs (V1.0): 2 BLOCK (code-reviewer, codex challenge) + 3 CONDITIONAL APPROVE (codex consult, CSO, security-auditor). V1.1 folds all 14 convergent findings. Cycle 2 reviewer outputs captured in `.docs/plans/v1.4.3-wp-foundation/reviews/packet-F-{code-reviewer, codex-challenge, codex-consult, cso, security-auditor}-cycle2.md`.

Pack locks when:
- Cycle 2 verdict set is unanimous APPROVE *or* unanimous-equivalent (≤2 reviewers CONDITIONAL with strictly surgical conditions folded into V1.x; 0 BLOCK; 0 active class-pattern).
- All folded conditions verifiable from packet text alone.
- No reviewer dissent on critical findings (per memory `feedback_reviewer_dissent_is_signal.md`).
- CSO + security-auditor unanimous APPROVE (mandatory gates).
- Cycle-2 confirms the §5.1 reviewer call (extend `PresenceNonceAction` 4 → 9 vs mapping shim) — V1.1 specifies extend; reviewers confirm or surface a stronger mapping-shim argument.
- §12 open question #1 resolved with a verified channel count (read the registry, don't claim).

Per `feedback_review_loop_l6_policy.md`: 15-cycle hard cap; class-pattern sweeps at 2-similar-findings; convergence rule allows surgical CONDITIONALs to lock.
