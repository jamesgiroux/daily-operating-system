# L0 packet — v1.4.6 W2 — DOS-331 / DOS-334

## Status

- Branch: `codex/v1.4.6-w2-surfacing-triggers`
- Base: `public/dev` at `c2167424` (`Add salience scoring read path (#401)`)
- Issues: DOS-331 surfacing policy, DOS-334 trigger policy
- Scope: one coordinated W2 PR; two logical lanes sharing migration/devtools registration files and joined by the trigger -> surfacing handoff
- L0 state: PASS after 2026-05-26 engineering-ladder + producer-era reconciliation; product/K-in/security/Codex challenge all passed after amendment cycles

## 0. Ladder metadata and scope

- Scope tier: **Standard with Amendment 3 add-on**. W2 is one bounded recommendation-domain PR, but it touches migrations, write paths, signal emission, provenance/privacy payloads, and surfacing policy.
- Origination class: **Extension**. W2 extends the v1.4.6 W1 recommendation and salience substrate; it is not a debug-driven cure for a traced symptom and not greenfield substrate.
- Trust topology: **local-to-local single-user**. W2 does not add MCP, SurfaceClient, remote, or multi-user exposure. Privacy-safe signal/audit payloads, source hygiene, prompt-injection discipline for external evidence inputs, and ADR-0108 log redaction still apply.
- Current ladder source: `.docs/plans/engineering-ladder.md` / `.docs/plans/engineering-ladder.html`, adopted 2026-05-18 and revised 2026-05-23.
- L0 panel for this amendment: `/codex challenge` + `ce-product-lens-reviewer` + `ce-security-lens-reviewer`, with `ce-learnings-researcher` K-in in parallel. Product lens reviews thresholds/budgets/cooldowns. Security lens reviews privacy-safe payloads, no direct bus semantics, and no surface/MCP authority.
- L2 router expectation if L0 passes: `/codex review` through `l2-bounded-reviewer`, plus `ce-data-migrations-reviewer` and `pr-review-toolkit:silent-failure-hunter` unless the diff shape changes. Advisory reviewers file path-alpha findings to maintenance unless they are AC/ADR/touched-regression bound.

## Goal

W1 made recommendations claim-backed and salience-scored. W2 decides what happens next:

- DOS-331 answers: render, defer, or suppress this recommendation now?
- DOS-334 answers: why did this candidate refresh run, what did it consider, and which downstream policy owned the result?

This is not notification automation and not a new queue. It is inspectable policy and audit state for existing claim/salience contracts.

## Producer classification

W2 is not a new durable recommendation producer. It coordinates derived policy/evaluation state over recommendation claims created by W1-A/upstream producers.

| Artifact | Class | W2 rule |
|---|---|---|
| `RecommendationClaim` writes | Durable claim producer | Out of scope. W2 reads active `ClaimType::Recommendation` claims and never bypasses `services::claims::commit_claim`. |
| `salience_evaluations` / factor rows | Derived read-model / evaluation evidence | W2-A may require stored salience evaluation ids before writing surfacing decisions. Salience does not mutate trust or claim lifecycle by itself. |
| `surfacing_decisions` | Derived policy / audit state | W2-A writes render/defer/suppress decisions and safe reasons. The table is not a surface API and not a second claim authority. |
| `triggers_log` | Operational audit state | W2-B logs run ids, trigger classes, safe source tokens, and returned salience/surfacing ids. No raw paths, claim text, prompt/output bodies, provider text, or source bodies. |
| `why_this_now` | Deterministic render helper | Template over typed factor and trigger refs only. No LLM calls and no raw source interpolation. |
| Future WP/MCP/Tauri consumers | Ability producer / projection | Out of scope for W2. Later lanes must expose an ability-backed recommendation projection before any surface consumes W2 state. |

## Non-goals

