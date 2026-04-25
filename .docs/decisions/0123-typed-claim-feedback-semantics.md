# ADR-0123: Typed Claim Feedback Semantics

**Status:** Accepted
**Date:** 2026-04-24
**Target:** v1.4.0 substrate (FeedbackAction enum + ClaimFeedback row + Trust-compiler effects) / v1.4.2 (UI surfaces beyond inline)
**Extends:** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md), [ADR-0105](0105-provenance-as-first-class-output.md) (SubjectAttribution amendment)
**Closes:** "v1.4.0 Claim Feedback and Prompt Granularity Review" — 7 open decisions

## Context

[ADR-0114](0114-scoring-unification.md) R1.7 placed `user_feedback_weight` as a canonical scoring factor and referenced a `FeedbackAction` enum with placeholder values `StillTrue | Outdated | WrongSource`. [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) §6 declared the agent trust-ledger updates as "User accepts an agent claim → α += 1 / User contradicts → β += 1" without typing accept vs. contradict.

The v1.4.0 Claim Feedback and Prompt Granularity Review identified that a 5-point accuracy scale or yes/no boolean cannot drive distinct trust, provenance, and repair behavior. Different user intents require different DB writes:

- "Wrong account" — subject-fit failure; suppress on this entity; do not punish source.
- "Outdated" — claim true historically; downweight current-state trust; do not retract.
- "Wrong source" — attribution failure; downweight source for this claim_type; keep claim open.
- "Can't verify" — uncertainty; do not punish claim; request corroboration.

Without typed semantics, feedback collected in v1.4.0 would not be translatable into the trust-compiler updates ADR-0114 specifies. **Trust Compiler ships in v1.4.0 spine; therefore feedback semantics close in v1.4.0 spine.** The 7 open decisions from the Linear review are answered below in §3.

## Decision

### 1. The `FeedbackAction` enum (closed)

```rust
pub enum FeedbackAction {
    /// User confirms the claim is true and current.
    /// → α += 1.0 on agent ledger; user corroboration recorded; trust boost via §4.
    ConfirmCurrent,

    /// User says the claim was true historically but is no longer current.
    /// → original claim transitions to `superseded`; user-authored absence claim
    ///   is committed at the same field_path; trust on the original downweights
    ///   via the freshness factor, not the source factor; α += 0.5 on the agent
    ///   (the claim was correctly authored once).
    MarkOutdated,

    /// User says the claim is false.
    /// → claim transitions to `withdrawn`; tombstone written if no remaining
    ///   sources support an alternate value; β += 1.0 on agent ledger; one-shot
    ///   modest source downweight (-0.05) — single mistakes may be misclassification.
    MarkFalse,

    /// User says the content is true but attached to the wrong subject (account,
    /// project, person, meeting). Carries the corrected subject when the user
    /// supplied one; otherwise null = "not relevant to this entity, don't know
    /// which one is right."
    /// → claim suppressed on the asserted subject (per-subject tombstone);
    ///   subject_evidence flagged for the inference path that produced this
    ///   attachment; β += 0.3 on agent (subject mistake, not truth mistake);
    ///   source reliability NOT downweighted.
    /// → consumes the typed `SubjectRef` from ADR-0105 SubjectAttribution amendment.
    WrongSubject { corrected_to: Option<SubjectRef> },

    /// User says the cited source does not actually support the claim.
    /// → source attribution downweighted (-0.2) on this source for this claim_type;
    ///   claim remains open if other sources clear the trust floor; transitions
    ///   to `withdrawn` if no remaining sources qualify; β += 0.2 on agent.
    WrongSource { source_index: usize },

    /// User cannot verify the claim from surfaced evidence.
    /// → no trust delta; claim flagged with `NeedsCorroboration` warning;
    ///   targeted repair job enqueued (per §7); if no corroboration found within
    ///   the corroboration window (default 14 days), claim downweights via
    ///   `lack-of-corroboration`, not user-rejection.
    CannotVerify,

    /// User says the claim is partly right; submits corrected text.
    /// → user-authored superseding claim is committed at the same field_path;
    ///   original claim transitions to `superseded`; source remains attributed
    ///   to the original; agent ledger gets α += 0.3 if the correction reads as
    ///   refinement (text-overlap heuristic ≥ 0.5) or β += 0.3 if contradiction.
    NeedsNuance { corrected_text: String },

    /// Render/privacy concern, not a truth concern. The claim should not appear
    /// in this surface (e.g., internal note shown as customer-facing fact).
    /// → surface-specific suppression marker recorded; no claim state change;
    ///   no source penalty; feeds ADR-0108 surface privacy policy.
    SurfaceInappropriate { surface: SurfaceId },

    /// User says the claim is true but not relevant to this meeting/context.
    /// → context-binding hint recorded against the ability invocation that
    ///   surfaced it (relevance signal, not truth signal); no trust delta.
    NotRelevantHere { invocation_id: InvocationId },
}
```

