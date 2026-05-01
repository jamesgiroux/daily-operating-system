# Implementation Plan: DOS-294

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-294 implements typed claim feedback as substrate semantics, not a UI survey. The ticket's load-bearing line is: "DailyOS needs typed feedback that can feed claim state, trust scoring, provenance, repair jobs, lint, and future prompts." The nine actions are closed: `confirm_current`, `mark_outdated`, `mark_false`, `wrong_subject`, `wrong_source`, `cannot_verify`, `needs_nuance`, `surface_inappropriate`, and `not_relevant_here`.

The trust/provenance split is also contractual. DOS-294 says: "`wrong_subject`: subject-fit failure; do not necessarily punish source", "`wrong_source`: attribution/source support failure; claim may still be true with other evidence", "`cannot_verify`: do not treat as false", and "`surface_inappropriate`: render/privacy feedback, not truth feedback." ADR-0123 closes the same shape: nine variants map to "a distinct triple of (claim state, source weight, agent ledger)" and reject "No 5-point scale. No yes/no" (`.docs/decisions/0123-typed-claim-feedback-semantics.md:89`).

Both 2026-04-24 amendments apply. DOS-294 comment amendment 1 says typed feedback "should be an exception/escalation mechanism, not the primary verification loop." DOS-294 comment amendment 2 says each feedback type must map to "claim state", "trust factor effect", "source reliability effect", "extractor reliability effect", "linker/subject-fit reliability effect", "attribution/source-support effect", "render policy", "repair job", and "tombstone/pre-gate behavior." DOS-307 is folded into this W3-E slot: "DailyOS owns claim verification by default" and "Only `needs_user_decision` should produce an explicit user task."

Current-code reality: `src-tauri/src/services/claims.rs` does not exist on this branch yet; DOS-7 creates it and the `claim_feedback` skeleton (`.docs/plans/wave-W3/DOS-7-plan.md:23-25`). Existing feedback is legacy field-level feedback: five `CorrectionAction` values in `src-tauri/src/db/feedback.rs:21-55`, append rows through `record_feedback_event` at `src-tauri/src/db/feedback.rs:75-95`, and direct dismissal tombstones at `src-tauri/src/services/intelligence.rs:1264-1288`.

## 2. Approach

Extend, do not fork, the DOS-7 `src-tauri/src/services/claims.rs` module. Add `FeedbackAction`, `ClaimFeedbackInput`, `ClaimFeedbackMetadata`, `ClaimFeedbackOutcome`, `ClaimVerificationState`, and `ClaimRenderPolicy` in that module unless DOS-7 already exposes a sibling claim model module. Register only service-level mutation entry points: `record_claim_feedback(ctx, db, input)` and pure helpers `feedback_semantics(action)` / `transition_for_feedback(current, action)`.

The `claim_feedback` table remains the append-only source of explicit user judgment per ADR-0123 (`.docs/decisions/0123-typed-claim-feedback-semantics.md:91-106`) and ADR-0126 (`.docs/decisions/0126-memory-substrate-invariants.md:44-50`, `:85-90`). W3-E writes rows, stamps `created_at` from `ctx.clock`, leaves `applied_at = NULL`, and lets DOS-5 consume un-applied rows. Re-application stays idempotent through `applied_at`.

Add a separate persisted system review state, not a new `claim_state` vocabulary: `ClaimVerificationState = Active | Contested | NeedsUserDecision`. DOS-7 owns lifecycle columns where `claim_state` is user intent and `surfacing_state` is rendering (`.docs/plans/wave-W3/DOS-7-plan.md:23`; ADR-0126 `.docs/decisions/0126-memory-substrate-invariants.md:20-40`). W3-E should coordinate with W3-C so the base schema includes `verification_state TEXT NOT NULL DEFAULT 'active'`, `verification_reason TEXT NULL`, and `needs_user_decision_at TEXT NULL`; if W3-C has already landed without them, W3-E adds the smallest migration for those mutable derived columns.

State machine: automatic processes may move `Active -> Contested -> NeedsUserDecision`; `NeedsUserDecision` is terminal for system-owned repair. Terminal means automated repair/trust recompute cannot silently move the same claim back to default rendering; only explicit user feedback, corrected evidence committed as a new claim, or an explicit contradiction reconciliation can close/supersede it.