- No changes to `signals::bus` semantics.
- No changes to `signals/policy_registry.rs`; W0 already predeclared `SalienceCandidateRefreshTriggered` and `SurfacingDecisionMade`.
- No changes to `services/claims.rs`, `commit_claim`, trust recompute, or claim lifecycle helpers.
- No MCP exposure, SurfaceClient exposure, or Tauri UI work.
- No WP block or direct surface reader work. W2 emits/logs; W3+ owns ability-backed rendering.
- No `external` surface class, external trigger disposition, or consumer-facing delivery semantics. External/MCP/surface consumer behavior requires a later L0 amendment.
- No LLM/provider calls in recommendation policy modules.
- No new recommendation feedback table. W4-A owns the feedback wrapper.

## Current-code anchors

- W2 wave plan: `.docs/plans/v1.4.6-waves.md:826`
- Signal variants already present: `src-tauri/src/signals/policy_registry.rs`
- Service signal facade: `src-tauri/src/services/signals.rs`
- W1-A frozen contracts: `src-tauri/src/services/recommendations/contracts.rs`
- W1-B salience read/write split: `src-tauri/src/services/recommendations/salience.rs`
- Recommendation module skeleton: `src-tauri/src/services/recommendations/{surfacing,triggers,why_this_now}.rs`
- Current migrations registered through v270: `src-tauri/src/migrations.rs`

## K-in findings

- ADR-0125 makes recommendations first-class claims. W2 reads `intelligence_claims` rows where `claim_type = 'recommendation'`; it does not create parallel recommendation state.
- ADR-0126 warns against entity-wide attention budgets and hard-coded magic thresholds. W2 budgets are keyed by `actor_kind + local_day + claim_type + sensitivity + surface_class`, with the exact render surface stored for audit. The policy defaults are versioned rows for the `recommendation` claim type, not global entity constants.
- ADR-0115 requires registry-backed signal policy. W2 emits predeclared typed invalidation signals by canonical string through `services::signals::emit_once_for_key_and_propagate`, passes the `PropagationEngine` through W2 service APIs, and never reaches into `signals::bus` directly.
- ADR-0108 privacy rules apply to rendered/audit payloads: no raw claim text, raw paths, prompt/output bodies, or free-text rationale. W2 stores opaque IDs, enum reason codes, factor kinds, scores, and bounded typed trigger refs.
- Existing `claim_feedback`, `claim_surface_dismissals`, and ADR-0123 feedback actions are enough for "recently dismissed/suppressed" reads. W2 may read current feedback/dismissal rows; W4-A later owns recommendation-specific feedback writes.

## Product policy

Policy version: `recommendation_surfacing_v1`.

W2 treats these as the v1 claim-type surfacing policy for `ClaimType::Recommendation`. The migration materializes versioned policy defaults in `recommendation_surfacing_policy` because the existing `ClaimTypeMetadata` registry does not yet have budget fields. This is not a generic budget substrate and not a user setting; it is an auditable recommendation-policy table keyed the same way ADR-0126 requires budgets to be reasoned about.

Budget key:

`actor_kind + local_day + claim_type + sensitivity + surface_class`

The exact `render_surface` is still recorded on every decision row, but primary recommendation volume is shared across primary app surfaces so the same user cannot receive three "top three" lists from briefing, entity detail, and work surfaces on the same local day.

`surface_class` is a closed W2 enum: `primary | background | quiet | review`. `quiet` is first-class auditable policy state and must not be collapsed into `background`; `external` is explicitly out of scope for W2.

Default surfacing thresholds, v1:

| Condition | Decision |
|---|---|
| claim not active/currently surfaceable | `Suppress(BelowThreshold)` with audit code `claim_not_surfaceable` |
| `claim_surface_dismissals` hides this claim on the requested surface | `Suppress(DismissedRecently)` |
| recent claim feedback suppresses this recommendation family | `Suppress(DismissedRecently)` |
| `salience.total >= 0.85` or urgency factor `>= 0.90` | `Render(Critical)` |
| `salience.total >= 0.68` and budget remains | `Render(Notable)` |
| `salience.total >= 0.68` and budget exhausted | `Defer(BudgetExhausted)` until next local day window |
| `0.45 <= salience.total < 0.68` | `Render(Background)` for Activity Log / background slots only |
| low-confidence but potentially important candidate | `Defer(AwaitingCorroboration)` |
| active trigger is still pending | `Defer(PendingTrigger)` |
| otherwise | `Suppress(BelowThreshold)` |