Nine variants, each mapping to a distinct triple of (claim state, source weight, agent ledger). No 5-point scale. No yes/no.

### 2. `ClaimFeedback` row shape (closed)

```rust
pub struct ClaimFeedback {
    pub id: ClaimFeedbackId,
    pub claim_id: ClaimId,
    pub subject: SubjectRef,                  // From ADR-0105 SubjectAttribution amendment
    pub action: FeedbackAction,
    pub note: Option<String>,
    pub created_by: ClaimActor,               // Almost always User in v1.4.0; future agent-to-agent feedback
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,    // Set by Trust Compiler when consumed
}
```

Stored in `claim_feedback` table. **Append-only.** The Trust Compiler reads the latest unconsumed rows on each scoring pass and writes `applied_at` after consuming. Re-application is idempotent: a row with `applied_at IS NOT NULL` is skipped.

### 3. The 7 open decisions, closed

| # | Question | Answer | Where it lives |
|---|---|---|---|
| 1 | Which feedback actions are first-class in v1.4.0 vs. v1.4.2? | All nine variants are first-class in the **enum and DB row** in v1.4.0 (DOS-294). Inline UI surfaces in DOS-8 expose six (`ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`) behind a low/medium-trust gate. The remaining three (`NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`) ship behind an overflow menu in v1.4.2. | §1; UI work split between DOS-8 (v1.4.0) and v1.4.2 |
| 2 | Does `CannotVerify` affect trust score directly? | No direct delta. Triggers a `NeedsCorroboration` warning + repair-job enqueue. If no corroboration found within 14 days, the claim downweights via `lack-of-corroboration` in `scoring::factors`, not via user-rejection. | §1 `CannotVerify`; §7 |
| 3 | Does `New to me` need to exist as feedback? | No. It is a personal note/bookmark, not feedback. Use the existing notes affordance. | Out of scope |
| 4 | Should user confidence be optional metadata? | No, hidden entirely for v1.4.0. A confidence slider is what the 5-point scale tried to be; rejected. May revisit in v1.5.0+ if a specific use case emerges. | Out of scope |
| 5 | How many feedback actions can appear inline? | Six inline (the first six in §1) — short enough to not crowd a claim row, long enough to cover ~95% of expected user intent. The remaining three live in an overflow menu in v1.4.2. | Decided; UI implementation split per #1 |
| 6 | Which claim types are editable directly vs. structured-only? | `NeedsNuance` with free text is allowed for: account assertions, person role descriptions, project status text. Structured-only (no `NeedsNuance`; only typed corrections via `WrongSubject` / `WrongSource` / `MarkFalse`): stakeholder roles, commitment owners, renewal dates, health bands. The structured-only set is anything where the UI already provides a typed input — text edit there would defeat the typing. | Decided; enforced by `claim_type` metadata on the claim row (DOS-7) |
| 7 | Query/concurrency budget for targeted claim repair? | Per `CannotVerify`: one repair job enqueued, executes within 24 hours, capped at 1 LLM call + 1 retrieval batch (≤10 sources). Per entity: max 5 active repair jobs. Per workspace: max 50 active repair jobs. Excess queues with low priority. | §7; enforced via DOS-236 invalidation queue |

