# DOS-834 / W4 L0 Packet - Correction Propagation, Felt Stickiness, and Measurement

- **Version:** v1.4.9 - W4 judgment moat / correction loop
- **Primary issue:** [DOS-834](https://linear.app/a8c/issue/DOS-834)
- **Related issues:** DOS-8, DOS-277, DOS-338, DOS-318, DOS-443, DOS-447, DOS-316, DOS-811, DOS-446, DOS-278, DOS-317
- **Author date:** 2026-06-03
- **Tier:** Tier 3 markdown-only
- **Scope tier:** Wave-scope substrate. L0 requires `/codex challenge` or a project-approved equivalent, `ce-feasibility-reviewer`, `ce-security-lens-reviewer`, and mandatory K-in. Add `ce-product-lens-reviewer` if the wave compresses or changes the headline proof.
- **Status:** Draft for L0 review; not approved until adversarial, feasibility, security-lens, product/scope if invoked, and K-in verdicts are recorded.

---

## §0 Origination, Scope, and Trust Topology

**Origination class:** Extension, not a bug patch. The v1.4.9 storage/security work is debug-driven; W4 is the judgment half of the release. It turns already-shipped claim, trust, feedback, salience, signal, invalidation, and ability-runtime substrate into a visible loop: the user corrects DailyOS once, another surface changes without direct editing, and the change survives re-enrichment plus rebuild.

**Headline contract:** A correction made through any supported entry point propagates to source reliability, claim lifecycle/trust, derived context, and ranking; changes a surface the user did not directly touch; survives re-enrichment plus DOS-832 rebuild; and is measured by DOS-338's cross-surface stickiness metric.

**Irreducible core if W4 compresses:**

1. **DOS-8 intake:** typed semantic claim feedback enters through ADR-0123 actions and shipped service boundaries.
2. **DOS-834 propagation/invalidation:** feedback invalidates the right claims, trust bands, derived contexts, and ranking surfaces without a parallel signal stack.
3. **DOS-277 felt stickiness:** user-authored agenda/prep changes survive re-enrichment and prove the result on a surface the user did not edit directly.
4. **DOS-338 measurement:** the loop is scored as cross-surface, post-re-enrichment, indirect surfacing stickiness, not as internal persistence.

Everything else in W4 is enhancement: engagement weighting, claim-review queue budget, deviation/expected-presence work, review flow polish, and additional recommendation feedback affordances.

**Trust topology:** Local-to-local, single-user machine. W4 does not widen the v1.4.9 MCP carve-out: `Confidential` and `UserOnly` claims remain blocked from MCP. User corrections, nuance text, review queue notes, and eval fixtures can contain sensitive local content; committed docs, fixtures, logs, and proof must stay PII-free.

**Migration slots:** The wave plan reserved W4 `v284-v289`, but this branch's schema head is already `v276` on `public/dev`. Before L1 adds any migration, the wave lead must reconcile slot ownership in `.docs/plans/v1.4.9-waves.md` and avoid collisions with W1/W2/W3/W6 branches. Expected W4 schema needs, if any: `claim_engagement_events`/review-budget additions, `deviation_baselines`/`expected_presence`, and DOS-338 eval/stickiness run state. Do not add schema for data already represented by `claim_feedback`, `invalidation_jobs`, `surfacing_decisions`, or `claim_review_deferrals`.

### §0.1 Branch and Authority Notes

This packet was authored on `codex/v1.4.9-w4-dos834-l0` from `public/dev`. The local wave-plan file in this base still contains stale W1 text about ADR-0136, decrypt/sentinel migration, and v273/v274 slots. DOS-831 PR work corrects that W1 authority. W4's own scope is stable in both copies: DOS-834 is the keystone, W4's proof is cross-surface correction stickiness, and DOS-338 extends the existing eval bridge.

L1 must rebase onto the corrected wave-plan authority before implementation. A local packet note is not enough to claim approved migration slots or storage/reset dependencies.

---

## §1 Existing Substrate W4 Must Consume

### §1.1 Typed Feedback Is Already Real

ADR-0123 defines the 10-action `FeedbackAction` contract: `ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`, and `MergeIntent`. The code already exposes the enum and action semantics in `src-tauri/abilities-runtime/src/abilities/feedback.rs`.

The receipt-shaped entry point already exists in `src-tauri/src/services/claim_receipt/feedback.rs`. It validates the envelope target, applies the sensitivity gate, rejects agent actors, validates action-specific metadata, sanitizes free text, mints server-side idempotency, then delegates to `services::claims::record_claim_feedback`.

`services::claims::record_claim_feedback` already:

- inserts append-only `claim_feedback` rows;
- updates verification/lifecycle state through the service boundary;
- tombstones claim edges when lifecycle requires it;
- bumps claim invalidation/version state;
- queues targeted repair for repair-bearing actions;
- emits `claim_feedback_recorded` and `claim_verification_state_changed` signals.

W4 must tighten and extend this path. It must not invent a second feedback table or direct SQL writer.

### §1.2 Signal + Invalidation Substrate Exists

ADR-0080 defines the signal loop: source signals, confidence, learned reliability, user corrections, and re-enrichment. ADR-0115 defines durable invalidation jobs, policy registry discipline, coalescing/back-pressure, and the no-silent-drop invariant. ADR-0126 binds explicit user judgment to `claim_feedback`, trust factors, and claim lifecycle while keeping engagement separate from trust.

Current code has the pieces W4 should wire:

- `services::signals` / `signals::*` emit and propagate existing events.
- `services::claims::emit_claim_feedback_signals` records feedback-related signals after mutation.
- `services::invalidation_jobs::enqueue_signal_claim_recompute_in_tx` and `process_one_claim_recompute_job` recompute claim trust for a subject and account health for account subjects.
- `services::trust_recompute::recompute_claim_trust_for_subject` consumes `claim_feedback`, corroborations, contradictions, freshness, and source data to update trust.
- `claim_receipt::event_bridge` emits a best-effort Tauri invalidation event for receipt re-rendering, but the bridge is not the substrate propagation mechanism.

DOS-834's job is to prove and close gaps in this chain: feedback must enqueue the right recompute/invalidation work, recompute must change derived state, and downstream surfaces must render the changed state.

### §1.3 Ranking, Salience, and Review Queue Substrate Exists

Recommendations are registered as first-class claims by ADR-0125. Current code has `services/recommendations/{contracts,salience,surfacing,render,feedback,triggers}.rs`, `surfacing_decisions`, and `triggers_log`.

Useful current seams:

- `services::recommendations::salience` reads `claim_feedback` as a salience input.
- `services::recommendations::feedback` routes recommendation feedback through `record_claim_feedback` and emits the same feedback signal.
- `services::recommendations::render` renders `surfacing_decisions` rows.
- `services::claim_review_queue` has claim/proposal/candidate target kinds and deferrals.
- `services::recommendations::deviation` is intentionally still a stub and names `deviation_baselines` as the likely lane-owned schema.

W4 may add ranking/review/deviation primitives, but only where the existing seams are insufficient. The first pass is to wire feedback and invalidation into existing salience/surfacing decisions.

### §1.4 Agenda / Prep Stickiness Has a Starting Point

DOS-277 should not be greenfield. Current prep code already preserves user-authored agenda fields in:

- `services::meetings::update_meeting_user_agenda`;
- `services::meeting_prep_status::write::record_user_authored`;
- `services::meeting_prep_status::UserAuthoredFields`;
- `services::meeting_prep_status` state-machine tests.

The current behavior proves local persistence and recompute commutativity. W4 must raise the bar to felt stickiness: a user-authored agenda/prep correction changes a later, indirectly generated prep/readiness surface and remains changed after re-enrichment plus DOS-832 rebuild.

### §1.5 Eval Harness Exists

ADR-0110 defines the ability eval harness. `src-tauri/src/bridges/eval.rs` already invokes abilities in `ExecutionMode::Evaluate` with fixture services, provider, and tracer. `services/recommendations/eval.rs` is only a salience-eval slot.

DOS-338 extends the existing harness. It must not create a separate test runner for W4. The metric must be fixture-backed and runnable locally/CI without network or real user data.

Important constraint: `ExecutionMode::Evaluate` is read/snapshot oriented and blocks ordinary service mutations. DOS-338 therefore cannot prove "correction applied" by pretending the eval bridge can write. The harness must use the existing ADR-0110 fixture shape, but split execution into:

- a **hermetic mutation phase** against a temp fixture DB using service contexts that permit writes and fixture-only actors/providers; and
- an **eval/read phase** through `bridges/eval.rs` / registered abilities to collect rendered outputs, provenance, surface hashes, and no-leak assertions.

This is still one eval harness extension, not a greenfield runner: fixture loading, replay providers, deterministic clock/RNG, report schema, and CI entry stay under the ADR-0110/eval bridge lineage.

---

## §2 Chosen Architecture

### §2.1 Correction Event Model

W4 standardizes correction intake around a single event chain:

1. Surface action enters through `claim_receipt::feedback::submit_claim_feedback`, `commands::claim_feedback`, recommendation feedback, W3 file projection write-back, or W5 MCP feedback.
2. The entry point normalizes actor/surface/provenance and calls `services::claims::record_claim_feedback` or a named service wrapper that ultimately delegates there.
3. `record_claim_feedback` writes append-only feedback, applies typed lifecycle/trust-state effects, bumps invalidation/version state, and emits substrate signals.
4. DOS-834 maps those signals to claim recompute, source-reliability update, derived-context invalidation, salience re-rank, review-queue refresh, and receipt/surface re-render.
5. Surfaces render from recomputed substrate state; they do not apply private local patches as the source of truth.

No correction path may mutate `intelligence_claims`, `claim_feedback`, `claim_edges`, `surfacing_decisions`, or review-queue state from a command handler or UI-only store.

### §2.2 Propagation and Invalidation Contract

DOS-834 owns the missing middle between feedback and changed downstream surfaces:

- `claim_feedback_recorded` must enqueue or synchronously execute the correct `claim_recompute` work for the affected subject.
- Trust-band crossings must invalidate `build_intelligence_context`, `gather_account_context`, recommendation salience/surfacing decisions, claim receipts, and any visible prep/readiness outputs that consumed the affected claim.
- `WrongSubject` and `MarkFalse` must also invalidate claim edges and any surface whose subject graph or linked-entity context consumed those edges.
- `CannotVerify` must queue targeted repair without directly punishing trust; later lack of corroboration affects trust through the existing repair/trust path.
- `NeedsNuance` must create or supersede through the claim service, not treat corrected text as a display-only override.
- Signal/invalidation failures must be visible: job dead-letter, explicit stale marker, or returned error where correctness requires bounded-sync behavior. Silent log-only paths are acceptable only for UI fan-out events after the substrate mutation has already committed.

The current `services::claims::emit_claim_feedback_signals` shape is intentionally best-effort post-commit signal emission. W4 must not treat that as sufficient propagation. L1 must introduce a service-owned feedback propagation path that records the durable propagation outcome for correctness-critical work, either by extending the claim-feedback writer outcome or by adding a named service wrapper around it. That path must be table-driven from ADR-0123 actions and subjects, and must distinguish:

- durable substrate work that must be enqueued/executed or marked stale/dead-letter; and
- best-effort Tauri/event fan-out that may warn without failing the correction.

The L1 design must classify each target as bounded-sync or async:

- **Bounded-sync required:** direct user feedback that changes a currently visible claim receipt or review item; within the same interaction, the visible receipt/trust/lifecycle state must update or show an explicit pending/stale state.
- **Async acceptable:** broad salience re-rank, background prep regeneration, source reliability aggregate recompute, batch deviation-baseline refresh, and repair jobs, as long as the queue state is durable and surfaces show stale/pending when they have not caught up.

### §2.3 Source Reliability and Trust Effects

W4 consumes ADR-0080/0114/0123, not a new scoring model:

- Explicit feedback updates claim lifecycle/trust through `record_claim_feedback` and trust recompute.
- Source reliability deltas are action-specific. `WrongSource` can downweight a source for a claim type; `WrongSubject` should downweight the inference method/subject evidence without punishing the source's truthfulness; `CannotVerify` queues repair and waits for corroboration outcome.
- Passive engagement goes to ranking only. It must not inflate trust or mutate claim truth.
- Recommendation feedback reuses claim feedback and salience inputs. It may change future rank/surfacing, but not by writing a parallel recommendation truth model.

The default lane for source reliability is the existing `signal_weights` / `trust_recompute::source_reliability_for_claim` path. If L1 finds that path cannot express an ADR-0123 effect, the change belongs in the service/trust layer with tests or an ADR amendment. Do not add a parallel source-reliability table, and do not bury source-reliability effects in a UI composer.

### §2.4 Cross-Surface Consumption

The W4 proof must include at least two different render heads/surfaces and one indirect surfacing:

- Tauri claim receipt / entity detail / daily briefing style surface.
- W3 readable file projection or W5 MCP once those dependencies land.
- A derived surface the user did not edit directly, such as recommendation surfacing, meeting readiness/prep, account context, or an open-loop/risk shift output.

MCP parity is gated by sensitivity. `Public` and permitted `Internal` claims may cross MCP under the W5 contract; `Confidential` and `UserOnly` never do. A test that proves propagation by leaking forbidden claims to MCP is invalid.

### §2.5 DOS-277 Felt Stickiness

DOS-277 is the user-facing proof for the loop:

- User edits agenda/prep on one surface.
- Re-enrichment runs and regenerates the system-owned prep/context.
- The user-authored layer remains distinct from generated content and continues to influence the rendered prep/readiness output.
- A different surface shows the updated agenda/prep state without a direct user edit on that surface.
- DOS-832 rebuild replays the durable state so the agenda/prep correction is not lost after fresh-schema reconstruction.

W4 chooses the second model: **user-authored prep layer with claim-equivalent substrate semantics**, not a new claim type for agenda prose. The existing `meeting_prep_status` user-authored layer is the starting point; W4 hardens it so a user agenda/prep edit records subject, actor, surface, field, observed/source-asof timestamp, sensitivity, lifecycle/version, and invalidation targets. Generated prep remains system-owned. User-authored prep is a durable layer merged at render/context time through services, not a frontend overlay and not a direct mutation of generated prep text.

If L1 later discovers agenda/prep must become claim-backed to satisfy rebuild replay, that change requires a packet amendment or a new L0 review. It cannot be silently switched during implementation.

### §2.6 DOS-338 Stickiness Metric

DOS-338 adds a fixture-backed metric to the existing eval harness. The mutation phase applies a correction through the same service entry point W4 ships, then the eval/read phase invokes the relevant read/render abilities or service projections to compute surface hashes:

```text
stickiness_pass =
  correction_applied
  && indirect_surface_changed
  && direct_surface_rerendered
  && re_enrichment_preserved
  && rebuild_preserved
  && no_forbidden_surface_leak
```

Minimum metric dimensions:

- `entry_point`: app | file_projection | mcp
- `feedback_action`: ADR-0123 action
- `subject_kind`: account | project | person | meeting | action where applicable
- `direct_surface`
- `indirect_surface`
- `pre_reenrichment_state_hash`
- `post_reenrichment_state_hash`
- `post_rebuild_state_hash`
- `trust_band_before` / `trust_band_after`
- `recompute_job_id` / `repair_job_id` / `dead_letter_reason`
- `sensitivity_gate_result`

The metric is about user-visible and surface-visible state, not just database rows. A test that only asserts a `claim_feedback` row exists is insufficient.

Cross-wave status must be explicit in every DOS-338 report:

- `app_feedback`: required for W4 L1.
- `file_projection_feedback`: required once DOS-628 is merged into the W4 base; otherwise reported as `blocked_by_w3_merge` and not counted as full headline pass.
- `mcp_feedback`: owned by W5; W4 may define the expected parity fixture, but the version headline remains open until W5 proves it modulo sensitivity gates.
- `rebuild_preserved`: required for the version headline. If DOS-832 is not merged when W4 lands, W4 must emit the replay fixture and mark the metric `blocked_by_dos832`, not `pass`.

---

## §3 Acceptance Criteria

**AC1 - Single feedback writer.** Every W4 correction entry point routes through `services::claims::record_claim_feedback` or a named service wrapper that delegates to it. Static review shows no command/UI path writes `claim_feedback`, lifecycle columns, claim edges, or recommendation feedback state directly.

**AC2 - Propagation map.** DOS-834 ships a typed propagation map for feedback actions and affected subjects. For each ADR-0123 action, the map names trust/source effects, invalidation jobs, derived-context targets, salience/review targets, bounded-sync vs async behavior, and failure handling.

**AC3 - Feedback recompute.** `claim_feedback_recorded` and verification/lifecycle-changing feedback enqueue or execute claim recompute for the affected subject through a durable service-owned propagation path. Trust recompute consumes the feedback row exactly once or idempotently, updates trust bands when thresholds move, and records stale/dead-letter state instead of silently dropping failures. Existing warn-only signal fan-out is insufficient for this AC except for non-authoritative UI events after substrate work has been durably recorded.

**AC4 - Source reliability.** `WrongSource`, `WrongSubject`, `MarkFalse`, `ConfirmCurrent`, `MarkOutdated`, `CannotVerify`, and `NeedsNuance` have tested source/trust semantics matching ADR-0123 through the existing trust/source-reliability substrate unless an ADR-backed amendment says otherwise. Source reliability changes are attributed to source, inference method, or repair outcome correctly; passive engagement never changes trust.

**AC5 - Derived context invalidation.** Feedback that changes claim trust/lifecycle invalidates and refreshes `build_intelligence_context`, `gather_account_context`, account health where relevant, claim receipts, and recommendation salience/surfacing decisions. At least one test proves a surface the user did not directly edit changes after a correction.

**AC6 - Claim edges and subject graph.** `WrongSubject`, tombstone/withdrawal, contradiction, and supersession paths update or invalidate claim edges and subject-linked derived state. Re-enrichment cannot resurrect a tombstoned or wrong-subject claim into an active indirect surface.

**AC7 - Review queue budget.** Review queue and candidate deferrals consume existing `claim_review_queue` primitives. Any new budget state has schema, bounds, coalescing, and stale/dead-letter behavior. Queue state is not a local UI-only filter.

**AC8 - Deviation / expected presence.** DOS-316/DOS-811 deviation work consumes ADR-0126's anomaly rule: schema compression must not suppress deviation-flagged claims. `deviation_baselines` or equivalent schema is introduced only if needed, with source_asof/provenance, invalidation, and review semantics.

**AC9 - DOS-277 felt stickiness.** User-authored agenda/prep changes use the hardened prep-layer model from §2.5, survive re-enrichment, affect a different prep/readiness or context surface, and are preserved through DOS-832 rebuild. The proof distinguishes user-authored state from generated prep content and includes subject, provenance, sensitivity, lifecycle/version, and invalidation evidence for the user-authored layer.

**AC10 - DOS-338 eval harness.** Stickiness measurement extends ADR-0110 using the mutation/read split in §1.5/§2.6. Fixtures are hermetic, PII-free, source-attributed, sensitivity-aware, and cover direct re-render, indirect-surface change, re-enrichment, rebuild replay, and forbidden MCP sensitivity gates. The harness must not bypass `ExecutionMode::Evaluate` write blocking by silently writing through eval-mode services.

**AC11 - Cross-entry parity.** App/Tauri feedback, W3 readable file projection feedback, and W5 MCP feedback produce the same substrate effects modulo sensitivity gates. If W3 or W5 is not merged when W4 L1 runs, W4 may ship only the implemented entry-point proof plus parity fixture contracts; the proof report must say `partial`, name the missing upstream wave, and the version headline remains gated until file/MCP parity tests land.

**AC12 - Observability.** Proof reports include correction event id, feedback id, recompute/repair job ids, affected subject, invalidation target list, before/after trust band, direct/indirect surface hashes, re-enrichment run id, rebuild replay id, and PII-safe failure reason codes.

**AC13 - No PII fixtures.** Committed fixtures use generic entities (`account_01`, `person_01`, `project_01`, `user@example.com`) and synthetic content. No real customer/account/person/domain names, emails, or local absolute paths appear in source, tests, docs, PR text, or proof bundles.

**AC14 - Gates.** Focused W4 tests plus full gates pass:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
```

---

## §4 Intelligence Loop Integration Check

**1. Claim model.** Corrections are explicit `claim_feedback` events attached to immutable claims. Agenda/prep stickiness uses the user-authored prep layer chosen in §2.5 with equivalent subject, provenance, sensitivity, lifecycle/version, and invalidation semantics. No display-only correction state ships.

**2. Provenance + trust.** Feedback preserves actor, surface, subject, action, per-action metadata, source attribution, source_asof, sensitivity, and field path. Trust-band rendering changes only through trust/source/lifecycle recompute. `CannotVerify` requests corroboration; it does not directly punish trust.

**3. Signals + invalidation.** Feedback emits durable substrate signals and invalidation jobs for claim recompute, derived contexts, salience/surfacing, receipts, prep/readiness, and edge/subject graph changes. Failures are surfaced as job failure, dead-letter, stale marker, or returned error.

**4. Runtime + surfaces.** Consumers include `build_intelligence_context`, `gather_account_context`, prep/readiness outputs, recommendation surfacing, claim receipts, W3 files, Tauri, and MCP. Behavior is equal across surfaces except for sensitivity gates.

**5. Feedback loop.** User corrections feed claim lifecycle, source reliability, repair/corroboration queues, salience/ranking, review queue, eval fixtures, and future source/inference weighting. Dismissals and passive engagement are ranking inputs, not truth inputs.

---

## §5 Implementation Surface

Likely files/modules:

- `src-tauri/src/services/claim_receipt/feedback.rs`
- `src-tauri/src/services/claims.rs`
- `src-tauri/src/services/trust_recompute.rs`
- `src-tauri/src/services/invalidation_jobs.rs`
- `src-tauri/src/services/recommendations/{salience,surfacing,render,feedback,deviation,eval}.rs`
- `src-tauri/src/services/claim_review_queue/{queue,candidates}.rs`
- `src-tauri/src/services/meeting_prep_status/{mod,write,read}.rs`
- `src-tauri/src/services/meetings.rs`
- `src-tauri/src/intelligence/prompts.rs`
- `src-tauri/src/prepare/meeting_context.rs`
- `src-tauri/src/bridges/eval.rs`
- W3 claim-file projection service once DOS-628 lands
- W5 MCP feedback/read service once W5 lands
- migrations only after slot reconciliation

Avoid:

- No new untyped feedback enum.
- No parallel feedback/review DB tables where existing `claim_feedback`, `claim_review_deferrals`, or `surfacing_decisions` suffice.
- No direct DB writes from commands.
- No UI-only correction overlays for headline behavior.
- No greenfield eval runner.
- No MCP sensitivity carve-out expansion.
- No real workspace data in fixtures or proof.

---

## §6 Test and Proof Plan

Focused unit/service tests:

- Feedback action propagation matrix over all ADR-0123 actions.
- `claim_feedback_recorded` enqueues/executes recompute and handles queue cap/dead-letter.
- Trust recompute consumes feedback rows idempotently and changes trust band only when factors require it.
- `WrongSubject` / `MarkFalse` tombstone edges and keep active readers from returning the claim.
- `CannotVerify` queues repair and does not directly downweight trust.
- Passive engagement changes salience/ranking only, not trust/lifecycle.
- Recommendation feedback reuses `record_claim_feedback` and changes future surfacing.
- Review queue deferral/budget state is durable and bounded.
- Deviation-flagged claims resist schema-compression demotion.
- User-authored agenda/prep survives recompute/re-enrichment and changes an indirect surface.

Fixture / integration tests:

- App-side correction changes a different Tauri surface after recompute.
- File-projection correction (after DOS-628) produces the same substrate effects.
- MCP correction (after W5) produces the same substrate effects for permitted sensitivity and refuses `Confidential`/`UserOnly`.
- Re-enrichment replay does not resurrect tombstoned/wrong-subject claims.
- DOS-832 rebuild replay preserves representative W4 corrections and DOS-277 agenda/prep stickiness.
- DOS-338 hermetic eval fixture computes stickiness pass/fail and records before/after surface hashes.

Proof bundle:

- L0 reviewer verdicts and K-in findings.
- Migration slot reconciliation note if schema changed.
- Focused test output.
- Full gate output.
- PII-safe stickiness report with event/job/surface hashes.
- Screenshot or textual proof of indirect surface change if the surface is user-visible.

---

## §7 K-in Findings

Knowledge-store discovery ran against `docs/solutions/` and `.docs/decisions/` for claim feedback, correction, propagation, invalidation, trust bands, review queue, agenda re-enrichment, and eval harness.

Relevant hits:

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md` - new/changed claim producers need a runtime-wide trust audit over source_asof, provenance, trust inputs, runtime consumption, and recompute triggers.
- `docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md` - side-effect emission errors need deliberate emit-or-log wrappers; W4 must distinguish substrate invalidation failures from best-effort UI fan-out.
- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` - new substrate/claim work must answer the full Intelligence Loop check, not only signal invalidation.
- `.docs/decisions/0080-signal-intelligence-architecture.md` - user corrections update source reliability and drive re-enrichment.
- `.docs/decisions/0110-evaluation-harness-for-abilities.md` - DOS-338 must extend the fixture harness.
- `.docs/decisions/0115-signal-granularity-audit.md` - durable invalidation, coalescing, back-pressure, and no silent drops.
- `.docs/decisions/0123-typed-claim-feedback-semantics.md` - W4 feedback semantics are already specified.
- `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md` - claim types, sensitivity, and recommendation claim substrate.
- `.docs/decisions/0126-memory-substrate-invariants.md` - engagement affects ranking; feedback affects trust; no display-only correction state.
- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md` - MCP feedback is a product head but respects sensitivity gates.
- `.docs/decisions/0130-surface-independent-composition-contract.md` - surfaces consume substrate output and preserve claim refs/provenance.
- `.docs/decisions/0131-structured-embedding-claim-canonicalization.md` - explicit signal/invalidation targets for ambiguous-pair corrections and trust-band changes.

No prior solution was found that already ships DOS-834's complete cross-surface stickiness loop. The main reinvention risks are adding a parallel feedback writer, a parallel invalidation queue, or a greenfield eval runner.

---

## §8 L0 Reviewer Dispatch

Required:

- `/codex challenge` or project-approved equivalent: adversarial review for gaps in the headline proof.
- `ce-feasibility-reviewer`: verify the propagation map and eval/rebuild proof are buildable against current code.
- `ce-security-lens-reviewer`: review sensitivity gates, fixture/proof PII, MCP behavior, and correction artifact retention.
- `ce-learnings-researcher`: mandatory K-in over `docs/solutions/` and `.docs/decisions/`.

Conditional:

- `ce-product-lens-reviewer`: required if W4 compresses, changes headline proof, or treats DOS-277/DOS-338 as optional.
- `ce-data-migrations-reviewer`: required if W4 adds migration(s) after slot reconciliation.
- `ce-performance-reviewer`: required if claim recompute, salience re-rank, or eval harness changes hot paths or queue fan-out.

Approval standard:

- Unanimous L0 approval required before L1 implementation.
- Any finding that W4 ships a display-only correction, a second feedback writer, a second invalidation substrate, or a metric that only proves internal persistence is BLOCKING.