Budget defaults, v1:

- `Critical` does not consume the Notable budget, but all primary Critical renders are still bounded: max 1 Critical render per `actor_kind + local_day + claim_type + sensitivity + surface_class`. Additional Critical candidates always defer for the primary surface with `BudgetExhausted` until the next local day. If the overflow candidate has urgency `>= 0.95` and material new evidence, W2 may also record a review/background overflow outcome grouped by subject/action, but it must not create a second primary Critical render for the same budget key.
- `Notable` consumes a shared primary daily budget of 3 for `recommendation/internal/primary` for the actor's local day.
- `Background` and `Quiet` do not consume Notable budget, but `Background` has an Activity Log cap of 10 per actor/local day/surface_class and groups by subject/action when over cap.
- Claim-level cooldown prevents the same claim from rendering in primary slots for 7 days.
- Subject/action cooldown prevents the same `subject_kind + subject_id + action_signature` from rendering in primary slots for 3 days. `action_signature` is derived from W1-A `recommendation::action_key`; no semantic embedding or raw action text is used.
- Recently suppressed feedback blocks resurfacing for 14 days. Critical/urgent candidates may override only when the trigger carries material new evidence after the dismissal timestamp: newer `source_asof`, changed `evidence_signature`, or changed subject/entity version. A new `source_signal_id` alone supports audit/dedupe, but it does not override dismissal or feedback suppression.

Policy rationale:

| Policy value | User-facing intent | Expected volume impact | False-positive risk | False-negative risk | DOS-338 validation |
|---|---|---|---|---|---|
| Critical threshold `0.85` or urgency `0.90` | Show the rare thing that should interrupt the normal shortlist. | At most 1 primary Critical per local day by storm guard. | Alarm fatigue if overflow review/background grouping fails. | A second urgent issue may be held outside primary until review/background or next day. | `high_signal_critical`, `critical_overflow_bounded`, `missed_important_urgent` |
| Notable threshold `0.68`, budget `3` | Keep the main "what matters today" list to the morning top three. | Max 3 primary Notable recommendations per actor/local day/surface class. | Borderline items may crowd out truly useful later items. | Some good-but-not-top-three items wait until tomorrow or background. | `high_signal_notable`, `low_signal_stays_quiet`, `primary_budget_cap` |
| Background threshold `0.45`, cap `10` | Preserve inspectability without creating primary-surface noise. | Activity Log can show lower-confidence candidates, grouped by subject/action. | Activity Log clutter if cap/grouping fails. | User may not see a weak early signal until it grows. | `background_grouping`, `low_signal_activity_log_only` |
| Claim cooldown `7 days` | Avoid repeating the same recommendation after the user has seen it. | Same claim cannot occupy primary slots repeatedly within a week. | A still-relevant item may stay hidden too long. | Urgent same-claim changes could be delayed. | `stale_same_claim_cooldown`, `material_new_evidence_breaks_cooldown` |
| Subject/action cooldown `3 days` | Avoid showing paraphrases of the same action. | Similar same-subject action family appears once per 3 days. | Generic action signatures may over-suppress related but distinct actions. | A second valid action of same kind may wait. | `subject_action_repeat_suppressed`, `distinct_action_allowed` |
| Feedback suppression `14 days` | Respect explicit user judgment. | Dismissed/noisy recommendations stay suppressed unless the world changes. | User might dismiss too broadly. | Important recurrence could stay hidden without material evidence. | `suppressed_requires_new_evidence`, `noisy_fixture_stays_quiet` |

## DOS-331 implementation shape

Owned files:

- `src-tauri/src/services/recommendations/surfacing.rs`
- `src-tauri/src/services/recommendations/why_this_now.rs` for deterministic text helper only
- Migration `src-tauri/src/migrations/271_recommendation_surfacing.sql`
- `src-tauri/src/migrations.rs`
- `src-tauri/src/devtools/mod.rs` seed/cleanup for new mock-data gate

Service API:

```rust
pub fn decide_surfacing(candidate: SurfacingCandidate, policy: SurfacingPolicy, now: DateTime<Utc>) -> SurfacingDecision;

pub fn evaluate_surfacing_for_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: SurfacingEvaluationInput,
) -> Result<SurfacingEvaluation, SurfacingError>;
```

The pure function computes the decision. The service function is the write path:

1. `ctx.check_mutation_allowed()?`
2. Load the recommendation claim by id, including `claim_type`, `sensitivity`, lifecycle state, subject, metadata evidence, and W1-A action key.
3. For mutation paths, W2-A is the sole owner of durable salience recompute: call W1-B `recompute_salience_for_claim` first and require `SaliencePersistence::Stored { evaluation_id }`. Pure/evaluate-only paths may use preview salience but must not write `surfacing_decisions` or emit signals.
4. Read `recommendation_surfacing_policy` for `recommendation_surfacing_v1` and derive the ADR-0126 budget key.
5. Read recent `surfacing_decisions` for budget/cooldown state.
6. Read existing `claim_surface_dismissals` for the requested `ClaimDismissalSurface` and `claim_feedback` for broader suppression hints.
7. Build sanitized `TriggerRef` values and deterministic `WhyThisNow`.
8. Insert one `surfacing_decisions` row for every mutation-path evaluation.
9. Emit `surfacing_decision_made` through `services::signals::emit_once_for_key_and_propagate` with source `recommendation_surfacing_policy` and a safe JSON value containing only IDs, policy version, decision kind, tier/reason code, and budget key.
10. Return `SurfacingEvaluation` with the stored `salience_evaluation_id`, `surfacing_decision_id`, signal outcome, derived signal IDs, and decision. Trigger scans must copy these returned IDs into `triggers_log`; they must not recompute salience independently.

`why_this_now.text` is deterministic template output:

`Salience driven by {factor_label}: {typed_reason}. Triggers: {trigger_summary}.`

No raw claim text is interpolated. Factor and trigger summaries come from closed enums and numeric fields.

Policy table shape (`recommendation_surfacing_policy`):

- `policy_version`
- `claim_type`
- `sensitivity`
- `surface_class`: `primary | background | quiet | review`
- `critical_threshold`
- `urgent_threshold`
- `notable_threshold`
- `background_threshold`
- `critical_daily_cap`
- `notable_daily_budget`
- `background_daily_cap`
- `claim_cooldown_days`
- `subject_action_cooldown_days`
- `feedback_suppression_days`
- `policy_source`: `claim_type:recommendation`
- `created_at`

Decision table shape (`surfacing_decisions`):

- `id`
- `evaluation_id`
- `claim_id`
- `subject_kind`, `subject_id`
- `claim_type`
- `sensitivity`
- `decision_kind`: `render | defer | suppress`
- `surfacing_tier`
- `reason_kind`
- `defer_until`
- `why_this_now_json`
- `trigger_refs_json`
- `salience_total`
- `primary_factor_kind`
- `salience_evaluation_id`
- `policy_version`
- `policy_source`
- `render_surface`
- `surface_class`
- `actor_kind`
- `local_day`
- `budget_key`
- `budget_limit`
- `budget_used_before`
- `cooldown_until`
- `action_signature`
- `evidence_signature`
- `material_evidence_after`
- `created_at`

Indexes:

- `(claim_id, created_at DESC)`
- `(subject_kind, subject_id, created_at DESC)`
- `(budget_key, created_at DESC)`
- `(subject_kind, subject_id, action_signature, created_at DESC)`
- `(render_surface, created_at DESC)`
- `(decision_kind, created_at DESC)`
- unique idempotency index on `evaluation_id`

## DOS-334 implementation shape

Owned files:

- `src-tauri/src/services/recommendations/triggers.rs`
- Migration `src-tauri/src/migrations/272_recommendation_triggers_log.sql`
- `src-tauri/src/migrations.rs`
- `src-tauri/src/devtools/mod.rs` seed/cleanup for new mock-data gate

Service API:

```rust
pub fn classify_trigger(input: TriggerPolicyInput, now: DateTime<Utc>) -> TriggerPolicy;

pub fn run_trigger_scan(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: TriggerScanInput,
) -> Result<TriggerScanOutcome, TriggerError>;
```