### 4. Trust Compiler integration

The Trust Compiler (DOS-5) consumes `ClaimFeedback` via the `user_feedback_weight` factor (per ADR-0114 R1.3, the factor takes pre-computed `age_days`, not a clock). Per-action deltas:

| Action | Source weight Δ | Agent ledger Δ | Claim state | Freshness Δ | Subject evidence Δ |
|---|---|---|---|---|---|
| `ConfirmCurrent` | +0.10 (capped) | α += 1.0 | unchanged | refreshed | unchanged |
| `MarkOutdated` | none | α += 0.5 | → `superseded` | downweight to historical | unchanged |
| `MarkFalse` | -0.05 (one-shot) | β += 1.0 | → `withdrawn` | n/a | unchanged |
| `WrongSubject` | none | β += 0.3 | → tombstoned at this subject | unchanged | -0.3 on the inference method that produced this attachment |
| `WrongSource` | -0.20 on cited source for this claim_type | β += 0.2 | unchanged unless no sources remain qualified | unchanged | unchanged |
| `CannotVerify` | none | none | unchanged | unchanged | unchanged; triggers repair |
| `NeedsNuance` | none | α += 0.3 (refinement) or β += 0.3 (contradiction) by text-overlap heuristic ≥ 0.5 | original → `superseded`; new claim committed | refreshed on new claim | inherits |
| `SurfaceInappropriate` | none | none | unchanged | unchanged | unchanged; surface-policy update |
| `NotRelevantHere` | none | none | unchanged | unchanged | unchanged; relevance signal feeds DOS-216 eval |

These deltas are the spine. **Tuning happens in v1.4.1** against shadow data; the `agent_trust_ledger` and source-weight tables are designed to recompute, so retroactive tuning is safe.

### 5. Substrate-shaping consequences

- `scoring::factors::user_feedback_weight(feedback: Option<&UserFeedbackInput>)` (ADR-0114 R1.3) consumes a `UserFeedbackInput { action: FeedbackAction, age_days: f64 }`. Pure; the extractor pulls latest feedback per claim per ADR-0114 §2.
- ADR-0113 §6 agent-ledger updates are typed by `FeedbackAction` rather than abstract "accept/contradict" — see §4.
- ADR-0113 §5 tombstone behavior is now triggered by `WrongSubject` (per-subject tombstone) and `MarkFalse` (broad tombstone), not by an untyped UI gesture.
- DOS-294 ships the `FeedbackAction` enum + `ClaimFeedback` row + service functions (`propose_feedback`, `apply_feedback_to_trust`).
- DOS-8 inline UI uses the typed enum directly. No more untyped `feedback_yes/no/wrong_source` handlers.

### 6. Subject correction interplay (ADR-0105 amendment)

`WrongSubject { corrected_to }` is the user-side complement to ADR-0105's `SubjectAttribution` substrate primitive. Behavior:

- `corrected_to: None` — "this content does not belong to this entity; I don't know whose it is." Tombstone the claim at the asserted subject only; do not propose a new attachment.
- `corrected_to: Some(SubjectRef)` — "this content belongs to that entity instead." Tombstone at the asserted subject; propose (not commit) a new claim at the corrected subject with the same claim_text, full provenance preserved, `actor = 'user_correction'`. The corrected claim enters via the standard propose/commit path so it gets the same gate as any other claim.

The inference method that produced the wrong attachment receives `subject_evidence` downweight `-0.3`. After repeated `WrongSubject` feedback against the same `InferenceMethod` for the same source class, the inference method gets globally suppressed for that source class (threshold: 5 instances within 30 days).

### 7. Repair scheduling for `CannotVerify`

When `CannotVerify` is recorded:

