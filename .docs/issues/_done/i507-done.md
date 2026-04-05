# I507 — Source-Attributed Correction Feedback

**Priority:** P2
**Area:** Backend / Signals + Intelligence
**Version:** 1.1.0
**Depends on:** I487 (Glean signal emission), I504 (AI-inferred relationships), I505 (Glean stakeholder intelligence)

## Problem

The feedback system has a working Bayesian engine (Thompson Sampling in `signals/bus.rs`, Beta distributions in `signal_weights`) but it's narrowly wired. User corrections to intelligence quality don't propagate back to the source that produced the bad data. This means the system never learns which sources are reliable for which kinds of intelligence — and as more sources come online (Glean, co-attendance, AI-inferred relationships), the gap compounds.

### What works today

**Entity resolution corrections only.** When a user replaces one meeting-entity link with another (`MeetingEntityMutation::Replace` in `services/meetings.rs`), `signals/feedback.rs::record_correction()`:
1. Finds which signal source recommended the wrong entity
2. Penalizes that source (beta += 1 in `signal_weights`)
3. Rewards sources that pointed to the correct entity (alpha += 1)

This is the only correction type that closes the full feedback loop: user action → identify wrong source → penalize → future signals from that source carry less weight.

**Enrichment quality corrections (partial).** `self_healing/feedback.rs::record_enrichment_correction()` penalizes a source when a user edits an enriched field. But it blames coarsely — `"intel_queue"` for any intelligence field edit, `"clay"` for profile fields. It never identifies that a specific risk assessment came from a Glean document vs. an email thread vs. a transcript.

### What's lost

| User Action | What Happens | What Should Happen |
|---|---|---|
| User edits a risk that came from Glean context | `intel_queue` blamed generically | `glean_search` penalized for that entity type |
| User corrects a person's title (Glean-sourced) | `clay` blamed (wrong source) | `glean_org` penalized |
| User dismisses an email signal | Signal emitted at 0.3 confidence, stops there | Source that generated the email signal penalized |
| User corrects email disposition | Written to `email_signals` as `signal_type = "feedback"`, never read | Email scoring model recalibrated |
| User finds meeting prep irrelevant | `dismissedTopics` stored in prep JSON, never read | Sources contributing to those topics penalized |
| User reopens a completed action | `action_reopened` signal emitted, stops there | Source that marked the action complete penalized |

The pattern: every correction flows into `signal_events` as a `user_correction`-sourced signal (good for propagation and prep invalidation), but none of them identify which upstream source was wrong. The Bayesian engine would work perfectly if told "Glean was wrong about this risk" — but nobody tells it.

### Why this matters for VP surfaces

Better day-to-day intelligence quality at the account level compounds upward. When a CSM corrects a risk assessment, that correction should:
1. Penalize the source that produced the bad risk (Glean? email? transcript?)
2. Lower that source's weight for future assessments of similar entities
3. Result in better synthesis for the VP Account Review, Portfolio Health Summary, and Stakeholder Map reports

Without source attribution, the system treats all sources as equally reliable forever. A Glean integration that consistently surfaces irrelevant documents gets the same 0.7 weight after 100 corrections as after zero. The VP's reports are built on unlearning intelligence.

## Scope Reduction (Phase 2 Review)

**LLM source attribution (§1-2 below) is deferred to post-v1.0.0.** The approach of asking the LLM to attribute which context sources informed each output field is:
1. **Unverified** — we have no evidence that LLMs reliably self-attribute reasoning to input sources. They may hallucinate attribution ("I based this on the Glean document") when the conclusion actually came from email context.
2. **Architecturally risky** — if attribution is unreliable, penalizing sources based on it makes the Bayesian engine worse, not better. Bad data in → bad learned weights.
3. **Token-expensive** — adding `source_attribution` to the JSON schema increases prompt and response size for every enrichment call.

**What ships in v1.0.0:** Person profile corrections (§3) and email disposition feedback (§4) only. These use **existing provenance** (enrichment_sources on people table, source on email_signals) — no LLM attribution needed.

**Post-v1.0.0:** Evaluate LLM source attribution as a separate experiment. Test against ground truth: given known inputs, does the LLM correctly attribute which source informed its output? If accuracy > 80%, ship it.

## Design

### ~~1. Source attribution on intelligence fields~~ (DEFERRED post-v1.0.0)

~~The core gap is that `intelligence.json` doesn't track which input sources informed which output fields.~~

Deferred. See "Scope Reduction" above. The `source_attribution` field on `IntelligenceJson` (defined in I508) remains as a schema placeholder but is not populated in v1.0.0.

### ~~2. Correction routing in `services/intelligence.rs`~~ (DEFERRED post-v1.0.0)