The pure classifier makes trigger classes typed and inspectable. The service function is the write path:

1. `ctx.check_mutation_allowed()?`
2. Resolve the input into an internal `TriggerSourceClass`, frozen `TriggerKind`, subject/window, canonical signal/source type, and safe source token. `source_signal_type` is never caller-provided raw text; it is a registry-backed signal name or allowlisted policy token.
3. Compute `dedupe_key`, `suppression_key`, `trust_floor`, `freshness_window_secs`, `trigger_disposition`, and `reason_code` from normalized tokens, opaque IDs, and keyed signatures only.
4. Check existing `triggers_log` rows for the same `dedupe_key + policy_version`.
   - `completed` inside the suppression window returns a coalesced outcome without a new scan.
   - fresh `started` returns in-progress/coalesced.
   - `failed_retryable` may insert a new attempt when `next_retry_at <= now`.
5. Insert a `started` `triggers_log` row for the attempt. The unique idempotency key is `run_id`, not `dedupe_key`.
6. Emit `salience_candidate_refresh_triggered` through `services::signals::emit_once_for_key_and_propagate` with source `recommendation_trigger_policy` and a safe JSON value containing only `run_id`, trigger class, subject/entity IDs, and reason code.
7. Find recommendation claims for the subject/window.
8. Hand each candidate into W2-A `evaluate_surfacing_for_claim(ctx, db, engine, ...)`. W2-B never calls `recompute_salience_for_claim` directly.
9. Copy `candidate_claim_ids_json`, `salience_evaluation_ids_json`, and `surfacing_decision_ids_json` from the W2-A outcomes. These IDs must match the `surfacing_decisions.salience_evaluation_id` and decision IDs W2-A wrote.
10. Update the `triggers_log` row to `completed` with candidate, salience, and surfacing decision IDs. On downstream failure, update to `failed_retryable` or `failed_terminal` with a closed error code and no raw error text.

Trigger class defaults:

| Trigger source class | Frozen `TriggerKind` | Trigger audit disposition | W2-A `surface_class` handoff | Trust floor | Freshness window | Dedupe key | Suppression key | Dedupe breaks when |
|---|---|---|---|---|---|---|---|---|
| `scheduled_freshness` | `ScheduledScan` | `silent_prepare` | `background` | `0.60` | `86400s` | `scheduled:{local_day}:{subject}:{surface_class}` | `subject_action:{subject}:{action_signature}` | local day changes or material evidence changes |
| `event_invalidation` | `SignalArrival` | `primary_candidate` | `primary` | `0.70` | `21600s` | `signal:{source_signal_id}:{subject}` | `signal:{canonical_signal_type}:{subject}` | new signal id or subject version changes |
| `manual_refresh` | `SignalArrival` | `review_candidate` | `review` | `0.50` | `0s` | `manual:{run_id}` | `manual:{actor_kind}:{subject}` | every explicit run id |
| `entity_change` | `EntityChange` | `primary_candidate` | `primary` | `0.65` | `21600s` | `entity:{entity_type}:{entity_id}:{subject_version}` | `entity:{entity_type}:{entity_id}` | subject/entity version changes |
| `claim_change` | `SignalArrival` | `primary_candidate` | `primary` | `0.70` | `21600s` | `claim:{claim_id}:{claim_version}` | `claim:{claim_id}` | claim version changes |
| `source_change` | `SignalArrival` | `review_candidate` | `review` | `0.75` | `21600s` | `source:{source_signal_id}:{subject}` | `source:{source_fingerprint}:{subject}` | new source signal or newer `source_asof` |
| `open_loop_change` | `SignalArrival` | `quiet_candidate` | `quiet` | `0.55` | `43200s` | `open_loop:{subject}:{open_loop_id}:{state}` | `subject_action:{subject}:{action_signature}` | open-loop state changes |
| `meeting_window` | `EntityChange` | `primary_candidate` | `primary` | `0.65` | `7200s` | `meeting:{meeting_id}:{subject}:{window_start}` | `meeting:{meeting_id}:{subject}` | meeting window rolls or meeting evidence changes |
| `decision_window` | `ScheduledScan` | `primary_candidate` | `primary` | `0.70` | `43200s` | `decision:{subject}:{decision_date}:{action_signature}` | `subject_action:{subject}:{action_signature}` | decision window/date or material evidence changes |
| `feedback_echo` | `FeedbackEcho` | `quiet_candidate` | `quiet` | `0.50` | `604800s` | `feedback:{feedback_id}:{claim_id}` | `feedback:{claim_id}` | new feedback id |

