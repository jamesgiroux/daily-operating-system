# DOS-834 / W4 L0 Packet - Correction Propagation, Felt Stickiness, and Measurement

- **Version:** v1.4.9 - W4 judgment moat / correction loop
- **Primary issue:** [DOS-834](https://linear.app/a8c/issue/DOS-834)
- **Related issues:** DOS-8, DOS-277, DOS-338, DOS-318, DOS-443, DOS-447, DOS-316, DOS-811, DOS-446, DOS-278, DOS-317
- **Author date:** 2026-06-03
- **Tier:** Tier 3 markdown-only
- **Scope tier:** Wave-scope substrate. L0 requires `/codex challenge` or a project-approved equivalent, `ce-feasibility-reviewer`, `ce-security-lens-reviewer`, `ce-data-migrations-reviewer`, `ce-performance-reviewer`, and mandatory K-in. Add `ce-product-lens-reviewer` if the wave compresses or changes the headline proof.
- **Status:** L0 approved after challenge cycle 7. L1 remains gated on W3/DOS-628 merge/rebase, DOS-832 rebuild proof availability, ADR-0123 redaction amendment, migration-slot reconciliation, and the W4 proof bundle gates below.

---

## §0 Origination, Scope, and Trust Topology

**Origination class:** Extension, not a bug patch. The v1.4.9 storage/security work is debug-driven; W4 is the judgment half of the release. It turns already-shipped claim, trust, feedback, salience, signal, invalidation, and ability-runtime substrate into a visible loop: the user corrects DailyOS once, another surface changes without direct editing, and the change survives re-enrichment plus rebuild.

**Headline contract:** A correction made through W4-supported entry points, Tauri/app feedback plus W3 readable-file projection feedback after DOS-628, propagates to source reliability, claim lifecycle/trust, derived context, and ranking; changes a surface the user did not directly touch; survives re-enrichment plus DOS-832 rebuild; and is measured by DOS-338's cross-surface stickiness metric. W5 MCP feedback remains fixture-contract-only in W4.

**Irreducible core if W4 compresses:**

1. **DOS-8 intake:** typed semantic claim feedback enters through ADR-0123 actions and shipped service boundaries.
2. **DOS-834 propagation/invalidation:** feedback invalidates the right claims, trust bands, derived contexts, and ranking surfaces without a parallel signal stack.
3. **DOS-277 felt stickiness:** user-authored agenda/prep changes survive re-enrichment and prove the result on a surface the user did not edit directly.
4. **DOS-338 measurement:** the loop is scored as cross-surface, post-re-enrichment, indirect surfacing stickiness, not as internal persistence.

Everything else in W4 is enhancement: engagement weighting, claim-review queue budget, deviation/expected-presence work, review flow polish, and additional recommendation feedback affordances.

**Trust topology:** Local-to-local, single-user machine. W4 does not widen the v1.4.9 MCP carve-out: `Confidential` and `UserOnly` claims remain blocked from MCP. User corrections, nuance text, review queue notes, and eval fixtures can contain sensitive local content; committed docs, fixtures, logs, and proof must stay PII-free.

**Migration slots:** The canonical wave plan owns migration slot reservations. It reserves W4 as `v284-v289`; this packet does not claim `v277-v283` even though the current local schema head is `v276`. W4 L1 must not add migrations until W3/DOS-628 and the corrected wave-plan authority are merged/rebased so the W4 block is available, or until `.docs/plans/v1.4.9-waves.md` is amended by the wave lead. Within the canonical W4 block, preserve this ownership order:

| Canonical W4 slot | Ownership | Required table / artifact |
| --- | --- | --- |
| `v284` | Correction feedback substrate | `claim_feedback_correction_envelopes`, `claim_feedback_propagation_jobs` / `claim_feedback_propagation_outcomes`, `correction_artifact_lifecycle_events`, and `correction_artifact_declassification_decisions` or equivalently named declassification-decision artifacts. |
| `v285` | Reliability learning | `source_reliability_feedback_deltas`, `source_claim_type_reliability`, `subject_inference_reliability`, and `source_reliability_backfill_runs` or named `migration_state` rows with run id, status, cursor, retry count, failure reason, key version, and terminal completed/failed/aborted markers. |
| `v286` | DOS-277 replay + prep jobs | `meeting_prep_correction_journal` plus mandatory durable `meeting_prep_regeneration_jobs` / outbox for W4 headline prep/readiness proof. |
| `v287` | DOS-338 measurement | `dos338_stickiness_runs` / `dos338_stickiness_observations`, storing only fixture-safe ids, hashes, gates, and reason codes. |
| `v288` | Review/engagement enhancement | `claim_engagement_events` and/or review-budget additions, only if DOS-317/DOS-443 ship in W4 L1. |
| `v289` | Deviation enhancement | `deviation_baselines` / `expected_presence`, only if DOS-316/DOS-811 ship in W4 L1. |

Do not add schema for data already represented by `claim_feedback`, `invalidation_jobs`, `surfacing_decisions`, or `claim_review_deferrals` unless the table above names the missing durability/provenance grain. If L1 compresses to the irreducible core, optional enhancement slots `v288-v289` stay unused or move out of the block; do not ship placeholder migrations.

### §0.1 Branch and Authority Notes

This packet was authored on `codex/v1.4.9-w4-dos834-l0` from `public/dev`. The local wave-plan file in this base still contains stale W1 text about ADR-0136, decrypt/sentinel migration, and v273/v274 slots. DOS-831 PR work corrects that W1 authority. W4's own scope is stable in both copies: DOS-834 is the keystone, W4's proof is cross-surface correction stickiness, and DOS-338 extends the existing eval bridge.

L1 must rebase onto the corrected wave-plan authority before implementation. A local packet note is not enough to claim approved migration slots or storage/reset dependencies.

### §0.2 L0 Challenge Cycle 1 Decisions

Cycle 1 failed this packet on three L0 blockers that are now explicit W4 contracts:

1. **DOS-277 prep corrections replay through a durable correction journal, not a display overlay.** W4 keeps the user-authored prep layer chosen in §2.5, but it must add or reuse a service-owned replay journal that DOS-832 can consume. The journal records meeting identity, field path, actor, surface, observed/source-asof timestamp, sensitivity, lifecycle/version, and the sanitized user-authored payload. Replay must call `services::meeting_prep_status::record_user_authored` or its hardened successor, never raw SQL. The journal is keyed by stable meeting/content identity plus field path, not volatile row ids. W4 cannot merge as the W4 headline until DOS-832 rebuild replay proves this path.
2. **Source reliability has claim-type grain.** W4 does not use the legacy `(source, entity_type, signal_type)` `signal_weights` row as the only persistence for ADR-0123 source effects. `WrongSource` updates source reliability for `(source_key_hash, claim_type, signal_type)` via the canonical helper in §2.3.1; `WrongSubject` updates subject-evidence/inference quality without punishing source truthfulness; `CannotVerify` queues repair and waits for corroboration outcome. L1 must add the service/trust-layer persistence needed for that grain, for example a `source_claim_type_reliability` table or an equivalent versioned replacement consumed by `trust_recompute::source_reliability_for_claim`. This is an extension of the trust layer, not a UI composer or parallel scoring model.
3. **Feedback provenance is persisted as a durable correction envelope.** W4 must not prove provenance by reconstructing it from optional joins after the fact. The feedback writer or named service wrapper persists, in the same transaction as `claim_feedback`, a durable envelope keyed by `feedback_id` with actor/surface, subject, target receipt, field path, source attribution, `source_asof`, sensitivity, idempotency/replay key, and sanitized action metadata. Existing `claim_feedback.payload_json` may hold action-specific data, but the provenance envelope must be queryable and testable as a first-class correction artifact.

### §0.3 L0 Challenge Cycle 2 Decisions

Cycle 2 failed the revised packet on four remaining L0 blockers that are now explicit W4 contracts:

1. **Schema ownership is mandatory, not optional.** The durable feedback envelope, action-specific propagation outcomes, claim-type source reliability, subject/inference reliability, DOS-277 replay journal, artifact lifecycle, and DOS-338 run state are W4-owned persistence, with the slot ledger in §0.
2. **Source reliability defaults are deterministic.** Claim-type source reliability uses explicit cold-start, fallback, and backfill semantics in §2.3; absent rows must not silently become zero, random, or source-punishing trust effects.
3. **Artifact lifecycle is service-owned.** Durable correction artifacts carry retention, deletion, redaction, source-removal, meeting-removal, workspace-reset, and proof-purge behavior in §2.7/AC9b. Append-only audit intent does not justify keeping raw sensitive payload forever.
4. **Propagation is table-driven by action.** W4 L1 must implement the §2.2 matrix for all 10 ADR-0123 actions, including sync class, failure handling, and derived-context/salience/prep targets. AC2 is no longer a vague requirement.

### §0.4 L0 Challenge Cycle 3 Decisions

Cycle 3 failed the revised packet on hidden implementation decisions that are now explicit W4 contracts:

1. **W3 is a hard W4 entry gate.** W4 L1 cannot begin from a base that lacks DOS-628 readable-file projection feedback. `blocked_by_w3_merge` is not an acceptable W4 proof status. W5 MCP parity is later, but W3 is on the critical path before W4.
2. **Full W4 completion requires rebuild proof.** W4 L1 may not merge as the W4 headline while DOS-832 rebuild replay is unavailable. A substrate-only partial would require explicit product/scope approval and a renamed scope; this packet does not approve that path.
3. **Source reliability uses one canonical source key helper.** Receipt feedback, the feedback writer, trust recompute, and backfill all derive the same source reliability key through the rule in §2.3.1; no path may invent synthetic `source_ref` semantics locally.
4. **Redaction is a privacy mutation exception, not an ambiguous overlay.** W4 preserves append-only feedback event identity, but payload-bearing/source-identifier columns may be irreversibly scrubbed by a service-owned redaction mutation codified by an ADR-0123 amendment before any L1 code mutates payload-bearing feedback/correction artifacts. Overlay-only redaction is insufficient if raw sensitive payload remains queryable.
5. **UserOnly taint is transitive.** User-authored prep can influence local state, but any derived output that semantically depends on `UserOnly` or `Confidential` prep remains blocked from MCP unless an explicit non-semantic declassification contract applies.
6. **Propagation fan-out is bounded and durable.** W4 must add chunked recompute, source reliability delta ledgers, durable propagation job ownership, logical target cardinality, coalescing keys, backpressure caps, and a bounded DOS-338 CI matrix before implementation.

### §0.5 L0 Cycle-Cap Waiver

`engineering-ladder.html` says two non-converging plan cycles escalate to L6. This W4 packet exceeded that threshold during wave-level hardening, and the project owner explicitly directed continued reviewer cycles for all blocked wave packets until the required L0 panel converges. This section records that L6/project-owner waiver for W4 L0 only. It does not waive unanimity, W3/DOS-628 entry gating, DOS-832 rebuild proof, migration-slot reconciliation, or any L1/L2/L3 gate.

### §0.6 L0 Challenge Cycle 4-6 Decisions

Cycles 4-6 found packet gaps that are now explicit W4 contracts:

1. **Lifecycle artifacts include subject/entity lifecycle.** Person, account, project, meeting, and action deletion/merge/rebind events must scrub or rebind asserted/corrected subject refs across correction envelopes, `subject_inference_reliability`, propagation outcomes, lifecycle events, declassification decisions, DOS-338 observations, and proof artifacts.
2. **Source reliability keys are versioned and rebuild-safe.** Source reliability uses `source_key_version` and `source_key_epoch_hash`; normal DOS-832 rebuild preserves the live workspace key/epoch, source removal excludes affected keyed aggregates, key rotation requires terminal rekey/stale-version handling, and reset purges or invalidates old rows.
3. **Backfills and coalesced propagation terminalize.** Source reliability backfills record run id, cursor, retry count, failure reason, key version, status, and terminal completed/failed/aborted/stale-key-version markers. Propagation caps write coalesced scope/cursor/sync/stale/dead-letter/retry state instead of silently dropping work.
4. **Declassification decisions have slot ownership.** W4 assigns declassification-decision artifacts to the v284 correction artifact substrate and treats stale decisions as fail-closed lifecycle artifacts.
5. **Migration proof is explicit.** W4 L1 must run the transactional migration guard, prove fresh install and upgrade, prove service-backfill idempotency/resume/terminal failure behavior, and follow the repository migration filename/version offset convention after rebase.
6. **W4 proves app plus W3, not MCP.** W3/DOS-628 readable-file projection feedback is required before W4 L1 and merge; W5 MCP feedback stays fixture-contract-only until W5.

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

1. Surface action enters through `claim_receipt::feedback::submit_claim_feedback`, `commands::claim_feedback`, recommendation feedback, and W3 file projection write-back after DOS-628. W5 MCP feedback is modeled only as a fixture contract in W4.
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

The required propagation map is table-driven from the closed ADR-0123 enum. Adding a new `FeedbackAction` without a map row is a build/test failure.

| Action | Trust/source effect | Durable propagation targets | Sync class and failure behavior |
| --- | --- | --- | --- |
| `ConfirmCurrent` | Claim alpha + user corroboration; claim-type source reliability positive delta for cited source(s); lifecycle/render returns to current where allowed. | Claim recompute for subject, receipt rerender, `build_intelligence_context`, `gather_account_context`, salience/surfacing refresh, review item close/deferral clear where applicable. | Receipt/review update bounded-sync or explicit pending; aggregate source and salience async with durable outcome row. |
| `MarkOutdated` | Freshness repair; current rendering suppressed or superseded; no source truthfulness penalty. | Claim recompute, freshness repair job, receipt/context invalidation, salience/surfacing demotion, prep/readiness invalidation if consumed. | Receipt suppression bounded-sync; repair and broad context refresh async with stale marker on failure. |
| `MarkFalse` | Claim beta + contradiction/reconcile; small source penalty only for source-support path; lifecycle suppresses except audit. | Claim recompute, edge/tombstone invalidation, contradiction/reconcile repair, receipt/context invalidation, salience/surfacing suppression, review queue refresh. | Suppression bounded-sync; repair/edge fan-out async but durable/dead-lettered. |
| `WrongSubject` | Claim beta for asserted subject; subject/inference reliability negative delta; no source truthfulness penalty. | Subject graph/edge invalidation, claim recompute for old and corrected subject when provided, derived-context invalidation, salience/surfacing refresh, prep/readiness invalidation if the claim was meeting-context input. | Old-subject suppression bounded-sync; corrected-subject recompute async unless currently visible. |
| `WrongSource` | Claim contested; source reliability negative delta at `(source_key_hash, claim_type, user_feedback)`; source-support repair. | Source reliability update, claim recompute, source-support repair job, receipt/context invalidation, salience/surfacing refresh. | Receipt caveat bounded-sync; source aggregate and repair async with durable outcome. |
| `CannotVerify` | No direct trust/source delta; claim becomes needs-corroboration/contested until repair outcome. | Bounded corroboration repair job, receipt/context invalidation for caution rendering, review queue candidate if repair budget exceeds limits. | Receipt caution bounded-sync; repair async. Failure must show repair queued/dead-lettered, not silent trust movement. |
| `NeedsNuance` | Superseder/correction through claim service; text overlap may affect claim alpha but source reliability unchanged unless later repair outcome says otherwise. | Superseder/contradiction path, claim recompute, receipt/context invalidation, salience/surfacing refresh, review queue if nuance requires human review. | Direct render/pending superseder bounded-sync; broad recompute async. |
| `SurfaceInappropriate` | No truth/source delta; surface-policy update only. | Surface policy/render suppression for named surface, receipt invalidation on that surface, salience/surfacing refresh for that invocation class, review queue if policy conflict is ambiguous. | Named-surface hide bounded-sync; policy aggregate async. |
| `NotRelevantHere` | No truth/source delta; relevance/ranking signal only. | Invocation/context relevance update, salience/surfacing demotion for matching context, DOS-338/eval observation, receipt/context rerender when currently visible. | Current invocation bounded-sync or pending; future ranking async. |
| `MergeIntent` | Typed merge proposal; no trust/source/lifecycle mutation on source claim until merge service executes. | Merge proposal persistence, subject graph candidate invalidation, review queue candidate, receipt/context annotation if proposal is visible. | Proposal receipt bounded-sync; graph/review refresh async. |

Each row writes one durable propagation outcome per target, including `feedback_id`, action, subject, target kind, sync class, enqueue/run id where applicable, stale/dead-letter reason, and retry state. The writer may coalesce jobs, but it may not drop a correctness-critical target without recording the coalesced target that owns it.

### §2.3 Source Reliability and Trust Effects

W4 consumes ADR-0080/0114/0123, not a new scoring model:

- Explicit feedback updates claim lifecycle/trust through `record_claim_feedback` and trust recompute.
- Source reliability deltas are action-specific. `WrongSource` can downweight a source for a claim type; `WrongSubject` should downweight the inference method/subject evidence without punishing the source's truthfulness; `CannotVerify` queues repair and waits for corroboration outcome.
- Passive engagement goes to ranking only. It must not inflate trust or mutate claim truth.
- Recommendation feedback reuses claim feedback and salience inputs. It may change future rank/surfacing, but not by writing a parallel recommendation truth model.

The default lane for source reliability is the trust layer consumed by `trust_recompute::source_reliability_for_claim`, but W4 must extend its persistence grain for ADR-0123 effects. The current `signal_weights` key `(source, entity_type, signal_type)` is insufficient by itself. L1 must introduce claim-type-aware source reliability persistence keyed by source and claim type, or a versioned replacement that exposes the same effective grain through the service/trust layer. Do not bury source-reliability effects in a UI composer, and do not let passive engagement mutate this trust path.

Default/fallback semantics are fixed:

- **Precedence:** claim-type reliability row for `(source_key_hash, claim_type, signal_type)` wins; if absent, fall back to legacy `signal_weights` for the broader `(source, entity_type, signal_type)`; if both are absent, use a neutral prior (`effective_multiplier = 1.0`, no alpha/beta delta).
- **Cold start:** creating the table does not backfill guessed rows. Existing claims keep their current trust until explicit feedback, repair outcome, or source learning creates a row.
- **Backfill:** W4 may add a service-owned, idempotent backfill only for rows that can be derived from existing `claim_feedback` with source attribution and claim type. It must write an auditable migration/backfill run state, not pretend the SQL migration completed service interpretation. The run state records run id, status (`queued`, `running`, `completed`, `failed`, `aborted`, `stale_key_version`), source-key version, high-water feedback id or source cursor, retry count, failure reason code, started/updated/terminalized timestamps, and a terminal marker before deployment can treat the backfill as complete.
- **Action writes:** `ConfirmCurrent` and qualifying `MarkFalse` update cited source reliability at claim-type grain; `WrongSource` updates the named/cited source at claim-type grain; `WrongSubject` updates subject/inference reliability only; `MarkOutdated`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`, and `MergeIntent` do not directly punish source truthfulness unless a later repair/corroboration outcome produces source evidence.
- **Tests:** absent rows must prove neutral behavior; legacy-only rows must prove fallback behavior; claim-type rows must prove precedence; repeated writes must be idempotent by `feedback_id`.

### §2.3.1 Canonical Source Reliability Key

W4 L1 adds one service helper, for example `services::source_reliability::derive_source_reliability_key`, used by receipt feedback, the feedback writer, trust recompute, and backfill. No caller may synthesize its own source key.

The helper returns a non-PII key envelope:

```text
SourceReliabilityKey {
  data_source,
  source_key_kind,
  source_key_version,
  source_key_epoch_hash,
  source_key_hash,
  claim_type,
  signal_type
}
```

Derivation order:

1. If the claim has a canonical `source_ref` / source registry pointer, normalize that value with `data_source` and store only a keyed hash plus non-sensitive key kind.
2. Else if the receipt feedback names a validated `source_content_hash`, normalize `data_source + source_content_hash` through the same helper. The receipt bridge may pass the validated hash into the helper, but it must not invent a separate synthetic `source_ref`.
3. Else if the claim has source attribution with a source item/content hash, normalize `data_source + source item/content hash`.
4. Else fall back to `data_source + claim_type + subject_type` as a broad legacy key and mark the row `source_key_kind = legacy_broad_fallback`.

Rows store the keyed hash, source-key version, epoch hash, data source enum, key kind, claim type, and signal type. Raw source identifiers, document paths, email ids, meeting ids, opaque external ids, source labels, and receipt snippets stay in the correction envelope only if allowed by sensitivity, and redaction/removal in §2.7 must scrub them from all payload-bearing artifacts. Trust recompute derives the same key from each claim at read time; it never joins on raw labels or a locally invented synthetic id.

The source-key contract is versioned:

- **Key source:** `source_key_hash` is a keyed hash of normalized canonical source material from the derivation order above. The key material comes from the local workspace key/secret used for PII-safe hashes, plus a W4 `source_key_version` constant and `source_key_epoch_hash` marker. Raw key material is never stored.
- **Rebuild stability:** DOS-832 rebuild preserves the workspace key/epoch for a live workspace, so the same source attribution derives the same `(source_key_version, source_key_epoch_hash, source_key_hash)` across rebuild. This is required for source reliability to survive W4's headline rebuild proof.
- **Rotation:** if the hash algorithm/key version changes, L1 must either run a service-owned rekey backfill with terminal run state before old aggregates are consumed, or mark old-version rows `stale_key_version` and exclude them from trust recompute until rebuilt. Mixed-version rows may not silently combine.
- **Source removal:** source removal writes a lifecycle/revocation event for the affected keyed hash/version and excludes affected deltas/aggregates from future trust recompute before scrubbing payload-bearing source identifiers. Preserved audit rows keep only keyed hashes or PII-safe tombstones.
- **Workspace reset:** workspace reset removes or invalidates the workspace key/epoch and purges reliability deltas/aggregates with the DB or service purge. Post-reset recompute must not consume pre-reset source reliability rows.
- **Tests:** L1 proves rebuild stability, source-removal exclusion, stale-version exclusion, rekey/backfill idempotency, and reset purge/invalidation for source reliability rows.

Source reliability updates are two-layered:

- `source_reliability_feedback_deltas` records one immutable/idempotent delta per `(feedback_id, source_key_version, source_key_epoch_hash, source_key_hash, claim_type, signal_type, effect_kind)` with unique constraint and small numeric delta.
- `source_claim_type_reliability` stores the aggregate by `(source_key_version, source_key_epoch_hash, source_key_hash, claim_type, signal_type)`, updated transactionally from unapplied deltas or rebuilt by chunked service backfill. Retries must observe the delta row and not double-count.

Backfill is chunked by source key / claim type cursor and records durable run state through the table or migration-state contract in §0. SQL-only guessed backfill is forbidden; service code interprets existing `claim_feedback`, source attribution, source-key version, source-removal/revocation state, and redaction state.

### §2.4 Cross-Surface Consumption

The W4 proof must include at least two different render heads/surfaces and one indirect surfacing:

- Tauri claim receipt / entity detail / daily briefing style surface.
- W3 readable file projection feedback, after DOS-628 lands. This is required for W4. W5 MCP remains a fixture-contract placeholder until W5 implements the MCP entry point.
- A derived surface the user did not edit directly, such as recommendation surfacing, meeting readiness/prep, account context, or an open-loop/risk shift output.

MCP parity is gated by sensitivity and is not a W4 merge proof. `Public` and permitted `Internal` claims may cross MCP under the later W5 contract; `Confidential` and `UserOnly` never do. A test that proves propagation by leaking forbidden claims to MCP is invalid.

### §2.5 DOS-277 Felt Stickiness

DOS-277 is the user-facing proof for the loop:

- User edits agenda/prep on one surface.
- Re-enrichment runs and regenerates the system-owned prep/context.
- The user-authored layer remains distinct from generated content and continues to influence the rendered prep/readiness output.
- A different surface shows the updated agenda/prep state without a direct user edit on that surface.
- DOS-832 rebuild replays the durable state so the agenda/prep correction is not lost after fresh-schema reconstruction.

W4 chooses the second model: **user-authored prep layer with claim-equivalent substrate semantics**, not a new claim type for agenda prose. The existing `meeting_prep_status` user-authored layer is the starting point; W4 hardens it so a user agenda/prep edit records subject, actor, surface, field, observed/source-asof timestamp, sensitivity, lifecycle/version, invalidation targets, and a replay-journal key stable across DOS-832 rebuild. Generated prep remains system-owned. User-authored prep is a durable layer merged at render/context time through services, not a frontend overlay and not a direct mutation of generated prep text.

The rebuild contract is explicit: W4 emits a prep-correction journal entry whenever `record_user_authored` or its hardened successor writes the user-authored prep layer. DOS-832 replays that journal through the same service writer after re-enrichment rebuilds meetings/prep context. If a meeting identity cannot be matched unambiguously, replay orphans the prep correction with a PII-safe reason code and does not attach it to the wrong meeting.

All agenda/notes/prep edit entry points must delegate to the hardened journal writer. The current dual lane (`services::meetings::update_meeting_user_agenda` and `services::meeting_prep_status::write::record_user_authored`) is implementation debt W4 must close or wrap; no app command may update the user-authored layer without producing the replay journal, lifecycle event, and invalidation outcome.

Prep journal privacy is stricter than ordinary claim receipt privacy because the payload is user-authored prose but not a claim row:

- Default sensitivity floor is `UserOnly` for raw user-authored prep payloads. L1 may lower only explicitly declassified, non-semantic aggregate fields through the service-owned decision below.
- Raw prep journal payloads never cross MCP. UserOnly/Confidential taint propagates transitively: any summary, paraphrase, recommendation, context block, surface hash explanation, or prep/readiness output that semantically depends on a UserOnly/Confidential prep correction is also blocked from MCP/read surfaces unless it passes the declassification rule below.
- Declassification is only allowed for non-semantic aggregates that cannot reveal the payload by quote, paraphrase, summary, entity-specific implication, or source pointer. Examples: `has_user_prep_correction = true`, fixture-safe hashes, stale/pending flags, and PII-safe reason codes. Declassification must write a service-owned decision with actor, surface, source sensitivity, derived field, and reason.
- Tauri/local renderers may show the user-authored layer, but prompts and derived contexts must wrap the payload through the existing user-data/sensitivity path and record the sensitivity decision.
- `build_intelligence_context`, `gather_account_context`, and prep/readiness outputs must have fixtures proving a `UserOnly` prep correction influences allowed local state without leaking raw text, paraphrase, summary, source pointer, or entity-specific implication into MCP or committed proof artifacts.

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
- `file_projection_feedback`: required for W4 L1 because W3/DOS-628 is a hard entry gate.
- `mcp_feedback`: owned by W5; W4 defines the expected parity fixture and reports `blocked_by_w5` until W5 proves it modulo sensitivity gates.
- `rebuild_preserved`: required for W4 L1 merge as the W4 headline. If DOS-832 is unavailable, this packet does not approve a merge; any substrate-only partial requires explicit product/scope approval and a renamed scope.

### §2.7 Correction Artifact Lifecycle and Privacy

Durable correction artifacts are user-owned local substrate, not disposable UI logs. W4 L1 must implement lifecycle behavior through services for:

- `claim_feedback` rows and their correction envelopes;
- source/claim-type reliability rows;
- subject/inference reliability rows;
- subject/person merge, rebind, and deletion references inside correction artifacts;
- prep correction journal entries;
- propagation outcome rows;
- DOS-338 run/observation rows and proof artifacts.

Lifecycle rules:

- **Retention:** correction artifacts remain while their parent claim, source, meeting/prep identity, or eval run remains live. DOS-338 run state is bounded to fixture-safe ids/hashes and may keep only recent/local runs; committed proof contains no raw local content.
- **Redaction:** W4 uses a privacy mutation exception, codified by an ADR-0123 amendment before any L1 code mutates payload-bearing feedback/correction artifacts. `claim_feedback` event identity remains append-only: `feedback_id`, `claim_id`, action, actor class, timestamps, lifecycle/trust effect, and PII-safe reason codes are preserved. Payload-bearing/source-identifier columns may be irreversibly replaced with a redaction marker and keyed hashes by a service-owned redaction call. Overlay-only redaction is not sufficient when the raw payload remains queryable.
- **Parent removal:** source removal redacts or tombstones every source identifier/source ref across correction envelopes, source reliability deltas, source reliability aggregates, propagation jobs/outcomes, DOS-338 observations, and proof artifacts. Raw document paths, email ids, meeting ids, opaque external ids, source labels, receipt snippets, target receipt source refs, and field-attribution refs are removed or replaced by PII-safe tombstone markers / keyed hashes. Claim removal redacts payload-bearing correction envelopes while preserving feedback id/action audit; meeting removal redacts prep payloads and source-like meeting identifiers, then orphans replay entries instead of deleting audit history silently.
- **Subject/entity lifecycle:** person, account, project, meeting, or action subject deletion, merge, and rebind events scrub or rebind asserted/corrected subject references across correction envelopes, `subject_inference_reliability`, propagation jobs/outcomes, lifecycle events, declassification decisions, DOS-338 observations, and proof artifacts. Ambiguous rebinds orphan replay/propagation instead of guessing. Audit identity that must survive is stored as keyed hashes or PII-safe tombstone markers only.
- **Workspace reset:** local reset/deletion removes correction artifacts with the DB or runs the same service purge before export. No committed fixture/proof may depend on a real local path or customer identifier surviving reset.
- **Replay safety:** redacted or orphaned artifacts do not replay into DOS-832 rebuild. Replay reports PII-safe `redacted`, `orphaned`, or `blocked_by_*` reason codes.
- **Proof purge:** DOS-338 proof artifacts expose a service purge and store only fixture-safe hashes/ids by default; raw surface text is not persisted in production proof tables.
- **Lifecycle authorization:** redaction, purge, reset, and declassification requests are first-party `Actor::User` local/Tauri-only in W4. MCP lifecycle requests are forbidden unless W5 adds a narrower approved scope and tests it.

The artifact lifecycle service must run in the same service-boundary discipline as feedback writes. A command handler may request deletion/redaction/reset, but it must not mutate these tables directly.

Lifecycle and declassification artifacts are also lifecycle-managed:

- `correction_artifact_lifecycle_events` payloads are PII-safe only: artifact kind/id hash, lifecycle action, actor class, local surface, timestamp, reason code, and keyed parent hash. They never store raw source ids, source labels, user prose, receipt snippets, or local paths.
- Declassification decisions are first-class artifacts keyed by `(source_artifact_kind, source_artifact_version/hash, derived_field, destination_surface, decision_version)`. They store only PII-safe source hashes and reason codes.
- Parent redaction, source removal, meeting removal, subject/entity deletion or merge/rebind, workspace reset, or artifact version change revokes affected declassification decisions before any MCP/read render can reuse them.
- Lifecycle event rows and declassification decisions participate in source/subject/meeting/reset scrub sweeps; stale decisions fail closed and produce a `declassification_revoked`, `redacted_parent`, or `orphaned_subject` reason code.
- Tests must prove a stale declassification decision cannot authorize a derived field after the parent UserOnly/Confidential artifact is redacted, removed, or versioned.

Append-only redaction consequences are explicit:

- Trust recompute and replay consume the preserved semantic action plus non-sensitive hashes, not raw payload text.
- Redaction of a payload that is required for future replay turns that replay target into `redacted` and prevents replay rather than guessing.
- Redaction emits invalidation/propagation outcomes so surfaces stop rendering stale raw/source details.
- Existing `claim_feedback.payload_json` rows that contain sensitive action metadata are scrubbed only through the service redaction mutation; tests must prove readers use the redacted view/state afterward.

### §2.8 Bounded Fan-Out, Workers, and Backpressure

W4 feedback propagation must be durable and bounded. Existing synchronous claim recompute can dead-letter subjects above the current active-claim cap; W4 must not treat subject size alone as a correctness failure.

Bounded recompute contract:

- Start with targeted claim recompute for the corrected claim and any directly named superseder/corrected subject.
- Subject-wide recompute runs in durable chunks no larger than the existing synchronous cap. Each chunk records cursor, claim id range or stable page token, affected subject, status, retry count, and dead-letter reason.
- A subject with more than 25 active claims must process as chunks; it must not dead-letter solely because of size.
- Outcome rows record one logical recompute target plus chunk children, not one row per surface render.

Durable propagation target ownership:

| Target kind | Operation | Worker owner | Coalescing key | Bound / failure behavior |
| --- | --- | --- | --- | --- |
| `claim_recompute` | targeted claim or chunked subject recompute | existing invalidation worker plus W4 chunk adapter | `subject_ref + claim_type + chunk_cursor` | chunk size <= synchronous cap; dead-letter only malformed or repeated failing chunk. |
| `source_reliability_delta` | apply idempotent reliability delta and update aggregate | W4 source reliability service/worker | `source_key_hash + claim_type + signal_type` | unique delta by `feedback_id`; batch aggregate updates by key; no full-history scan on retry. |
| `targeted_repair` | corroboration/source/subject/freshness/policy repair | existing targeted repair worker | `claim_id + repair_kind` | existing budgets apply; over-budget rows stay queued or low-priority, not dropped. |
| `salience_surfacing` | rerank affected recommendation candidates | recommendation trigger/salience service via W4 propagation job | `subject_ref + surface/context key` | SQL/service query must enforce `LIMIT 25` or cursor pages of <=25 over affected claim refs, consumed claim refs, and candidate ids before sort/rank; batch-prefetch feedback. |
| `prep_regeneration` | regenerate prep/readiness derived state after prep/claim correction | mandatory durable W4 prep-regeneration outbox for headline proof | `meeting_stable_key + field_path` | in-memory `MeetingPrepQueue` is insufficient for W4 headline proof; queued/running/completed/dead-letter state must survive restart. |
| `deviation_refresh` | refresh deviation/expected-presence state | optional W4 deviation worker if DOS-316/DOS-811 ship | `subject_ref + deviation_kind` | optional; must define cap and stale/dead-letter behavior before schema lands. |
| `receipt_surface_update` | bounded direct receipt/review pending/current update | feedback writer / receipt service | `feedback_id + direct_surface` | bounded-sync: update visible state or return explicit pending/stale marker. |

`claim_feedback_propagation_jobs` / outcomes use logical target identity: `(feedback_id, target_kind, operation, coalescing_key, sync_class)`. Concrete render rows, individual UI events, and every possible future surface do not each get a row. A single logical target may own multiple concrete effects if it records the coalesced scope, cursor/page token, retry count, stale/dead-letter state, and PII-safe failure reason.

Backpressure rules:

- Each feedback event has a default maximum of 32 logical propagation rows, well below the existing global pending invalidation cap. If the action would exceed it, W4 writes one coalesced batch target with bounded scope predicate, cursor/page token, sync class, retry state, stale/dead-letter status, and failure reason, not unbounded rows and not silent omission.
- Workers are restart-safe: queued/running/completed/dead-letter status is in SQLite, not process memory.
- Stale markers are visible to direct surfaces when bounded-sync work cannot finish inside the interaction budget.
- Salience/surfacing rerank must use a bounded candidate window or batch feedback prefetch to avoid per-claim feedback scans.
- Source reliability backfill uses auditable run state by source key version, source key/claim type, and cursor. It can resume after interruption and can terminalize as completed, failed, aborted, or stale-key-version instead of staying ambiguous.

DOS-338 CI budget:

- Unit tests cover every ADR-0123 propagation row.
- The required CI stickiness smoke matrix is bounded to representative actions: `ConfirmCurrent`, `WrongSource`, `WrongSubject`, `CannotVerify`, `NeedsNuance`, and `SurfaceInappropriate`, plus the DOS-277 prep correction case. Exhaustive all-action/all-entry-point matrices may run locally/manual but are not required for every CI pass.
- CI reports wall-clock time, fixture count, and skipped cross-wave statuses. It must fail if the smoke matrix expands without an explicit budget update.

---

## §3 Acceptance Criteria

**AC1 - Single feedback writer.** Every W4 correction entry point routes through `services::claims::record_claim_feedback` or a named service wrapper that delegates to it. Static review sweeps all direct `record_claim_feedback` callers and all command/UI feedback paths; none may write `claim_feedback`, correction envelopes, lifecycle columns, claim edges, recommendation feedback state, source reliability, or prep journals directly. Existing transactional callers, including recommendation feedback, must keep feedback, envelope, propagation, and domain metadata atomic through existing nested transaction support. Test-only fixture inserts are allowed only in synthetic fixtures with explicit comments/helpers; production code is not exempt.

**AC2 - Propagation map.** DOS-834 ships the §2.2 typed propagation map for all 10 ADR-0123 actions and affected subjects. For each action, tests assert trust/source effects, invalidation jobs, derived-context targets, salience/review targets, bounded-sync vs async behavior, and failure handling. A missing action row is a build/test failure. The map must use the §2.8 logical target identity and may not fan out unbounded concrete surface rows.

**AC3 - Feedback recompute.** `claim_feedback_recorded` and verification/lifecycle-changing feedback enqueue or execute targeted/chunked claim recompute for the affected subject through a durable service-owned propagation path. Trust recompute consumes the feedback row exactly once or idempotently, updates trust bands when thresholds move, and records stale/dead-letter state instead of silently dropping failures. Subjects with more than 25 active claims process in chunks and do not dead-letter solely because of size. Existing warn-only signal fan-out is insufficient for this AC except for non-authoritative UI events after substrate work has been durably recorded.

**AC4 - Source reliability.** `WrongSource`, `WrongSubject`, `MarkFalse`, `ConfirmCurrent`, `MarkOutdated`, `CannotVerify`, and `NeedsNuance` have tested source/trust semantics matching ADR-0123 through service-owned trust/source-reliability persistence. Source reliability uses the canonical §2.3.1 helper and supports `(source_key_version, source_key_epoch_hash, source_key_hash, claim_type, signal_type)` grain for source-support actions, with cold-start, fallback, version/epoch stability, source-removal exclusion, reset invalidation, chunked backfill, terminal backfill run state, precedence, delta-ledger idempotency, and retry tests. `WrongSubject` updates subject-evidence/inference quality without downweighting source truthfulness. `CannotVerify` queues repair and waits for corroboration outcome. Passive engagement never changes trust.

**AC5 - Derived context invalidation.** Feedback that changes claim trust/lifecycle invalidates and refreshes `build_intelligence_context`, `gather_account_context`, account health where relevant, claim receipts, and recommendation salience/surfacing decisions. At least one test proves a surface the user did not directly edit changes after a correction.

**AC6 - Claim edges and subject graph.** `WrongSubject`, tombstone/withdrawal, contradiction, and supersession paths update or invalidate claim edges and subject-linked derived state. Re-enrichment cannot resurrect a tombstoned or wrong-subject claim into an active indirect surface.

**AC7 - Review queue budget.** Review queue and candidate deferrals consume existing `claim_review_queue` primitives. Any new budget state has schema, bounds, coalescing, and stale/dead-letter behavior. Queue state is not a local UI-only filter.

**AC8 - Deviation / expected presence.** DOS-316/DOS-811 deviation work consumes ADR-0126's anomaly rule: schema compression must not suppress deviation-flagged claims. `deviation_baselines` or equivalent schema is introduced only if needed, with source_asof/provenance, invalidation, and review semantics.

**AC9 - DOS-277 felt stickiness.** User-authored agenda/prep changes use the hardened prep-layer model from §2.5, survive re-enrichment, affect a different prep/readiness or context surface, and are preserved through DOS-832 rebuild via the prep-correction replay journal. All agenda/notes/prep edit lanes delegate to the hardened writer. The replay key is derived from ADR-0061 calendar-event meeting identity when present, else a keyed hash of source calendar ids, start time, organizer, and normalized attendee set, plus field path; ambiguous matches orphan. The proof distinguishes user-authored state from generated prep content and includes subject, provenance, sensitivity, lifecycle/version, replay key, orphan-on-ambiguous behavior, and invalidation evidence for the user-authored layer.

**AC9a - Durable feedback provenance envelope.** Every W4 feedback write persists a durable correction envelope in the same transaction as `claim_feedback`, keyed by `feedback_id`, with actor/surface, subject, target receipt, field path, ADR-0105/0107 `data_source` / source identifiers / field attribution, `source_asof`, sensitivity, idempotency/replay key, and sanitized action metadata. Tests query the envelope directly; proof cannot rely only on reconstructing provenance through joins.

**AC9b - Artifact lifecycle and non-claim prep privacy.** Correction artifacts implement §2.7 retention, redaction, source-removal, subject/entity deletion/merge/rebind, meeting-removal, reset, replay-safety, and proof-purge behavior through services. Source removal scrubs all source identifiers/source refs across envelopes, reliability deltas/aggregates, propagation jobs/outcomes, lifecycle events, declassification decisions, DOS-338 observations, and proof artifacts, leaving only PII-safe tombstones/keyed hashes. Subject/entity deletion, merge, and rebind scrub or rebind asserted/corrected subject refs across envelopes, `subject_inference_reliability`, propagation jobs/outcomes, lifecycle events, declassification decisions, DOS-338 observations, and proof artifacts, leaving only PII-safe tombstones/keyed hashes and orphaning ambiguous replay/propagation. Prep journal raw payloads have a `UserOnly` sensitivity floor; UserOnly/Confidential taint propagates transitively; MCP/read surfaces must not receive raw text, paraphrase, summary, source pointer, or entity-specific implication unless a current non-semantic declassification decision exists. Lifecycle services reject non-user actors, MCP-originated requests, system/runtime callers, and command-handler bypasses.

**AC10 - DOS-338 eval harness.** Stickiness measurement extends ADR-0110 using the mutation/read split in §1.5/§2.6 and the bounded CI budget in §2.8. Fixtures are hermetic, PII-free, source-attributed, sensitivity-aware, and cover direct re-render, indirect-surface change, re-enrichment, rebuild replay, and forbidden MCP sensitivity gates. The harness must not bypass `ExecutionMode::Evaluate` write blocking by silently writing through eval-mode services.

**AC11 - Cross-entry parity and gates.** W3/DOS-628 readable-file projection feedback is a hard W4 L1 entry gate. App/Tauri feedback and W3 readable-file projection feedback must produce the same substrate effects modulo sensitivity gates before W4 can merge. DOS-832 rebuild replay is required before W4 can merge as the W4 headline. W5 MCP feedback remains W5-owned; W4 defines the MCP parity fixture contract and reports `blocked_by_w5` until W5 lands, but it does not claim MCP parity.

**AC12 - Observability.** Proof reports include correction event id, feedback id, recompute/repair/propagation job ids, logical target list, coalescing keys, affected subject, before/after trust band, direct/indirect surface hashes, re-enrichment run id, rebuild replay id, and PII-safe failure reason codes.

**AC13 - No PII fixtures.** Committed fixtures use generic entities (`account_01`, `person_01`, `project_01`, `user@example.com`) and synthetic content. No real customer/account/person/domain names, emails, local absolute paths, source identifiers, or sensitive source labels appear in source, tests, docs, PR text, commit messages, Linear comments, reviewer artifacts, or proof bundles. Screenshots/textual proof must be fixture-generated or redacted. L1 adds or runs an automated fixture/proof scan so this is not reviewer-only discipline.

**AC13a - Bounded fan-out.** W4 ships §2.8 chunking, durable worker ownership, coalescing, row caps, restart-safe queues, and backpressure tests. Tests prove >25 active-claim subjects do not dead-letter by size, repeated source reliability replay is idempotent without full-history scans, coalesced targets persist scope/cursor/sync/stale/dead-letter/retry state, and multiple feedback events coalesce into bounded propagation rows without silent omission.

**AC14 - Gates.** Focused W4 tests plus full gates pass:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
src-tauri/scripts/check_migrations_transactional.sh
```

If W4 L1 adds migrations or service backfills, the proof bundle must include fresh-install, upgrade-from-current, rerun/idempotency, interrupted-backfill resume, terminal failed/aborted-state, and source-key-version rotation/stale-row exclusion evidence. Migration filenames must follow the repo's filename/version offset convention after the W3/DOS-628 rebase and final slot reconciliation.

---

## §4 Intelligence Loop Integration Check

**1. Claim model.** Corrections are explicit `claim_feedback` events attached to immutable claims. Agenda/prep stickiness uses the user-authored prep layer chosen in §2.5 with equivalent subject, provenance, sensitivity, lifecycle/version, and invalidation semantics. No display-only correction state ships.

**2. Provenance + trust.** Feedback preserves actor, surface, subject, action, per-action metadata, source attribution, source_asof, sensitivity, and field path. Trust-band rendering changes only through trust/source/lifecycle recompute. `CannotVerify` requests corroboration; it does not directly punish trust.

**3. Signals + invalidation.** Feedback emits durable substrate signals and invalidation jobs for claim recompute, derived contexts, salience/surfacing, receipts, prep/readiness, and edge/subject graph changes. Failures are surfaced as job failure, dead-letter, stale marker, or returned error.

**4. Runtime + surfaces.** Consumers include `build_intelligence_context`, `gather_account_context`, prep/readiness outputs, recommendation surfacing, claim receipts, W3 files, and Tauri. W4 defines the MCP fixture contract for W5 but does not claim MCP feedback parity; behavior is equal across W4-supported surfaces except for sensitivity gates.

**5. Feedback loop.** User corrections feed claim lifecycle, source reliability, repair/corroboration queues, salience/ranking, review queue, eval fixtures, and future source/inference weighting. Dismissals and passive engagement are ranking inputs, not truth inputs.

---

## §5 Implementation Surface

Likely files/modules:

- `src-tauri/src/services/claim_receipt/feedback.rs`
- `src-tauri/src/services/claims.rs`
- `src-tauri/src/services/correction_artifacts.rs` or equivalent service-owned lifecycle/redaction module
- `src-tauri/src/services/source_reliability.rs` or equivalent canonical source-key/delta-ledger module
- `src-tauri/src/services/claim_feedback_propagation.rs` or equivalent durable propagation job/worker module
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
- migrations in the reconciled W4 block from §0 only after slot reconciliation
- ADR-0123 amendment documenting the feedback payload redaction privacy mutation exception before L1 code mutates payload-bearing feedback/correction artifacts

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
- Durable correction envelope persists actor/surface/subject/target/source_asof/sensitivity/field-path provenance in the same transaction as `claim_feedback`.
- Source reliability tests cover the canonical source-key helper, version/epoch rebuild stability, key rotation stale-row exclusion or terminal rekey backfill, source-removal exclusion, workspace reset purge/invalidation, claim-type-grained `WrongSource`, `ConfirmCurrent`, qualifying `MarkFalse`, neutral absent rows, legacy fallback, claim-type precedence, delta-ledger retry idempotency, non-source-punishing `WrongSubject`, and `CannotVerify` repair-without-direct-downweight behavior.
- Correction artifact lifecycle tests cover payload scrub mutation, source identifier removal across every W4 artifact table, subject/entity deletion/merge/rebind scrub across correction envelopes, `subject_inference_reliability`, propagation outcomes, lifecycle events, declassification decisions, DOS-338 observations, and proof artifacts, lifecycle event PII-safety, declassification decision revocation, meeting removal, workspace reset/purge, replay suppression for redacted/orphaned entries, and DOS-338 proof purge.
- Lifecycle authorization tests prove redaction, purge, reset, and declassification reject `Actor::Agent`, MCP-originated requests, system/runtime callers, and command-handler bypasses.
- Prep journal privacy tests prove raw user-authored prep text is `UserOnly`, local-only, and excluded from MCP/context outputs while permitted local derived state still changes; tests must include paraphrase/summary/source-pointer leak sentinels and stale-declassification revocation, not only exact-string matching.
- Bounded fan-out tests cover >25 active-claim chunking, source reliability replay without double-count/full-history scan, durable source-reliability backfill run terminalization, durable worker restart, mandatory durable prep-regeneration outbox replay, SQL-level salience `LIMIT 25`/cursor enforcement, default 32-row propagation cap/coalescing with persisted scope/cursor/sync/stale/dead-letter/retry state, stale markers, and propagation outcome logical-target cardinality.
- `claim_feedback_recorded` enqueues/executes recompute and handles queue cap/dead-letter.
- Trust recompute consumes feedback rows idempotently and changes trust band only when factors require it.
- `WrongSubject` / `MarkFalse` tombstone edges and keep active readers from returning the claim.
- `CannotVerify` queues repair and does not directly downweight trust.
- Passive engagement changes salience/ranking only, not trust/lifecycle.
- Recommendation feedback reuses `record_claim_feedback` and changes future surfacing.
- Review queue deferral/budget state is durable and bounded.
- Deviation-flagged claims resist schema-compression demotion.
- User-authored agenda/prep survives recompute/re-enrichment, writes a replay-journal entry, changes an indirect surface, and orphans rather than misattaches when rebuild identity is ambiguous.

Fixture / integration tests:

- App-side correction changes a different Tauri surface after recompute.
- W3 file-projection correction produces the same substrate effects; W4 L1 cannot start without DOS-628 merged.
- MCP correction (after W5) produces the same substrate effects for permitted sensitivity and refuses `Confidential`/`UserOnly`; W4 carries only the fixture contract and does not claim MCP parity.
- Non-claim prep correction influences local prep/readiness while MCP receives no raw, paraphrased, summarized, or source-identifying prep payload and reports a sensitivity gate result.
- Re-enrichment replay does not resurrect tombstoned/wrong-subject claims.
- DOS-832 rebuild replay preserves representative W4 corrections and DOS-277 agenda/prep stickiness.
- DOS-338 hermetic eval fixture computes stickiness pass/fail and records before/after surface hashes.

Proof bundle:

- L0 reviewer verdicts and K-in findings.
- Migration slot reconciliation note naming the final W4 block, every schema artifact from §0, migration filename/version offset validation, fresh-install proof, upgrade proof, and service-backfill idempotency/terminal-state proof.
- Focused test output.
- Full gate output.
- PII-safe stickiness report with event/job/surface hashes.
- Fixture-generated or redacted screenshot/textual proof of indirect surface change if the surface is user-visible.

---

## §7 K-in Findings

Knowledge-store discovery ran against `docs/solutions/` and `.docs/decisions/` for claim feedback, correction, propagation, invalidation, trust bands, review queue, agenda re-enrichment, and eval harness.

Relevant hits:

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md` - new/changed claim producers need a runtime-wide trust audit over source_asof, provenance, trust inputs, runtime consumption, and recompute triggers.
- `docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md` - side-effect emission errors need deliberate emit-or-log wrappers; W4 must distinguish substrate invalidation failures from best-effort UI fan-out.
- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` - new substrate/claim work must answer the full Intelligence Loop check, not only signal invalidation.
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` - prompt/context channels require centralized sensitivity gates; W4 applies this to non-claim prep taint.
- `.docs/decisions/0080-signal-intelligence-architecture.md` - user corrections update source reliability and drive re-enrichment.
- `.docs/decisions/0101-service-boundary-enforcement.md` - W4 mutations, redactions, lifecycle events, and replay go through services.
- `.docs/decisions/0104-execution-mode-and-mode-aware-services.md` - DOS-338 uses the mutation/read split and cannot write through eval mode.
- `.docs/decisions/0105-provenance-as-first-class-output.md` - correction envelopes preserve provenance and field attribution.
- `.docs/decisions/0107-source-taxonomy-alignment.md` - source keys and source removal follow canonical source vocabulary.
- `.docs/decisions/0108-provenance-rendering-and-privacy.md` - rendering/redaction differs by actor/surface and MCP is actor-filtered.
- `.docs/decisions/0110-evaluation-harness-for-abilities.md` - DOS-338 must extend the fixture harness.
- `.docs/decisions/0115-signal-granularity-audit.md` - durable invalidation, coalescing, back-pressure, and no silent drops.
- `.docs/decisions/0061-calendar-event-id-as-meeting-key.md` and `.docs/decisions/0065-meeting-prep-editability.md` - DOS-277 replay should consume stable meeting identity and the existing user-authored prep layer.
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
- `ce-data-migrations-reviewer`: review the reconciled schema ownership plan and required W4 migration artifacts.
- `ce-performance-reviewer`: review recompute, salience, repair, propagation-outcome, and eval fan-out risks.
- `ce-learnings-researcher`: mandatory K-in over `docs/solutions/` and `.docs/decisions/`.

Conditional:

- `ce-product-lens-reviewer`: required if W4 compresses, changes headline proof, or treats DOS-277/DOS-338 as optional.

Approval standard:

- Unanimous L0 approval required before L1 implementation.
- Any finding that W4 ships a display-only correction, a second feedback writer, a second invalidation substrate, or a metric that only proves internal persistence is BLOCKING.

### §8.1 L0 Cycle 7 Approval Record

Cycle 7 reached unanimous L0 approval for the revised W4 packet.

| Reviewer | Verdict | Blocking findings | Carry-forward |
| --- | --- | --- | --- |
| `/codex challenge` | APPROVE | None | Cleaned the non-blocking MCP summary wording after approval. |
| `ce-feasibility-reviewer` | APPROVE | None | Linear should remain the canonical L6 waiver record; L1 must bind source-key version/epoch behavior to the post-DOS-831 keyed-hash helper and avoid generic `migration_state` if it cannot store terminal metadata. |
| `ce-security-lens-reviewer` | APPROVE | None | L1/L2 must prove source-key source-removal/reset exclusion, stale declassification fail-closed behavior, semantic UserOnly leak tests, and no accidental MCP parity claim. |
| `ce-data-migrations-reviewer` | APPROVE | None | L2 must verify the concrete backfill state, migration proof, W3 rebase, and v284-v289 slot ownership. |
| `ce-performance-reviewer` | APPROVE | None | L1 must replace current >25-claim dead-letter behavior, enforce salience SQL/page bounds, and prevent mixed key epochs in worker coalescing. |
| `ce-learnings-researcher` | APPROVE | None | ADR-0123 amendment, terminal backfill state, ADR-0115 no-silent-drop coalescing, and migration filename/version offset convention remain mandatory implementation gates. |

Product/scope reviewer was not invoked because this packet does not compress W4 or make DOS-277/DOS-338 optional. The L6/project-owner waiver in §0.5 allows continued L0 cycles after the two-cycle escalation threshold; it does not waive unanimity or any downstream gate.
