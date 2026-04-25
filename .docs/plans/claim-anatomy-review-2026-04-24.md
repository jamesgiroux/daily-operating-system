# Claim Anatomy Review — 2026-04-24

**Purpose:** systematic walk-through of every dimension a claim should express in the v1.4.x substrate. Identifies gaps before code starts. Produces an explicit decision per dimension: spine / v1.4.1 / v1.4.2+ / out-of-scope.

**Why now:** SubjectAttribution (this morning), ThreadId (this afternoon), and recency / `source_asof` (later this afternoon) all surfaced as substrate gaps reactively. Each was real. The pattern suggests there are more. Cheap to enumerate before code starts; expensive after.

**Method:** for each dimension below — what is it, what does the substrate cover today, what's the gap, decision.

---

## Covered (no action)

### 1. Identity & lineage

`id`, `claim_sequence`, `previous_claim_id`, `superseded_by`, `superseded_at`, `dedup_key`. ADR-0113 R1.6 + DOS-7.

### 2. Actor

`ClaimActor` enum: `User` / `UserRemoval` / `Human { role, id }` / `Agent { name, version }` / `System { component }` / `External { source }`. ADR-0113 §1, R1.5.

### 3. Subject

`SubjectRef` (Account / Project / Person / Meeting / Multi / Global) + `SubjectEvidence` (InputBound / SourceMatched / Inferred). ADR-0105 amendment 2026-04-24.

### 4. Source attribution

`source_refs`, `prompt_fingerprint`, `synthesis_marker`. ADR-0105 §4. **Note: §10 below amends `source_asof` semantics; the field exists, the population path doesn't.**

### 5. Trust

`TrustAssessment` with 6 named factors (source reliability, freshness, corroboration count, contradiction signal, user feedback, subject-fit confidence). ADR-0114 + DOS-5.

### 6. State machine

`proposed` / `committed` / `tombstoned` / `superseded` / `withdrawn`. ADR-0113 R1.1.

### 7. Corroboration & contradiction

`claim_corroborations` and `claim_contradictions` child tables. ADR-0113 R1.6 / §7.

### 8. Threading

`thread_id` substrate allowance (column + envelope field; meaning deferred to v1.4.2 DOS-297). ADR-0124 / DOS-296.

### 9. Feedback semantics

`ClaimFeedback` table with closed-form 9-variant `FeedbackAction` enum. ADR-0123 / DOS-294.

---

## Gaps requiring action

### 10. Recency / evidence-asof — **SPINE**

**Current state.** `source_asof: Option<DateTime<Utc>>` exists in ADR-0105 §4 as optional metadata. No code path populates it. `glean_provider.rs:1182` stamps `observed_at = now`. `dimension_prompts.rs:608` asks the LLM for `sourcedAt` per item, but the returned timestamp lives in a JSON blob and is never lifted into a first-class column.

**Effective behavior today.** Every claim freshly synthesized from a 6-month-old Salesforce note looks fresh. Trust Compiler `freshness_weight` (DOS-10) will be calibrated against ingestion time, not evidence time.

**Decision:** SPINE substrate primitive. Amend ADR-0105 with `source_asof` population semantics + `SourceTimestampUnknown` warning + freshness fallback chain. New spine issue (DOS-299, see below) extends DOS-211 + DOS-5 scope.

### 11. Temporal scope of the claim — **SPINE schema allowance**

**Dimension.** A claim like "Bob said X in 4/23 meeting" (PointInTime) has fundamentally different freshness, supersession, and contradiction semantics than "Bob is the champion" (State) or "engagement has been declining" (Trend). Today they collapse into the same shape.

**Why it matters for spine.** DOS-10 freshness decay (in v1.4.1) will be wrong for PointInTime claims unless the claim row carries the distinction. PointInTime doesn't decay — the event really happened. State decays. Trend's freshness is the recency of underlying data points.

**Decision:** SPINE schema allowance via ADR-0125. Add `temporal_scope` column + enum, default `State`. v1.4.1 DOS-10 freshness consults it. Spine ships the field; v1.4.1 ships the meaning. Same pattern as ThreadId.

### 12. Sensitivity / surface eligibility — **SPINE schema allowance**

**Dimension.** Internal-only notes about stakeholder personal context should *structurally* never surface on customer-facing prep. ADR-0108 handles surface rendering rules but the claim itself doesn't carry a sensitivity tier. Today enforced by surface logic, not by the claim row.

**Decision:** SPINE schema allowance via ADR-0125. Add `sensitivity` column + `ClaimSensitivity` enum (`Public` / `Internal` / `Confidential` / `UserOnly`), default `Internal`. v1.4.1 DOS-214 enforces at provenance render layer. v1.4.2 entity surfaces + v1.4.3 briefing surfaces enforce ceiling per surface.

### 13. Counter-claim awareness denormalized — **v1.4.1**

**Dimension.** When a claim is committed and a contradicting claim later appears, the original claim doesn't structurally know "I have an active contradiction." Every read path JOINs `claim_contradictions` to find out.

