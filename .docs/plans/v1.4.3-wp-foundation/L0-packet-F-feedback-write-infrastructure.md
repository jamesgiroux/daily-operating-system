# L0 Packet F — v1.4.3 W4: Feedback Write Infrastructure

## 1. Header

- **Author:** James Giroux (with Claude)
- **Date:** 2026-05-19
- **Linear:** [DOS-683](https://linear.app/a8c/issue/DOS-683) (v1.4.3 W4 — Feedback Write Infrastructure)
- **Branch:** TBD (`dos-683-w4-feedback-write` off `dev` once V1.x locks; rebase on top of `dos-698-w3-magazine-theme` if PR-E1 hasn't merged yet)
- **Wave plan:** `.docs/plans/v1.4.3-waves.md` §W4 (lines 203–218)
- **Upstream gate:** W2 (DOS-682, merged — all 11 primitive blocks + `account-overview` composite + presence-nonce REST scaffolding live on dev). W3 (PR #315, awaiting merge — adds `dailyos_account` CPT + plugin-owned baseline token shim; W4 implementation will rebase on top once merged).

**Intelligence Loop integration check — APPLIES.** W4 ships substrate that writes to the claim graph through `record_claim_feedback` per ADR-0123:

1. **Claim model:** Feedback writes mutate existing claims (state, source weights, agent ledger) and may create user-authored superseding/absence/tombstone claims. Subject attribution preserved via `SubjectRef` (ADR-0105). Temporal scope: feedback rows carry `consumed_at` timestamp. Sensitivity: `user_intent_text` is sensitivity=User per ADR-0108. Lifecycle: claim transitions to `superseded` / `withdrawn` / unchanged depending on `FeedbackAction` variant.
2. **Provenance + trust:** Each feedback row carries actor (`UserActor` for hand-clicked corrections, distinct from `AgentActor` automation). Trust factors mutate per ADR-0123 §4: source-attribution downweight on `WrongSource`, freshness-factor downweight on `MarkOutdated`, no penalty on `CannotVerify`. Trust-band-rendering refreshes on the next projection.
3. **Signals + invalidation:** Feedback emits signals via `enqueue_signals_for_feedback` (existing in `services/claims.rs`) and triggers cache invalidation on the claim's composition keys. Propagation through `intel_queue` reaches affected projections within the next render.
4. **Runtime + surfaces:** W4 exposes a single new ability surface — `surface_feedback::issue_nonce` + `consume_nonce` — gated by the existing actor/scope authorization (`authorize_local_render` is read-only; this is the first write path that must pass `check_mutation_allowed`). MCP surface NOT extended in W4 (feedback writes are click-bound, not ability-invocable yet).
5. **Feedback loop:** This IS the feedback loop. W4 closes the round-trip: rendered claim → user click → nonce mint → user confirm → substrate write → invalidation → next render reflects the feedback.

**ADR-named contracts:** **ADR-0123** (typed claim feedback semantics — 9-variant `FeedbackAction` enum), **ADR-0111** (surface-independent ability invocation — feedback writes through services/claims, NOT direct DB writes from WP), **ADR-0126** (memory substrate invariants — all mutations through `services/`), **ADR-0108** (actor/surface-filtered provenance — user_intent_text sensitivity=User).

## 2. Changelog

- **V1.0 (2026-05-19):** Initial L0 draft. Authored against `.docs/plans/v1.4.3-waves.md` §W4 + Linear DOS-683 + ADR-0123 + ADR-0111 + ADR-0126 + ADR-0108 + existing presence-nonce scaffolding in `wp/dailyos/includes/class-dailyos-plugin.php:520-816` + existing `record_claim_feedback` service at `src-tauri/src/services/claims.rs:6700`. Reviewer panel: codex challenge + code-reviewer + codex consult + `/cso` MANDATORY + `security-auditor` MANDATORY per CLAUDE.md L0 Amendment 3 (W4 touches nonce lifecycle + write path + new trust boundary). K-in evidence captured in §4.

## 3. Status Snapshot

- Linear ticket: DOS-683 (Backlog, v1.4.3 — WordPress Foundation, priority High).
- Substrate reuse:
  - **Rust:** `record_claim_feedback` (`services/claims.rs:6700`) handles all 9 `FeedbackAction` variants per ADR-0123; `MutationGuard` reserves the commit chokepoint; `enqueue_signals_for_feedback` propagates invalidation.
  - **WP plugin:** presence-nonce REST scaffold (`issue_presence_nonce` + `can_issue_presence_nonce` + `sweep_presence_nonces` + `strip_presence_nonces_from_post_data`) exists from v1.4.2 W4-F; provides the click-bound shape we extend.
  - **MCP runtime:** `MutationGuard` + `check_mutation_allowed` enforce write-path-only-for-mutation discipline.
- Net-new in W4:
  - **Migration v181:** `surface_feedback_nonces` table (nonce_id, surface_client_id, session_id, claim_id, action_kind, user_intent_text, issued_at, consumed_at, expires_at).
  - **Migration v182:** index on `(surface_client_id, expires_at)` for efficient sweep.
  - **Migration v183 (reserved):** rollback path if any.
  - **Rust service:** `src-tauri/src/services/surface_feedback.rs` with `issue_nonce`, `consume_nonce`, `expire_nonces_sweep`.
  - **Audit event types:** `pairing.feedback.nonce_issued`, `.consumed`, `.expired`, `.replay_rejected`.
  - **PHP REST endpoint:** `wp/dailyos/v1/feedback` (POST issues + consumes the 2-phase nonce flow).
  - **JS feedback affordance UI:** per-block React component rendered inline when claim has feedback affordance enabled; button + optional textarea (`user_intent_text`, 500 char cap, sensitivity=User).
  - **Wire-up:** consume_nonce → `record_claim_feedback` substrate service.
  - **Tests:** Rust integration tests for nonce lifecycle + replay rejection + actor-binding; PHPUnit for REST endpoint + permission checks; end-to-end proof fixture per §8.
- **W4 unlocks:** v1.4.4 surface migration (composite + per-block feedback affordances on briefing/meeting/project surfaces); v1.4.x audit forensic validation (W6) consumes new audit event types.

## 4. Pre-work — K-in evidence (CLAUDE.md mandate) + substrate reuse audit

**Knowledge-store grep results (`docs/solutions/` + `.docs/decisions/`):**

| Query | Hits | What we reuse |
|---|---|---|
| `record_claim_feedback`, `claim_feedback` | ADR-0123 (typed claim feedback semantics) + 4 Rust files (services/claims.rs has the canonical impl) | The 9-variant `FeedbackAction` enum + the `ClaimFeedback` row shape + the trust/lifecycle/agent-ledger triple per variant are LOCKED. W4 does not change them — only adds the WP→runtime delivery path. |
| `surface_feedback`, feedback nonce | none in `.docs/decisions/`; net-new to W4 | New substrate. Modeled on the existing `presence_nonce` shape (`wp/dailyos/includes/class-dailyos-plugin.php:722-816`) for symmetry. |
| presence nonce, click-bound | ADR-0111 (surface-independent ability invocation) | Confirms the click-bound discipline: feedback writes flow through `services::claim`, NOT direct DB writes from WP. The W4 REST endpoint mints a nonce, waits for user click confirmation, then signs a runtime call that consumes the nonce + invokes `record_claim_feedback`. |
| memory substrate invariants | ADR-0126 | "All mutations go through `services/`" (CLAUDE.md cardinal rule mirror). W4's surface_feedback service must be the ONLY entry point from WP for feedback writes. |
| actor-filtered provenance, sensitivity=User | ADR-0108 | `user_intent_text` (the optional textarea note) carries sensitivity=User. Audit log displays only the actor handle + redacted hash, never the raw text. Per-surface display policy must redact the text unless the requesting actor === the originating actor. |
| W4-F presence-nonce v1.4.2 spike artifacts | `wp/dailyos/includes/class-dailyos-plugin.php:520-816` + `wp/dailyos/tests/PresenceNonceTest.php` | The presence-nonce REST scaffold (`issue_presence_nonce`, `can_issue_presence_nonce`, `strip_presence_nonces_from_post_data`, `sweep_presence_nonces`) is the existing seam W4 extends. W4 adds a sibling endpoint (`/v1/feedback`) using the same authorization shape (signed runtime call) and a parallel table (`surface_feedback_nonces` vs `surface_presence_nonces`). |
| `MutationGuard`, `check_mutation_allowed` | `services/claims.rs` | Existing chokepoint contract. `record_claim_feedback` already passes `ctx.check_mutation_allowed()`; W4 service `surface_feedback::consume_nonce` must do the same before signing through to claims. |
| audit events, pairing.feedback | none specifically; ADR-0094 (audit-log + enterprise observability) covers the event-shape contract | New event types must register with the existing audit infrastructure (no schema additions; the `event_type` column accepts the 4 new strings). |

**Net K-in conclusion:** W4 is the WRITE-PATH WIRE-UP between (a) WP's per-block feedback affordance UI and (b) the substrate's already-locked `record_claim_feedback`. Net-new: a 2-phase nonce lifecycle service + table + audit events + REST endpoint + per-block JS affordance. Every piece either consumes an already-shipped substrate OR mirrors an existing pattern (presence-nonce shape, ClaimFeedback row shape).

**Substrate already-shipped (table):**

| What we need | Where it lives | W4 disposition |
|---|---|---|
| 9-variant FeedbackAction enum | `services/claims.rs:FeedbackAction` per ADR-0123 | Consumed verbatim. |
| ClaimFeedback row shape | `services/claims.rs:ClaimFeedback` per ADR-0123 §2 | Consumed; W4 builds the `ClaimFeedbackInput` from the consumed nonce + user-submitted action_kind. |
| `record_claim_feedback` service | `services/claims.rs:6700` | Wired from `surface_feedback::consume_nonce` (W4 new service). |
| `MutationGuard` chokepoint | `services/claims.rs` | Enforced by `record_claim_feedback` already; W4's nonce-consume path passes through it. |
| `enqueue_signals_for_feedback` | `services/claims.rs` | Triggered automatically by `record_claim_feedback`; W4 does not invoke directly. |
| Presence-nonce REST scaffold | `class-dailyos-plugin.php:520-816` | Pattern mirrored for `/v1/feedback`; same auth shape (signed runtime call), same sweep job structure. |
| `dailyos_runtime_client` (signed loopback transport) | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php` | Used to mint nonce + sign consume call from WP. |
| ADR-0108 actor-filtered provenance | `provenance/projection.rs` | `user_intent_text` redaction policy: the text is stored in the DB row but the projection layer (display-safe leak guards from W2 DOS-477) filters it out for non-originating actors. |
| Audit infra | `audit_events` table + writer service | `event_type` column accepts new strings without schema migration. |

## 5. What this packet authors

### 5.1 Migration v181 — `surface_feedback_nonces` table

File: `src-tauri/src/migrations/v181_surface_feedback_nonces.rs` (or `.sql` per existing convention — see migration-slot policy).

Schema:

```sql
CREATE TABLE surface_feedback_nonces (
    nonce_id           TEXT    PRIMARY KEY,    -- UUIDv7 (sortable by issued_at)
    surface_client_id  TEXT    NOT NULL,       -- FK-ish to surface_client_sessions; not enforced for sweep flexibility
    session_id         TEXT    NOT NULL,       -- claim-scoped session for this user-presence binding
    claim_id           TEXT    NOT NULL,       -- the claim the user is acting on
    action_kind        TEXT    NOT NULL,       -- one of the 9 FeedbackAction variant tags (canonical lowercase)
    user_intent_text   TEXT,                   -- nullable; optional textarea (500 char cap enforced WP-side; sensitivity=User)
    issued_at          INTEGER NOT NULL,       -- unix ms; UTC
    consumed_at        INTEGER,                -- unix ms; NULL until user clicks confirm
    expires_at         INTEGER NOT NULL,       -- issued_at + 24h default; expired rows reaped by sweep job
    actor_kind         TEXT    NOT NULL,       -- 'user' for hand-clicked corrections; one nonce never carries 'agent'
    request_id         TEXT    NOT NULL        -- correlation ID for audit trail
);
```

### 5.2 Migration v182 — sweep index

```sql
CREATE INDEX surface_feedback_nonces_sweep_idx
    ON surface_feedback_nonces (surface_client_id, expires_at)
    WHERE consumed_at IS NULL;
```

Partial index: only un-consumed nonces are eligible for sweep (consumed rows are audit-trail-only).

### 5.3 Migration v183 — reserved

Reserved per wave-plan §W4. Used only if cycle-N reviewer finding requires an in-flight schema rollback during implementation.

### 5.4 Rust service — `src-tauri/src/services/surface_feedback.rs`

Public API (mutations only — read paths NOT introduced; per ADR-0111 read-side already covered by `project_composition`):

```rust
pub fn issue_nonce(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: IssueNonceInput,
) -> Result<IssueNonceOutcome, SurfaceFeedbackError> {
    ctx.check_mutation_allowed()?;
    // … validate input, write row, emit pairing.feedback.nonce_issued audit event.
}

pub fn consume_nonce(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ConsumeNonceInput,  // carries nonce_id, requesting actor, action_kind echo
) -> Result<ClaimFeedbackOutcome, SurfaceFeedbackError> {
    ctx.check_mutation_allowed()?;
    // 1. Load nonce by nonce_id.
    // 2. Validate: not consumed (replay rejection), not expired, action_kind matches issued,
    //    requesting actor === issuing actor (presence binding).
    // 3. Mark consumed; emit pairing.feedback.nonce_consumed audit event.
    // 4. Translate to ClaimFeedbackInput (using the nonce's claim_id + user_intent_text).
    // 5. Call services::claim::record_claim_feedback (which itself enters MutationGuard +
    //    enqueue_signals_for_feedback).
    // 6. Return the substrate outcome to the caller.
}

pub fn expire_nonces_sweep(
    db: &ActionDb,
    now: DateTime<Utc>,
) -> Result<SweepOutcome, SurfaceFeedbackError> {
    // Reap rows where expires_at < now AND consumed_at IS NULL.
    // Emit pairing.feedback.nonce_expired audit event per row reaped (batch-write for efficiency).
}
```

**Error variants** (`SurfaceFeedbackError`): `NonceNotFound`, `NonceExpired`, `NonceAlreadyConsumed`, `ActorMismatch` (replay-from-different-session), `ActionKindMismatch` (UI race or replay), `UnknownClaimId`, `Mutation(ClaimError)` (downstream from `record_claim_feedback`).

### 5.5 Audit events (4 types)

Added to the audit-event allowlist; no schema migration required (the `event_type` column is `TEXT`).

| Event type | Emitted when | Payload (`event_data` JSON) |
|---|---|---|
| `pairing.feedback.nonce_issued` | `issue_nonce` succeeds | `{ nonce_id, surface_client_id, session_id, claim_id, action_kind, expires_at, actor_kind, request_id }` |
| `pairing.feedback.nonce_consumed` | `consume_nonce` succeeds (BEFORE the downstream `record_claim_feedback` audit row, so the chain is `nonce_issued → nonce_consumed → claim_feedback_recorded`) | `{ nonce_id, claim_id, action_kind, request_id, claim_feedback_id, ... }` |
| `pairing.feedback.nonce_expired` | `expire_nonces_sweep` reaps an un-consumed nonce past `expires_at` | `{ nonce_id, claim_id, expires_at, swept_at }` |
| `pairing.feedback.replay_rejected` | `consume_nonce` rejects because the nonce was already consumed or the actor doesn't match | `{ nonce_id, claim_id, rejection_reason: 'already_consumed' \| 'actor_mismatch' \| 'expired', request_id }` |

`user_intent_text` is **NEVER** emitted in audit event payloads — sensitivity=User per ADR-0108. Audit consumers can look up the redacted text via the actor-filtered projection if authorized.

### 5.6 PHP REST endpoint — `wp/dailyos/v1/feedback`

Added to `wp/dailyos/includes/class-dailyos-plugin.php` in `register_rest_routes` alongside the existing presence-nonce endpoint:

- **Route:** `POST /wp-json/dailyos/v1/feedback`
- **Permission callback:** mirrors `can_issue_presence_nonce` — requires authenticated WP user + a valid pairing marker + the request's `surface_client_id` to match an active surface session.
- **Request body:** `{ claim_id, action_kind, user_intent_text?, surface_client_id, session_id }` — JSON. Validates `action_kind` against the 9-variant allowlist. Caps `user_intent_text` at 500 chars (per W4-F class E security limit).
- **Two-phase semantics:**
  - Phase 1 (issue): WP REST handler signs a runtime call to `surface_feedback::issue_nonce` via `DailyOS_Runtime_Client`; returns `{ nonce_id, expires_at }` to the JS affordance.
  - Phase 2 (consume): JS receives the `nonce_id`; on user click "Confirm" it POSTs `{ nonce_id, claim_id, action_kind, user_intent_text }` to the same endpoint; WP signs a runtime call to `surface_feedback::consume_nonce`; runtime records the feedback + returns the outcome.

Same endpoint URL handles both phases distinguished by the presence of `nonce_id` in the body. Documented as such; alternative (separate `/v1/feedback/issue` + `/v1/feedback/consume`) considered + rejected (more surface area; same auth shape).

### 5.7 JS feedback affordance — per-block UI

Added as a shared React component used by W2 primitive blocks that render claim data. v1.4.3 W4 surfaces it only on `dailyos/account-overview` composite (per wave-plan §W4 Stage 4c proof scope); other blocks adopt it in v1.4.4 surface migration.

Component shape:
- Trigger button: "Feedback" (or 9 action-specific buttons collapsed behind a single trigger; first cycle picks one — see §12).
- Action menu: 9 entries mapping to the `FeedbackAction` variants per ADR-0123.
- Optional textarea: 500 char cap; appears for `NeedsNuance` (required) + `WrongSubject` (optional) + `WrongSource` (optional).
- Confirm/Cancel buttons.
- Loading state while waiting for the runtime call.
- Success state: claim re-renders with the updated trust band / lifecycle / withdrawn marker.
- Error state: shows the rejection reason from the substrate (e.g., "Already submitted" for replay).

CSS: per-block `style.css` + plugin-owned baseline tokens (no inline CSS per memory `feedback_no_inline_css.md`).

### 5.8 Wire-up: end-to-end fixture

`src-tauri/tests/dos683_w4_feedback_end_to_end_test.rs`:

1. Stand up local-to-local runtime + render `dailyos/account-overview` for a fixture account with one `LikelyCurrent` claim.
2. Simulate JS: POST `/v1/feedback` with `action_kind: 'mark_outdated'`. Receive `nonce_id`.
3. POST `/v1/feedback` again with `nonce_id` + same action_kind. Runtime consumes the nonce + records feedback.
4. Re-render: claim now shows `superseded` lifecycle + `UseWithCaution` (or `NeedsVerification`) trust band per ADR-0123 §1.
5. Audit log contains: `nonce_issued → nonce_consumed → claim_feedback_recorded`.
6. Replay: POST `/v1/feedback` a third time with the same `nonce_id`. Runtime rejects + emits `replay_rejected`. Render unchanged.

## 6. Decisions to lock at L0

1. **All write paths go through `services::surface_feedback`.** WP REST handler is a thin envelope: validates request, signs the runtime call, relays the response. No DB writes from WP. (Locks ADR-0111 + ADR-0126 mirror.)
2. **`record_claim_feedback` is consumed unchanged.** W4 does not modify `services::claims::record_claim_feedback` or the 9-variant `FeedbackAction` enum. CI gate: PR must not touch `services/claims.rs` non-test files. (Mirrors v1.4.3 W3 §9 inv #13 composite boundary discipline.)
3. **Two-phase nonce.** No single-shot path. Every feedback write requires `issue_nonce` THEN `consume_nonce` from the same actor on the same surface_client_id.
4. **Replay-rejection is non-negotiable.** A consumed nonce can never be consumed again; the rejection is recorded as `pairing.feedback.replay_rejected` audit event for forensic trail.
5. **`user_intent_text` sensitivity=User.** Never appears in audit event payloads. Redaction enforced at the projection layer per W2 DOS-477 leak guards (extend the existing channel list to cover the new feedback-render path).
6. **24h default TTL on un-consumed nonces.** Sweep job runs hourly (mirrors `dailyos_nonce_sweep` cron). Expired rows transition to audit-only trail; not deleted.
7. **W4 ships affordance UI ONLY on `dailyos/account-overview`** for v1.4.3 acceptance. Other W2 primitive blocks + composites get affordance UI in v1.4.4 surface migration. Decision rationale: smallest surface area that proves the end-to-end loop without expanding scope into v1.4.4 entity-surface composition.
8. **Actor binding is hard.** The consume request's WP user MUST match the issue request's WP user (cross-check via `surface_client_id` + WP user_id in the session). No "anyone holding the nonce can consume" semantics.

## 7. Visual parity matrix

L4 captures: 9 affordance states (one per `FeedbackAction` variant) + 3 chrome states (loading, success, replay-rejected) on the `dailyos/account-overview` block. ~12 screenshot pairs total. Parked end-of-batch per James (per Task #28 W2 batching protocol).

## 8. Test/fixture spec

### 8.1 Rust integration: nonce lifecycle
`src-tauri/abilities-runtime/tests/surface_feedback_nonce_lifecycle.rs` — happy path: issue → consume → audit trail correct.

### 8.2 Rust integration: replay rejection
`src-tauri/abilities-runtime/tests/surface_feedback_replay_rejection.rs` — issue → consume → consume-again rejected with `replay_rejected` audit event.

### 8.3 Rust integration: expiry sweep
`src-tauri/abilities-runtime/tests/surface_feedback_expire_sweep.rs` — issue → wait past expires_at → run sweep → audit event emitted, row marked expired.

### 8.4 Rust integration: actor mismatch
`src-tauri/abilities-runtime/tests/surface_feedback_actor_mismatch.rs` — issue from actor A → consume from actor B rejected.

### 8.5 Rust integration: action_kind mismatch
`src-tauri/abilities-runtime/tests/surface_feedback_action_kind_mismatch.rs` — issue with `mark_outdated` → consume with `mark_false` rejected (UI race protection).

### 8.6 Rust integration: full claim feedback round-trip
`src-tauri/tests/dos683_w4_feedback_end_to_end_test.rs` — §5.8 end-to-end fixture (one test per the 4 most common variants: `MarkOutdated`, `MarkFalse`, `WrongSubject`, `NeedsNuance`).

### 8.7 PHPUnit: REST endpoint permissions
`wp/dailyos/tests/FeedbackEndpointTest.php` — permission_callback rejects unauthenticated; requires pairing marker; surface_client_id must match active session.

### 8.8 PHPUnit: REST endpoint input validation
`wp/dailyos/tests/FeedbackEndpointInputTest.php` — action_kind allowlist (rejects unknown), user_intent_text 500-char cap, claim_id sanity, JSON shape.

### 8.9 PHPUnit: redaction at projection
`wp/dailyos/tests/FeedbackUserIntentRedactionTest.php` — `user_intent_text` is NEVER returned to a non-originating actor. Cross-references W2 DOS-477 channel list (extends from 10 to 11 channels: add "feedback-projection rendering" if not already covered).

### 8.10 Audit log forensic test
`src-tauri/abilities-runtime/tests/surface_feedback_audit_trail.rs` — for a happy-path feedback, verify the chain `nonce_issued → nonce_consumed → claim_feedback_recorded` lands in the audit log with consistent `request_id` correlation.

## 9. Invariants (CI-enforced)

1. **No DB writes from WP outside the REST handler path.** Grep gate: `\$wpdb->(insert|update|delete|query)` in `wp/dailyos/includes/` outside `services/` — already enforced by existing `raw_wpdb_outside_services` gate.
2. **No `record_claim_feedback` modifications in W4 PR.** Diff gate: PR must not edit `src-tauri/src/services/claims.rs` non-test code. Two-stage POSIX grep similar to W3 inv #13.
3. **`surface_feedback::consume_nonce` MUST pass through `ctx.check_mutation_allowed()`.** Grep gate on `services/surface_feedback.rs`.
4. **No raw `user_intent_text` in audit event payloads.** Grep gate on `audit-event writer` callers in surface_feedback service.
5. **REST endpoint MUST use `DailyOS_Runtime_Client` for runtime calls.** No raw `wp_remote_post` to the runtime sentinel from feedback handler.
6. **Replay-rejection MUST emit `pairing.feedback.replay_rejected`.** Verified by §8.2 integration test.
7. **No customer-specific data in test fixtures.** CLAUDE.md rule. Use `acct-test-001`, `claim-test-001` generic IDs.
8. **No PII in commit messages.** CLAUDE.md rule.
9. **L2-status on every code commit.** CLAUDE.md rule + commit-msg hook.
10. **Nonce TTL ≤ 24h.** Enforced at row insert.

## 10. PR shape

W4 ships as one PR (PR-F1) to `dev`, multi-commit for L2 reviewability, squash-merge at landing. Ordered:

1. **Migration v181 + v182** + Rust migration test scaffolding.
2. **`services/surface_feedback.rs`** new file + Rust unit tests.
3. **Audit event registration** (allowlist extension; no schema changes).
4. **PHP REST endpoint** + PHPUnit permission + input-validation tests.
5. **JS feedback affordance component** + style.css.
6. **Account-overview block integration** (renders affordance UI inline).
7. **End-to-end fixture** (`dos683_w4_feedback_end_to_end_test.rs`) + audit-trail test.
8. **CI gates** (composite boundary inv #2 for claims.rs; replay-rejection assertion).

## 11. Reviewer matrix

Per wave-plan §W4 + CLAUDE.md L0 Amendment 3 (nonce lifecycle + write path + trust-boundary):

- `/codex challenge` (adversarial probe — L0 + L2)
- `code-reviewer` (Rust + PHP correctness)
- `/codex consult` (sequencing + cross-layer integration)
- `/cso` — **MANDATORY** (nonce lifecycle + write path + new trust boundary)
- `security-auditor` — **MANDATORY** (per Amendment 3 — claim/provenance/write-path)

**K-in obligation per CLAUDE.md:** reviewers grep `docs/solutions/` + `.docs/decisions/` BEFORE scoring. V1.0 K-in evidence in §4.

## 12. Open questions

1. **Single trigger vs 9 buttons.** Action menu UX — collapsed behind a single "Feedback" button vs 9 always-visible buttons. v1.4.3 ships ONE pattern; v1.4.4 + L4 evidence may revise. **Recommended:** single trigger + reveal menu (matches WP block-controls discoverability), but lock the call at cycle 1.
2. **Nonce TTL — 24h vs shorter.** Wave plan says 24h. Real human latency between rendering a claim and deciding to give feedback could be days. 24h enforces "you must still have the page open." Shorter (e.g., 1h) would tighten the replay window. **Recommended:** 24h (matches wave plan); revisit at L4 if users complain.
3. **`SurfaceInappropriate` + `NotRelevantHere` variants in W4 scope?** These two ADR-0123 variants don't change claim truth state; they're surface-suppression markers. Including them in W4 doesn't add substrate risk but expands affordance UI surface area. **Recommended:** YES, ship all 9 — partial coverage is worse UX than complete.
4. **Audit redaction for `user_intent_text` — do we hash in the row or store cleartext + filter at projection?** Both work. Hash-in-row makes audit incident response harder (can't recover the original text); cleartext + filter is the W2 DOS-477 pattern. **Recommended:** cleartext + projection-layer filter, consistent with W2.
5. **Plugin admin UI for sweep job health?** A small admin panel showing sweep cron last-run + un-consumed nonce count. **Recommended:** NO for v1.4.3; file to v1.4.4 admin-UX scope.
6. **JS bundle size impact.** Adding a 9-action menu + textarea + state machine to a single block adds ~5-10 KB. Acceptable? **Recommended:** YES; bundle stays under 50KB per block budget.

## 13. v1.4.4+ + backlog lineage

| Item | Where it lives now | What v1.4.x / backlog does |
|---|---|---|
| Feedback affordance on briefing/meeting/project composites | W4 ships on `account-overview` only | v1.4.4 surface migration — adds affordance per composite. |
| Per-claim action filtering (some claim types support fewer variants) | W4 ships all 9 variants for every claim | v1.4.4 surface migration — adds per-claim-type action allowlist. |
| Plugin admin: sweep health panel | not in W4 | v1.4.4 admin UX. |
| MCP surface: feedback writes via ability invocation (not just click-bound) | not in W4 | v1.5.x — feedback-as-ability requires re-thinking actor-attribution per ADR-0111. |
| Causal lineage feedback signals → claim trust | partial via `enqueue_signals_for_feedback` | v1.5.x — full causal lineage substrate (per memory `project_causal_lineage_deferred.md`). |
| Recommendations layer driven by feedback signals | not in W4 | v1.5.x (per memory `project_recommendations_layer_vision.md`). |

## 14. Acceptance criteria

A. **`surface_feedback_nonces` table exists.** Migration v181 + v182 land cleanly; rollback path documented for v183.
B. **`services::surface_feedback::{issue_nonce, consume_nonce, expire_nonces_sweep}` ship.** All 3 pass `ctx.check_mutation_allowed()`; all 3 emit correct audit events.
C. **`POST /wp-json/dailyos/v1/feedback` works for both phases.** Permission + input validation enforced; signed runtime calls work.
D. **JS feedback affordance renders on `dailyos/account-overview`.** All 9 actions selectable; textarea appears for `NeedsNuance` + (optional) `WrongSubject`/`WrongSource`.
E. **End-to-end fixture passes for 4 representative variants** (`MarkOutdated`, `MarkFalse`, `WrongSubject`, `NeedsNuance`).
F. **Replay rejection works.** Second consume of the same nonce rejected + audit event emitted.
G. **Expiry sweep works.** Un-consumed nonces past TTL reaped + audit event emitted.
H. **Actor mismatch rejected.** Cross-user replay rejected + audit event.
I. **`user_intent_text` never leaks to non-originating actors** (W2 DOS-477 channel list extended; PHPUnit covers).
J. **CI gates pass.** §9 invariants enforced.
K. **L2 unanimous APPROVE** per §11 reviewer matrix, bounded by AC A–J.
L. **L4 hands-on** — 12 screenshot pairs captured (parked end-of-batch per James).
M. **CSO + security-auditor APPROVE** (mandatory L0 + L2 review per Amendment 3).
N. **No regressions in W2 primitive blocks or W3 magazine theme.**

## 15. Lock criteria

Cycle 1 reviewer outputs captured in `.docs/plans/v1.4.3-wp-foundation/reviews/packet-F-{code-reviewer, design-reviewer, codex-consult, codex-challenge, cso, security-auditor}-cycle1.md`. Pack locks when:
- Verdict set is unanimous APPROVE *or* unanimous-equivalent (≤2 reviewers CONDITIONAL with strictly surgical conditions folded into V1.x; 0 BLOCK; 0 active class-pattern).
- All folded conditions verifiable from packet text alone.
- No reviewer dissent on critical findings (per memory `feedback_reviewer_dissent_is_signal.md`).
- CSO + security-auditor unanimous APPROVE (mandatory gates).

Per `feedback_review_loop_l6_policy.md`: 15-cycle hard cap; class-pattern sweeps at 2-similar-findings; convergence rule allows surgical CONDITIONALs to lock.