Mutation algorithm: gate with `ctx.check_mutation_allowed()` like existing mutators (`src-tauri/src/services/feedback.rs:208-213`; `src-tauri/src/services/context.rs:35-61`); load claim by `claim_id` in the writer lane; validate action-specific metadata; insert `claim_feedback`; apply immediate lifecycle/render side effects in the same transaction; enqueue repair rows where applicable; bump per-entity claim invalidation through DOS-7's `commit_claim`/invalidation path; emit a user-readable activity signal after the transaction.

End-state alignment: this makes feedback a durable claim-adjacent contract consumed by Trust Compiler, provenance rendering, repair, lint, and evaluation. It forecloses untyped strings like the existing `feedback_type: "dismiss"` path (`src-tauri/src/services/intelligence.rs:1267-1278`) from becoming new claim semantics.

## 3. Key decisions

Feedback-action matrix:

| Action | Claim lifecycle / verification | Trust factor | Reliability effect | Repair job | Render policy |
|---|---|---|---|---|---|
| `ConfirmCurrent` | keep `claim_state=active`, set verification `Active`, refresh feedback evidence | `user_feedback_weight` boost; agent `alpha += 1.0` per ADR-0123 | source +0.10 capped | none | default render; show corroborated-by-user evidence |
| `MarkOutdated` | keep historical claim, set `surfacing_state=dormant`, `demotion_reason=outdated`, set `superseded_by` if correction claim exists | freshness/history downweight; agent `alpha += 0.5` | no source penalty | optional freshness/source-asof repair | hide from current-state surfaces; available in history |
| `MarkFalse` | `claim_state=withdrawn`; broad tombstone/pre-gate if no source still qualifies | strong negative; agent `beta += 1.0` | modest source -0.05 one-shot | contradiction/tombstone reconcile | suppress default render; show only audit/history |
| `WrongSubject` | per-subject tombstone on asserted subject; propose corrected-subject claim if supplied | truth not directly false; agent `beta += 0.3` | linker/subject-fit -0.3; source unchanged | subject-fit repair | suppress on asserted subject; do not suppress corrected subject path |
| `WrongSource` | keep active if other evidence qualifies; otherwise `withdrawn`; verification `Contested` | attribution caveat; agent `beta += 0.2` | source-attribution -0.20 for source/claim_type | source support repair | render with source caveat or suppress if unsupported |
| `CannotVerify` | no truth/lifecycle change; verification `Contested` with `needs_corroboration` reason | no direct delta | none | enqueue corroboration repair, capped per ADR-0123 | qualify as needs evidence; no user task by itself |
| `NeedsNuance` | original `surfacing_state=dormant` + `superseded_by` user-authored claim | alpha or beta 0.3 by text-overlap heuristic | source inherited on original, no source penalty | optional contradiction/merge repair | render superseding qualified claim |
| `SurfaceInappropriate` | no truth state change; surface-specific suppression marker | no delta | none | optional policy/sensitivity repair | hide only on named surface; feeds privacy/render policy |
| `NotRelevantHere` | no truth state change; context-binding hint only | no delta | none | no repair; eval/relevance signal | hide/deprioritize only for invocation/context |

UI scope: W3-E does not implement React render-time controls. It ships typed enum/schema, service API, command DTO if needed by existing Tauri boundary, compact-label mapping (`Accurate`, `Outdated`, `Wrong`, `Can't verify`, overflow labels), and render-policy outputs. ADR-0123 explicitly targets "v1.4.0 substrate" and defers UI beyond inline to v1.4.2 (`.docs/decisions/0123-typed-claim-feedback-semantics.md:5`, `:167-173`); DOS-8/W6-D consume the API.

`needs_user_decision` render consumption: default claim loaders must exclude it from ordinary surfaces and return a `ReviewRequired`/`HelpDecide` marker only through an explicit review-queue/read-stub path. W3-E adds the test stub required by the wave plan (`.docs/plans/v1.4.0-waves.md:498-501`); production UI is later.

Activity records: `claim_feedback` is the trust source of truth; user-readable activity is a derived event. W3-E emits `claim_feedback_recorded` / `claim_verification_state_changed` into the persisted signal/activity stream after commit, following the existing feedback signal pattern in `src-tauri/src/services/feedback.rs:313-342`. The payload must contain action label, claim id, subject ref, surface/invocation if present, and a non-sensitive summary; it must not be consumed by Trust Compiler.

Action metadata validation is strict: `WrongSubject` accepts optional `corrected_subject`; `WrongSource` requires a source ref/index that exists on the claim provenance; `NeedsNuance` requires non-empty corrected text only for claim types ADR-0123 allows (`.docs/decisions/0123-typed-claim-feedback-semantics.md:117`); `SurfaceInappropriate` requires surface; `NotRelevantHere` requires invocation id.