`trigger_disposition` is an internal audit/handoff hint only. It is not a delivery channel, notification target, surface API, or permission to render. W2-B passes the mapped W2-A `surface_class` into `evaluate_surfacing_for_claim`; W3+ ability projections decide whether anything is rendered.

Table shape:

- `id`
- `run_id`
- `policy_version`
- `trigger_kind`
- `trigger_class`
- `source_signal_id`
- `source_signal_type` (canonical registry/allowlisted token; never raw caller text)
- `entity_type`, `entity_id`
- `subject_ref_json`
- `window_start`, `window_end`
- `dedupe_key`
- `suppression_key`
- `trust_floor`
- `freshness_window_secs`
- `trigger_disposition`: `silent_prepare | primary_candidate | review_candidate | quiet_candidate`
- `evidence_ref_ids_json`
- `evidence_signature`
- `candidate_claim_ids_json`
- `salience_evaluation_ids_json`
- `surfacing_decision_ids_json`
- `status`: `started | completed | failed_retryable | failed_terminal` (`coalesced` is an outcome returned to callers, not a stored row status)
- `result_kind`: `prepared_silently | render_decision_recorded | held_for_review | stayed_quiet | failed`
- `downstream_policy`
- `reason_code`
- `attempt_count`
- `next_retry_at`
- `error_code`
- `created_at`, `completed_at`

Indexes:

- unique idempotency index on `run_id`
- `(dedupe_key, policy_version, status, next_retry_at)`
- `(trigger_kind, created_at DESC)`
- `(trigger_class, created_at DESC)`
- `(entity_type, entity_id, created_at DESC)`
- `(result_kind, created_at DESC)`

Persisted `triggers_log` string/JSON privacy matrix:

| Field | Privacy class | Construction rule |
|---|---|---|
| `id` | generated row ID | Generated by W2 storage code; never copied from caller text. |
| `run_id` | generated/validated opaque run ID | Generated by W2 or validated as UUID/ULID-like opaque ID before storage/key use. |
| `policy_version` | closed policy code | Constant allowlisted policy version such as `recommendation_trigger_policy_v1`. |
| `trigger_kind` | closed enum code | Serialized from frozen `TriggerKind`. |
| `trigger_class` | closed enum code | Serialized from internal `TriggerSourceClass`. |
| `source_signal_id` | opaque signal ID | Existing signal row ID only; nullable when no source signal exists. |
| `source_signal_type` | canonical registry/allowlisted token | Registry-backed signal name or W2 allowlisted source token; never caller raw text. |
| `entity_type` | closed enum/allowlisted token | Internal entity kind token only. |
| `entity_id` | opaque entity ID | Existing entity ID only; no display name/domain/path text. |
| `subject_ref_json` | JSON subject ID object | Subject kind + opaque subject ID only; no names, claim text, or source strings. |
| `dedupe_key` | normalized keyed policy key | Built only from closed prefixes, opaque IDs, canonical signal/source types, local-day/window tokens, and keyed signatures/hashes such as `action_signature` or `source_fingerprint`. |
| `suppression_key` | normalized keyed policy key | Same constraints as `dedupe_key`; no raw source, action, path, or signal text. |
| `trigger_disposition` | closed enum code | One of `silent_prepare | primary_candidate | review_candidate | quiet_candidate`; audit/handoff hint only, never delivery authority. |
| `evidence_ref_ids_json` | JSON opaque ID array | Evidence IDs only; no `EvidenceRef.source` or source body. |
| `evidence_signature` | keyed signature/hash | Stable keyed signature over evidence identity/materiality; no raw evidence content. |
| `candidate_claim_ids_json` | JSON opaque ID array | Claim IDs only; no claim text or recommendation text. |
| `salience_evaluation_ids_json` | JSON opaque ID array | W2-A returned salience evaluation IDs only. |
| `surfacing_decision_ids_json` | JSON opaque ID array | W2-A returned surfacing decision IDs only. |
| `status` | closed enum code | One of `started | completed | failed_retryable | failed_terminal`. |
| `result_kind` | closed enum code | One of `prepared_silently | render_decision_recorded | held_for_review | stayed_quiet | failed`; `render_decision_recorded` means W2-A recorded a render decision, not that any user-facing surface rendered it. |
| `downstream_policy` | closed allowlisted policy token | `recommendation_surfacing_policy` or another explicitly allowlisted W2 policy token only. |
| `reason_code` | closed enum code | Trigger policy reason enum; no free text. |
| `error_code` | closed enum/code | Nullable closed error code only; raw downstream errors are logged nowhere in W2 tables/signals. |