1. A `claim_repair_job` row is enqueued via DOS-236 invalidation queue with `priority = normal`.
2. Job retrieves up to 10 candidate corroborating sources via Glean + local context.
3. Job invokes a `find_corroborating_evidence` Read ability (lands in v1.4.1; v1.4.0 ships the queue + skeleton).
4. Found corroboration → claim corroboration record (ADR-0113 R1.6); user notified.
5. No corroboration within 14 days → claim downweights via `lack-of-corroboration` factor; lint surface flags it.

Budgets: 1 LLM call + 1 retrieval batch per job; max 5 active repair jobs per entity; max 50 per workspace. Excess queues at `priority = low`.

### 8. Non-goals for v1.4.0

- UI surfaces beyond DOS-8's inline six. Overflow menu (`NeedsNuance` editor, `SurfaceInappropriate`, `NotRelevantHere`), claim detail panel, lint feedback debt — v1.4.2.
- Cross-claim feedback aggregation ("user often marks Glean Salesforce sources wrong" → systematic source downweight). Tracked separately for v1.5.0+.
- Feedback from non-User actors. Enum supports `created_by: ClaimActor` for future agent-to-agent feedback; v1.4.0 commits user-only.
- Per-action delta tuning. The §4 numbers are starting values; v1.4.1 shadow data tunes them.
- `find_corroborating_evidence` Read ability — v1.4.1; v1.4.0 ships the queue + skeleton.

## Consequences

### Positive

- Trust Compiler's `user_feedback_weight` factor has a typed input, not a placeholder.
- Per-action behavior is specified before code lands. No ambiguity about whether `MarkFalse` punishes the source or the claim.
- `WrongSubject` becomes a substrate-level operation pairing cleanly with ADR-0105 `SubjectAttribution`. Subject mistakes do not punish sources.
- `CannotVerify` does not punish claims for user uncertainty — closes the feedback-as-noise risk that yes/no would have introduced.
- The 7 open decisions are answered before kickoff. No spec churn during the spine push.

### Negative / risks

- Nine variants is more than the placeholder three. UI overflow menu in v1.4.2 must handle the additional three cleanly.
- `NeedsNuance` text-overlap heuristic for "refinement vs. contradiction" is approximate. Acceptable in v1.4.0; can be replaced with a small classifier in v1.4.1.
- Per-action deltas in §4 are unvalidated starting values. Mitigated by shadow data + tuning in v1.4.1; the underlying tables are designed to recompute, so retroactive tuning is safe.
- Repair job queue depends on DOS-236 (durable invalidation), which is v1.4.1 scope. v1.4.0 ships the `claim_repair_job` row schema + enqueue stub; the worker runs in v1.4.1. `CannotVerify` feedback collected in v1.4.0 is queued and processed once the worker lands.

### Neutral

- No new tables beyond `claim_feedback` and `claim_repair_job` (skeleton). ADR-0113's existing tables absorb the structural updates.
- DOS-8 inline buttons remain; their handlers route to `propose_feedback(action)` rather than untyped boolean handlers.

## References

- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `SubjectAttribution` amendment supplies `SubjectRef` consumed by `WrongSubject`.
- [ADR-0113: Human and Agent Analysis as First-Class Claim Sources](0113-human-and-agent-analysis-as-first-class-claim-sources.md) — agent-ledger updates typed via this ADR (§6 amended by §4 here).
- [ADR-0114: Scoring Unification](0114-scoring-unification.md) — `user_feedback_weight` factor consumes the typed action; R1.3 `UserFeedbackInput` shape adopted here.
- v1.4.0 Claim Feedback and Prompt Granularity Review (Linear doc 0b668ff1c80a) — sourcing of the 7 decisions closed in §3.
- DOS-294 — implementation issue; ships `FeedbackAction` + `ClaimFeedback` + service functions.
- DOS-8 — inline UI surfacing the six primary actions in v1.4.0; remaining three in v1.4.2.