## 4. Security

The new attack surface is an IPC/service mutation that can affect trust, rendering, repair, and tombstones. Validate `claim_id` exists, the claim subject belongs to the current local workspace, the supplied subject correction is allowed for the claim type, the source ref points to a source already attached to the claim, and free-text note/correction length is capped and stored as user content.

Never trust UI labels. Existing commands pass raw strings into `CorrectionAction::parse` (`src-tauri/src/commands/people_entities.rs:644-669`); W3-E should parse only the closed snake_case enum and reject unknown values before any DB write.

Do not log claim text, corrected text, notes, source excerpts, or customer names. Activity/signal rows use user-readable but bounded labels and ids; raw text stays in `claim_feedback.note` / corrected claim payload behind normal local data protections. `SurfaceInappropriate` is a privacy/sensitivity signal, not a truth contradiction, so it must not downrank the source or claim.

Cross-entity bleed is highest risk for `WrongSubject`. The implementation must tombstone only the asserted subject and route a corrected-subject claim through `commit_claim` pre-gates, matching ADR-0123 (`.docs/decisions/0123-typed-claim-feedback-semantics.md:146-153`) and ADR-0113 tombstone pre-gate ordering (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:114-122`).

## 5. Performance

Hot path cost is one writer-lane transaction: indexed claim lookup, one `claim_feedback` insert, a bounded set of lifecycle column updates, optional repair-job insert, and per-entity invalidation. Required indexes: `claim_feedback(claim_id, applied_at, created_at DESC)`, `claim_feedback(action, applied_at)`, `intelligence_claims(subject_ref, verification_state, surfacing_state)`, and repair queue uniqueness for `(claim_id, repair_kind, status)`.

Default render paths should not scan feedback history. They consume denormalized `verification_state`, `surfacing_state`, `demotion_reason`, and trust columns on `intelligence_claims`; Trust Compiler consumes un-applied feedback in batches. `CannotVerify` must enqueue at most one active repair job per claim/action and honor ADR-0123 caps: one LLM call, one retrieval batch, max 5 active jobs per entity, max 50 workspace-wide (`.docs/decisions/0123-typed-claim-feedback-semantics.md:155-165`).

Claim feedback invalidates dependent surfaces without full regeneration by bumping the per-entity claim invalidation primitive DOS-7 already uses (`.docs/plans/wave-W3/DOS-7-plan.md:27`). No `entity_graph_version` singleton bump and no legacy `intelligence.json` rewrite in W3-E.

## 6. Coding standards

Services-only mutations: all claim feedback writes go through `services/claims.rs::record_claim_feedback`; command handlers are thin wrappers like `people_entities.rs:592-618`, not semantic owners. No direct new writes to `entity_feedback_events`, `suppression_tombstones`, or `intelligence_feedback`; those are legacy compatibility paths (`src-tauri/src/db/intelligence_feedback.rs:103-148`).

Mode awareness: first line of the mutator is `ctx.check_mutation_allowed()?`; tests use `FixedClock` and live test context, following `services/context.rs:64-107`. No direct `Utc::now()` or `thread_rng()` in claims code. SQL timestamps should bind `ctx.clock.now()` rather than copy legacy `datetime('now')` patterns (`src-tauri/src/db/intelligence_feedback.rs:49`, `:83`).

Intelligence Loop 5-question check: feedback emits claim invalidation; Trust Compiler can discover un-applied rows; briefing/context surfaces consume render policy rather than legacy dismissals; activity/lint can read the typed event; repair jobs are queued but bounded. Fixtures use generic subjects and sources only.

Clippy budget: closed enums must be exhaustive; matrix tests must fail on an unhandled `FeedbackAction`. Serde names stay snake_case to match ticket/API strings; Rust variants use `ConfirmCurrent`, etc.

## 7. Integration with parallel wave-mates

W3-C / DOS-7 is the hard dependency. It creates `services/claims.rs`, `intelligence_claims`, `claim_feedback`, `claim_repair_job`, `agent_trust_ledger`, and invalidation ordering (`.docs/plans/wave-W3/DOS-7-plan.md:20-27`). W3-E must rebase onto that service and extend it, not create `services/claim_feedback.rs` or another writer path. If `verification_state` columns are not in W3-C's schema, coordinate migration numbering before opening W3-E.

W3-D / DOS-301 owns projection writers and repair of legacy AI surfaces. W3-E only marks render policy/invalidation; it does not write legacy AI columns or `intelligence.json`.

W3-B supplies `SubjectRef` / provenance source references. `WrongSubject` and `WrongSource` validation must consume those types, not parse ad hoc JSON. W3-G supplies `source_asof`; W3-H supplies claim type registry rules for `NeedsNuance` eligibility. W4 / DOS-5 consumes `claim_feedback`, `agent_trust_ledger`, and trust columns but W3-E owns the per-action semantic map that DOS-5 implements.

`src-tauri/src/services/context.rs` remains frozen W2-A surface; W3-E reads it for the context contract but does not edit it.

## 8. Failure modes + rollback

If `claim_feedback` insert succeeds but lifecycle/update/repair fails, the transaction rolls back entirely. Feedback cannot be persisted without its immediate side effects because that would leave Trust Compiler and render policy out of sync.

If activity signal emission fails after commit, do not roll back the feedback. Log non-sensitive error metadata and let the feedback row remain canonical, matching the existing post-commit signal posture in `src-tauri/src/services/intelligence.rs:1293-1305`.

If a repair job enqueue fails for `CannotVerify`, return an error and roll back unless the claim can be marked `Contested` with a durable `repair_enqueue_failed` reason that lint can surface. Preferred pick: rollback, because the ticket says repair behavior is part of the action semantics.

Rollback after a bad W3-E deploy is disabling the new command/API and reverting service changes; append-only `claim_feedback` rows can remain ignored by Trust Compiler if `applied_at IS NULL`. If schema columns were added, keep them nullable/defaulted and stop writing them rather than deleting user feedback.

W1-B universal write fence honored: W3-E writes only inside service transactions, calls the DOS-7 invalidation primitive, and never writes legacy file projections directly.

## 9. Test evidence to be produced

Core tests: `feedback_action_deserializes_only_nine_closed_values`, `record_claim_feedback_requires_live_mode`, `record_claim_feedback_rejects_missing_action_metadata`, `feedback_matrix_covers_every_action`, `feedback_matrix_wrong_subject_does_not_penalize_source`, `feedback_matrix_wrong_source_does_not_mark_truth_false`, `cannot_verify_enqueues_single_bounded_repair_job`, `surface_inappropriate_only_changes_surface_render_policy`, `not_relevant_here_is_relevance_not_trust`.

State/render tests: `system_state_machine_active_to_contested_to_needs_user_decision_terminal`, `needs_user_decision_excluded_from_default_claim_loader`, `needs_user_decision_review_queue_stub_returns_marker`, `terminal_state_not_auto_resolved_by_repair_success_without_explicit_resolution`, `mark_outdated_preserves_historical_claim_but_suppresses_current_render`.

Persistence/activity tests: `claim_feedback_rows_are_append_only_and_unapplied`, `trust_compiler_skips_applied_feedback_rows_stub`, `claim_feedback_emits_user_readable_activity_event`, `activity_event_payload_excludes_claim_text_and_notes`, `feedback_bumps_per_entity_claim_invalidation_without_full_regeneration`.

Wave merge-gate contribution: W3-E supplies the feedback-action matrix artifact, render-policy stub evidence, and Suite S checks for enum injection/cross-subject corrections/PII-free activity payloads. Suite P contribution is transaction/query count for feedback writes and repair enqueue caps. Suite E contribution is one fixture per primary action, including hundreds of low/medium claims where only high-impact ambiguous claims enter `needs_user_decision`.

Standard gate remains `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`; no frontend production UI test is required unless the implementation touches UI bindings.

## 10. Open questions

1. Storage coordination: should W3-C add `verification_state`, `verification_reason`, and `needs_user_decision_at` to the base `intelligence_claims` schema, or should W3-E own a follow-on migration? W3-E needs durable state for the render stub.
2. Activity log substrate: until DOS-275 lands, is a persisted `signal_events` activity event sufficient for "Activity log records user-readable feedback events", or should W3-E introduce a claim-specific activity table? My pick is no new table in W3-E.
3. Trust factor naming drift: ADR-0114 line 316 still says `user_feedback_weight` reads `user_feedback_signals`; ADR-0123 says `claim_feedback`. Confirm DOS-5 should consume `claim_feedback` directly.
4. Terminal resolution: after a claim reaches `needs_user_decision`, should explicit `ConfirmCurrent` mutate that same claim back to `Active`, or should it supersede with a user-authored corroboration while leaving the terminal audit row intact? My pick is the latter for auditability.