**Decision:** v1.4.1 amendment. Add `has_active_contradiction: bool` denormalized column to `intelligence_claims`, maintained by trigger or service code on contradiction insert/resolve. Optimization, not load-bearing for spine — JOINs work in spine.

### 14. Causal lineage between claims — **v1.5.x**

**Dimension.** "This claim exists because of that claim." If claim A "Bob is champion" leads to claim B "champion is healthy," retracting A should... cascade? mark B as orphaned? Today nothing structural.

**Decision:** out of v1.4.x scope. Genuinely hard design work; connects to recommendations layer lineage. Saved as memory: known design question.

### 15. Claim type taxonomy — **SPINE primitive**

**Dimension.** `claim_type` is currently a free-form string. Dedup (per claim_type), freshness decay (per claim_type per DOS-10), commit policy gates, and rendering policy all key on this string. Drift is invisible — a Transform ability that emits `stakeholder_role` vs `champion_role` for semantically the same thing breaks every consumer silently.

**Decision:** SPINE substrate primitive via ADR-0125. `ClaimTypeRegistry` const slice — compile-time exhaustive. Pattern mirrors ADR-0115 Signal Policy Registry. Initial set covers DOS-218 + DOS-219 outputs (~10-15 entries). New types require ADR amendment + registry-extension PR.

### 16. Reversibility / undo — **NOT A CLAIM CONCERN**

**Dimension.** Claims that drove Publish actions vs informational claims have different reversal semantics.

**Decision:** lives at action / publish level (ADR-0117 Pencil/Pen, ADR-0103 maintenance constraints). Out of scope for claim anatomy.

### 17. Locale / temporal interpretation — **v1.4.1**

**Dimension.** "Friday" / "next week" / "before launch" in transcripts means different things in different timezones. Today resolution is implicit at extraction time.

**Decision:** v1.4.1 amendment. Add `resolved_with_locale: Option<Locale>` to `FieldAttribution` where temporal parsing happened. Not load-bearing for spine — extraction logic already uses `ctx.user`'s timezone implicitly; we just need to record the resolution explicitly.

### 18. Decision relevance / actionability tier — **v1.5.x**

**Dimension.** Some claims are background ("Bob's title changed"); some should drive prompts ("renewal risk shifted from low to moderate"). Today claims are all "facts" — no first-class distinction.

**Decision:** v1.5.x with recommendations layer (memory: `recommendations_layer_vision.md`). The classification itself is hard; couples to recommendation surfacing.

---

## Summary

### Spine additions (4 substrate primitives, in 2 new spine issues)

| # | Dimension | ADR | Issue |
|---|---|---|---|
| 10 | `source_asof` semantics + freshness fallback chain | ADR-0105 amendment | DOS-299 (new) |
| 11 | Temporal scope schema allowance | ADR-0125 | DOS-300 (new, combined) |
| 12 | Sensitivity schema allowance | ADR-0125 | DOS-300 |
| 15 | Claim type registry | ADR-0125 | DOS-300 |

Spine count: **15 → 17 issues**.

### v1.4.1 amendments (no new issues; scope additions to existing work)

- DOS-10: consume `temporal_scope` in `freshness_weight` factor; supersession semantics parameterized by scope.
- DOS-214: enforce `sensitivity` at provenance render layer.
- v1.4.1 backlog: `has_active_contradiction` denormalized boolean (#13).
- v1.4.1 backlog: `resolved_with_locale` on FieldAttribution (#17).

### v1.5.x (memory only; no issues yet)

- Causal lineage between claims (#14).
- Decision relevance / actionability tier (#18) — couples to recommendations layer.

### Out of scope (declared, not silently deferred)

- Reversibility (#16) — lives at action/publish level, not claims.

---

## What's not addressed and why

- **Composability of multiple temporal scopes on one claim.** A claim like "Bob has been champion for 6 months" is both State (current role) and Trend (duration). v1.4.0 ships single `temporal_scope` per claim; multi-scope claims model as multiple claims. Acceptable.
- **Sensitivity inheritance from sources.** A claim sourced from an Internal Salesforce note should default `Internal`. v1.4.0 ships defaults but no automatic source-class inheritance. v1.4.1 adds inheritance via `ClaimTypeMetadata.default_sensitivity`.
- **Cross-claim type relationships.** "Champion role" claim and "champion email engagement" claim are about the same person — should they share claim graph edges? Touched by ADR-0124 threading and the future causal lineage work; not a claim-row primitive.
- **Hash-based content stability.** `dedup_key` already content-addresses; we don't separately track "claim text fingerprint" because the dedup_key serves that purpose.

---

## Validation

This review is the substrate's last design pass before code starts. The Tuesday gate (Golden Daily Loop on bundles 1 + 5) validates against user-visible failure modes. This review validates against substrate completeness.

**Bar for post-kickoff additions:** if a new dimension surfaces after code starts, the bar is high — ADR amendment proposing change + impact analysis + decision before code touches the substrate. The right-thing-by-default principle requires that the substrate be settled before abilities ship; chasing dimensions after the fact is exactly the pattern v1.4.0 was rescoped to avoid.