The privacy regression must be schema-driven: it inventories every persisted `triggers_log` string/JSON column and fails if a current or future string/JSON column is missing from this matrix or the sentinel scan. Numeric/timestamp columns are type-constrained, but any serialized signal payload string leaves are scanned under the same sentinel rule.

## Privacy and trust boundaries

- Store IDs, enum codes, factor kinds, timestamps, numeric scores, and trigger disposition only.
- Do not store raw recommendation/claim text in W2 tables.
- Do not store raw source paths, prompt text, provider output, or user-authored free text.
- Every string/JSON field in the `triggers_log` privacy matrix must be assembled from allowlisted IDs, canonical enum/signal names, opaque refs, JSON ID arrays, closed policy/error codes, or keyed hashes/signatures. They must never include caller-provided raw signal names, file paths, prompt/provider strings, claim text, user-authored free text, raw downstream error text, or raw evidence source strings.
- `error_code` is a closed enum/code only. Raw downstream error messages are not persisted in `triggers_log` or emitted in signal payloads.
- `why_this_now.rs` owns a closed template renderer. It must not use `Debug` formatting for rationale or trigger inputs, because that could serialize raw source strings.
- `TriggerRef.source` is never caller-provided. W2 maps all inputs to allowlisted source tokens such as `scheduled_scan`, `signal_event`, `entity_change`, `manual_refresh`, and `feedback_echo`.
- Signal emission uses constant sources (`recommendation_surfacing_policy`, `recommendation_trigger_policy`) and safe JSON values. No raw paths, claim text, prompt/provider output, or free-form error messages in `signal_events.value`.
- Evidence references written by W2 are opaque IDs or stable hashes. W2 does not serialize W1-A `EvidenceRef.source` directly unless it has passed through the same source-token allowlist used for `TriggerRef.source`.
- Existing `claim_feedback.payload_json` remains owned by `services::claims`; W2 reads only action, claim, timestamp, and allowlisted metadata keys needed for surface dismissal/suppression.
- Mutating service functions reject Evaluate/Simulate mode before writing tables or signals.

## Tests

Migration tests:

- v271 creates `recommendation_surfacing_policy` + `surfacing_decisions` with indexes and retry-safe idempotency.
- v272 creates `triggers_log` with retry status fields and indexes, and does not put a unique index on `dedupe_key`.
- current schema version reaches v272.

Surfacing tests:

- high-urgency candidate renders `Critical`.
- second Critical on the same actor/local day/surface class defers for primary even with urgency `>= 0.95` and material new evidence; the overflow may be held/grouped for review/background but cannot create a second primary Critical render.
- normal high-salience candidate renders `Notable` under budget.
- over-budget candidate defers with `BudgetExhausted`.
- recent surfacing cooldown defers with `CooldownActive`.
- recent `claim_surface_dismissals` row suppresses with `DismissedRecently` on that surface.
- recent feedback suppresses with `DismissedRecently` until material evidence arrives via newer `source_asof`, changed `evidence_signature`, or changed subject/entity version; a new `source_signal_id` alone does not override suppression.
- subject/action repeat suppresses or defers by the 3-day cooldown while a distinct action signature is allowed.
- low-confidence but important candidate defers with `AwaitingCorroboration`.
- low salience suppresses with `BelowThreshold`.
- `why_this_now` contains only deterministic enum/numeric summaries.
- raw-looking trigger source/path input is sanitized out of `trigger_refs_json`, `why_this_now_json`, and emitted signal value.
- Evaluate mode writes no `surfacing_decisions` row and emits no signal.
- `surfacing_decision_made` uses the propagating deterministic signal facade, and duplicate idempotency keys do not re-propagate.