~~Replace generic `"intel_queue"` blame with source-attributed correction routing.~~

Deferred. Without reliable source attribution data, this would make things worse.

### 3. Person profile correction routing

When a user corrects a person's title, department, or role, the `enrichment_sources` JSON on the `people` row (maintained by `update_person_profile()`) already tracks which source last wrote each field. Use this provenance:

```rust
// In services/people.rs, when user edits a profile field
if let Some(prior_source) = enrichment_sources.get(field_name) {
    let _ = upsert_signal_weight(
        db, prior_source, "person",
        "profile_enrichment",
        0.0, 1.0,  // penalize source that wrote the wrong value
    );
}
```

This is the simplest win — provenance already exists on the `people` table. A Glean-sourced title that gets corrected penalizes `"glean"` for `("person", "profile_enrichment")`. Future Glean profile writes for this entity type get a lower reliability weight.

### 4. Email disposition feedback (close the dead end)

`correct_email_disposition` in `commands.rs` writes to `email_signals` with `signal_type = "feedback"` and stops. Wire this to `upsert_signal_weight`:

```rust
// First, read the original email signal to identify which source scored it
let original = db.get_email_signal_by_id(&signal_id)?;
let source = original.source.as_deref().unwrap_or("email_enrichment");

// Then penalize that source
let _ = upsert_signal_weight(
    db,
    source,
    "email",
    "email_priority",
    0.0, 1.0,  // penalize: beta += 1
);
```

**Note:** The `email_signals` table must have a `source` column tracking which enrichment pipeline scored the email. Verify this exists — if not, the email enrichment pipeline needs to set it (e.g., `"email_enrichment"` or `"ai_classification"`).

### 5. Activation threshold awareness

The existing 5-correction activation threshold in `get_learned_reliability()` means source attribution corrections won't affect weights until 5 corrections accumulate for a given `(source, entity_type, signal_type)` triple. This is appropriate — it prevents over-fitting to a single correction. But it means the feedback loop needs volume to activate.

For Glean specifically: a user who corrects 5 Glean-sourced risks across their accounts will start seeing Glean's risk contribution downweighted. This is the "day-to-day corrections compound upward" effect.

## Files to Modify

| File | Change |
|---|---|
| ~~`src-tauri/src/intelligence/prompts.rs`~~ | ~~Add `source_attribution` to enrichment JSON schema.~~ DEFERRED — `source_attribution` remains as a schema placeholder on `IntelligenceJson` (from I508) but is not populated. |
| ~~`src-tauri/src/intelligence/io.rs`~~ | ~~Parse source_attribution from LLM output.~~ DEFERRED. |
| ~~`src-tauri/src/services/intelligence.rs`~~ | ~~Source-attributed correction routing.~~ DEFERRED. |
| `src-tauri/src/services/people.rs` | On user profile field edit, read `enrichment_sources` provenance, penalize the source that wrote the wrong value. |
| `src-tauri/src/commands.rs` (`correct_email_disposition`) | Wire email disposition feedback to `upsert_signal_weight`. Close the dead end. |

## Acceptance Criteria (v1.0.0 — scoped down)

1. ~~After enrichment, `intelligence.json` includes `source_attribution`~~ — DEFERRED
2. ~~User corrects a risk field → source penalized~~ — DEFERRED (requires reliable LLM attribution)
3. ~~After 5+ corrections, learned reliability < 0.5~~ — DEFERRED
4. User corrects a Glean-sourced person title. `"glean"` penalized for `("person", "profile_enrichment")` via `enrichment_sources` provenance on the people table.
5. User corrects an email disposition. Source that scored the email is penalized for `("email", "email_priority")`. The existing dead-end in `correct_email_disposition` is wired to `upsert_signal_weight`.
6. ~~VP Account Review report~~ — DEFERRED
7. `signal_weights` table has rows after person profile corrections or email disposition corrections — visible via `SELECT * FROM signal_weights`

## Out of Scope

- **Meeting prep relevance feedback** (dismissed topics → source penalization) — requires deeper attribution than field-level; the LLM synthesizes topics from multiple sources. Future issue.
- **Action completion quality feedback** (reopened actions → source penalization) — requires attributing which source recommended the action. Future issue.
- **Positive feedback** (user confirms an intelligence field is correct → alpha += 1) — v1 focuses on correction-only feedback. Positive reinforcement is a natural extension but adds UI complexity.
- **Cross-entity learning** ("Glean is unreliable for enterprise accounts") — the `signal_weights` table already has `entity_type` as a dimension, so this happens naturally as corrections accumulate per entity type. No special handling needed.
- **Real-time weight display** — showing users "Glean reliability: 62%" in the UI. Future transparency feature.