Trigger tests:

- scheduled refresh logs trigger and invokes W2-A evaluation, which owns salience recompute.
- urgent signal refresh logs trigger and emits `salience_candidate_refresh_triggered`.
- duplicate dedupe key coalesces without duplicate scan rows.
- retryable partial failure can run again after `next_retry_at` and does not get blocked by the original dedupe key.
- retryable partial failure retry copies the W2-A returned salience evaluation IDs, and trigger-log salience IDs exactly match the surfacing decision IDs they caused.
- in-progress duplicate returns coalesced/in-progress instead of starting a competing scan.
- every trigger class maps to the expected frozen `TriggerKind`, trigger audit disposition, W2-A `surface_class` handoff, trust floor, freshness window, dedupe key, and suppression key.
- stale source stays quiet / held for review by explicit reason.
- low-confidence candidate carries explicit trust floor and goes to review/defer.
- user-suppressed candidate does not re-render.
- urgent-but-low-trust candidate goes to review/defer instead of primary render.
- noisy fixture stays quiet; high-signal fixture surfaces; primary daily volume stays within the budget key.
- privacy regression feeds raw-path-looking, prompt-looking, provider-output-looking, raw evidence-source-looking, raw error-looking, and free-text-looking sentinels through trigger input/evidence/error paths, inventories every persisted `triggers_log` string/JSON column against the privacy matrix above, and asserts every matrix field plus emitted signal payload string leaf contains only allowlisted IDs, enum/signal codes, opaque refs, keyed signatures/hashes, and closed error/policy codes.
- Evaluate mode writes no `triggers_log` row and emits no signal.
- `salience_candidate_refresh_triggered` uses the propagating deterministic signal facade, and duplicate idempotency keys do not re-propagate.

Validation commands:

- `cargo test --manifest-path src-tauri/Cargo.toml -p dailyos recommendations::surfacing --lib`
- `cargo test --manifest-path src-tauri/Cargo.toml -p dailyos recommendations::triggers --lib`
- `cargo test --manifest-path src-tauri/Cargo.toml -p dailyos migration_27 --lib`
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
- `pnpm tsc --noEmit`

## Rollout and coordination

- W2 is safe to implement now because W1-A and W1-B are merged into `dev`.
- The two W2 modules are logical lanes coupled by trigger -> salience -> surfacing handoff and shared registration files (`migrations.rs`, `devtools/mod.rs`), so a single coordinated PR is lower-risk than two racing PRs.
- If implementation discovers missing signal behavior, stop and coordinate before touching `signals::bus` or `signals/policy_registry.rs`.
- If implementation needs new claim lifecycle behavior, stop and coordinate before touching `services/claims.rs` or `commit_claim`.

## L0 reviewer prompts

1. Does the packet's §0 scope tier / origination / trust-topology declaration match the actual touched paths?
2. Does this policy improve the user's working understanding rather than increasing notification/UI volume?
3. Are budgets/cooldowns scoped narrowly enough per ADR-0126?
4. Does the trigger log explain enough without storing sensitive raw content?
5. Does W2 correctly reuse W1-B salience, W1-A recommendation claims, and existing service signal APIs without inventing substrate?
6. Does the producer classification hold: W2 writes derived audit/read-model state, not durable recommendation claims or surface authority?
7. Are there acceptance criteria from DOS-331 or DOS-334 not covered by tests?
8. Did K-in cite any `docs/solutions/` or `.docs/decisions/` entries that make any W2 substrate net-new claim invalid?
