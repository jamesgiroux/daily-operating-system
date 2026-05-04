# I509 — Transcript Personal Interpretation + Sentiment Extraction

**Priority:** P1
**Area:** Backend / Intelligence
**Version:** v1.0.0 (Phase 2)
**Depends on:** I508a (intelligence schema types), I503 (TranscriptSentiment type)
**Absorbs:** I501 (transcript sentiment extraction — merged here as the sentiment fields are a subset of interaction dynamics)

## Scope Change (2026-03-03)

The first-principles review (`.docs/research/2026-03-03-architecture-first-principles-review.md`) established a clear boundary: **org-level call analysis belongs in Glean; personal interpretation belongs in DailyOS.** This reframes I509.

**What moves to Glean (out of I509 scope):**
- Talk ratios, question density, monologue detection — org-level interaction metrics. Gong extracts these; Glean indexes them. When Glean Agents ship (v1.1.0), DailyOS consumes these as structured input.
- Objective per-speaker sentiment assessment based on transcript content analysis. Glean has the org context (who is this person, what's their role, what's the deal context) to do this better.

**What stays in I509 (personal interpretation):**
- **How this call moves MY priorities** — did expansion signals I've been tracking strengthen? Did the champion's tone shift after I raised pricing? Is the risk I flagged 3 meetings ago materializing?
- **Relationship trajectory tracking** — not "what was the sentiment in this meeting" but "how has my relationship with this stakeholder evolved across meetings?"
- **Competitor context through personal lens** — not "competitor X was mentioned" (Glean can do that) but "competitor X was mentioned in the context of the renewal I'm managing — what does this mean for my strategy?"
- **Escalation signals that require personal context** — the same escalation language means different things depending on the user's relationship history and priorities.
- **Sentiment as a signal** — structured sentiment extraction still lives here because DailyOS needs it as a local signal for health scoring (I499). Whether Glean eventually provides richer sentiment doesn't remove the need for a local extraction that feeds the signal bus.

**Practical impact:** The INTERACTION_DYNAMICS prompt section is simplified. We keep SENTIMENT (local signal bus needs it) and personal interpretation fields. We drop talk_balance, question_density, and monologue_risk (Glean's lane). We add a PERSONAL_INTERPRETATION section that requires user context (priorities, relationship history) to be meaningful.

## Problem

DailyOS processes call transcripts but only extracts outcomes — what happened. It doesn't extract **what this meeting means for the user's priorities and relationships.** The enrichment pipeline later synthesizes meeting outcomes into entity intelligence, but the per-meeting signal is impoverished: wins/risks/decisions are coarse, there's no structured sentiment, and there's no connection to the user's evolving priorities.

Email enrichment already extracts structured sentiment. Meeting sentiment is arguably more valuable — a tense meeting is a stronger signal than a terse email — but currently the only sentiment signal from meetings is indirect: wins count as positive, risks count as negative, via the `transcript_outcomes` signal. ADR-0097 defines structured transcript sentiment fields that feed directly into health scoring dimensions.

### What we extract today vs. what Gong extracts

| Signal | DailyOS Today | Gong |
|--------|--------------|------|
| Meeting summary | Yes | Yes |
| Action items | Yes (with owners, priority, due dates) | Yes |
| Wins/risks/decisions | Yes | Partial (deal warnings) |
| Strategic analysis | Yes (TAM-perspective insight) | No |
| **Talk ratio** (rep vs. customer) | **No** | Yes |
| **Per-speaker sentiment** | **No** (overall only) | Yes |
| **Question density** | **No** | Yes |
| **Competitor mentions** | **No** | Yes (configurable trackers) |
| **Escalation language** | **No** | Yes |
| **Decision-maker engagement** | **No** | Yes (engagement map) |
| **Forward-looking language** | **No** | Yes |
| **Monologue detection** | **No** | Yes |

The right-hand column represents signals that feed directly into I508's intelligence dimensions — and we're leaving them on the table.

### How these feed the intelligence schema (I508)

| Extraction | I508 Dimension | Field It Feeds |
|-----------|---------------|---------------|
| Per-speaker sentiment | Relationship Health | `stakeholder_insights` (engagement level per person) |
| Decision-maker engagement | Relationship Health | `relationship_depth` (executive access assessment) |
| Competitor mentions | Strategic Assessment | `competitive_context` (new field) |
| Escalation language | Strategic Assessment | `risks` (with cited evidence) |
| Talk ratio | Engagement Cadence | `meeting_cadence` (meeting quality, not just frequency) |
| Question density | Engagement Cadence | `meeting_cadence` (discovery vs. status update) |
| Forward-looking language | Strategic Assessment | `executive_assessment` (optimism/pessimism signal) |
| Monologue detection | Engagement Cadence | `meeting_cadence` (one-sided = risk signal) |

## Design

### 1. Extend transcript extraction prompt

Add `SENTIMENT` and `INTERACTION_DYNAMICS` sections to `build_transcript_prompt()`.

#### SENTIMENT section (from I501)

```
SENTIMENT:
- overall: positive|neutral|negative|mixed
- customer: positive|neutral|negative|mixed
- engagement: high|moderate|low|disengaged
- forward_looking: yes|no
- competitor_mentions: [comma-separated list or "none"]
- champion_present: yes|no|unknown
- champion_engaged: yes|no|n/a
END_SENTIMENT

Rules for sentiment:
- overall: The general tone of the entire meeting. "mixed" when positive and negative signals coexist.
- customer: The customer's tone specifically — ignore internal team sentiment. If no customer was present, use "n/a".
- engagement: How actively the customer participated. "high" = asking questions, making commitments. "disengaged" = short answers, camera off, multitasking cues.
- forward_looking: Did the conversation include future plans, next steps, roadmap discussion? "yes" if any substantive forward-looking language was used.
- competitor_mentions: List any competitor products or vendors mentioned by name. "none" if no competitors discussed.
- champion_present: Was the identified champion (primary internal advocate) in the meeting? "unknown" if champion identity is unclear.
- champion_engaged: If champion was present, were they actively contributing? "n/a" if champion not present or unknown.
```

#### INTERACTION_DYNAMICS section

Add an `INTERACTION_DYNAMICS` section to `build_transcript_prompt()`:

```
INTERACTION_DYNAMICS:
Analyze the conversation dynamics of this meeting. Base ONLY on evidence in the transcript.

TALK_BALANCE: <customer_pct>/<internal_pct> — Estimate speaking time split between customer participants and internal team. If transcript doesn't distinguish speakers clearly, write "unclear".

SPEAKER_SENTIMENT:
- <Speaker Name>: <positive|neutral|cautious|negative|mixed> — <one-line evidence quote or observation>
END_SPEAKER_SENTIMENT

ENGAGEMENT_SIGNALS:
- question_density: <high|moderate|low> — <who asked more questions, customer or internal?>
- decision_maker_active: <yes|no|unclear> — <was the most senior customer participant actively contributing?>
- forward_looking: <high|moderate|low> — <was the conversation focused on future plans or reviewing past issues?>
- monologue_risk: <yes|no> — <did any single speaker dominate for extended stretches without interaction?>
END_ENGAGEMENT_SIGNALS

COMPETITOR_MENTIONS:
- <Competitor Name>: <context — what was said, who said it, what it implies>
END_COMPETITOR_MENTIONS

ESCALATION_LANGUAGE:
- <quote or paraphrase that signals concern, frustration, or risk> — <speaker>
END_ESCALATION_LANGUAGE
```

### 2. Parse into structured output

Add parsing for the new sections in `processor/transcript.rs`, producing `TranscriptSentiment` and `InteractionDynamics` structs:

```rust
/// Structured sentiment from the SENTIMENT block (from I501)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSentiment {
    pub overall: Option<String>,         // positive|neutral|negative|mixed
    pub customer: Option<String>,        // positive|neutral|negative|mixed|n/a
    pub engagement: Option<String>,      // high|moderate|low|disengaged
    pub forward_looking: Option<bool>,
    pub competitor_mentions: Vec<String>, // empty vec if "none"
    pub champion_present: Option<String>, // yes|no|unknown
    pub champion_engaged: Option<String>, // yes|no|n/a
}

/// Interaction dynamics from the INTERACTION_DYNAMICS block
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InteractionDynamics {
    /// Estimated talk balance: customer % / internal %
    pub talk_balance: Option<String>,
    /// Per-speaker sentiment assessments
    pub speaker_sentiment: Vec<SpeakerSentiment>,
    /// Engagement quality signals
    pub engagement_signals: Option<EngagementSignals>,
    /// Competitor mentions with context
    pub competitor_mentions: Vec<CompetitorMention>,
    /// Escalation language detected
    pub escalation_signals: Vec<EscalationSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerSentiment {
    pub name: String,
    pub sentiment: String,  // positive|neutral|cautious|negative|mixed
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EngagementSignals {
    pub question_density: Option<String>,    // high|moderate|low
    pub decision_maker_active: Option<String>, // yes|no|unclear
    pub forward_looking: Option<String>,     // high|moderate|low
    pub monologue_risk: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitorMention {
    pub competitor: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationSignal {
    pub quote: String,
    pub speaker: Option<String>,
}
```

### 3. Store alongside meeting outcomes

`TranscriptSentiment` and `InteractionDynamics` are stored on the `meetings` table alongside existing outcome data (in `meeting_outcomes` JSON blob or new JSON columns). This data is then available to the enrichment prompt when it assembles entity intelligence.

**Sentiment storage (from I501):**
- A capture of type `"sentiment"` with serialized JSON is stored via `insert_capture()`, preserving per-meeting sentiment alongside wins/risks/decisions captures.
- A `transcript_sentiment` signal is emitted via `emit_signal()` at confidence 0.8 (higher than generic `transcript_outcomes`). This makes sentiment available to the health scoring engine (I499) through the signal bus.

### 4. Flow into enrichment context

The enrichment prompt (`intelligence/prompts.rs`) currently includes raw transcripts in the `recent_transcripts` field. With interaction dynamics extracted and stored, the enrichment prompt can reference structured dynamics data instead of (or alongside) raw text:

```
## Recent Meeting Dynamics
Meeting "Q3 Planning Review" (2026-02-15):
- Talk balance: 60% customer / 40% internal
- Champion sentiment: positive ("excited about the new features")
- Decision-maker: active (CFO contributed to pricing discussion)
- Competitor mentioned: Contentful ("evaluating for one division")
- Escalation: "We're concerned about the migration timeline" — VP Engineering
```

This is far more useful to the LLM than raw transcript text for producing `competitive_context`, per-stakeholder engagement levels in `stakeholder_insights`, and evidence-cited `risks`.

### 5. Signal emission

Key dynamics produce signals for the signal bus:

- **Competitor mention** → signal type `competitor_mentioned`, entity-linked, confidence 0.7. Feeds `competitive_context` on next enrichment.
- **Escalation language** → signal type `escalation_detected`, entity-linked, confidence 0.8. Feeds `risks`.
- **Decision-maker disengaged** → signal type `stakeholder_disengagement`, person-linked, confidence 0.6. Feeds `relationship_depth`.
- **Monologue risk** → no signal (informational, not actionable on its own).

Signals use `emit_signal()` (not `emit_signal_and_propagate()`) to avoid recursive enrichment on every transcript. The dynamics data flows into intelligence on the next scheduled enrichment cycle.

### 6. What this does NOT do

- Does NOT replace Gong. Gong does deal management, sales coaching, pipeline forecasting. DailyOS does account intelligence.
- Does NOT require speaker diarization. The LLM infers speakers from transcript structure (most transcript tools label speakers). If speaker labels are absent, dynamics that require per-speaker attribution return "unclear."
- Does NOT add a new AI call. The dynamics extraction happens in the existing `process_transcript()` call — it's an expansion of the same prompt, not a new pipeline.
- Does NOT change the existing meeting outcome fields (summary, discussion, actions, wins, risks, decisions). Those are preserved exactly as they are. Interaction dynamics is additive.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/processor/transcript.rs` | Extend `build_transcript_prompt()` with SENTIMENT + INTERACTION_DYNAMICS sections. Add parsing for both blocks. Add `TranscriptSentiment`, `InteractionDynamics` structs and sub-types. Add `sentiment` and `interaction_dynamics` fields to `TranscriptResult`. Store sentiment as capture + emit `transcript_sentiment` signal. |
| `src-tauri/src/processor/enrich.rs` | Add `parse_sentiment_block()` function. Add `sentiment: Option<TranscriptSentiment>` to `EnrichmentParsed` struct returned by `parse_enrichment_response()`. Validate enum values. |
| `src-tauri/src/types.rs` | Add `TranscriptSentiment` and `InteractionDynamics` to `TranscriptResult` or meeting outcome types. |
| `src-tauri/src/db/meetings.rs` | Store sentiment + interaction dynamics alongside meeting outcomes. |
| `src-tauri/src/intelligence/prompts.rs` | Include structured dynamics + sentiment in enrichment context (replace or augment raw transcript inclusion). |
| `src-tauri/src/intelligence/io.rs` | `TranscriptSentiment` type defined here via I503. No changes if I503 lands first. |
| `src-tauri/src/signals/bus.rs` | Add signal type definitions for `transcript_sentiment`, `competitor_mentioned`, `escalation_detected`, `stakeholder_disengagement`. |
| `src/types/index.ts` | Add matching TypeScript types for both structs. |

## Acceptance Criteria

### Sentiment (from I501)
1. `build_transcript_prompt()` output includes the `SENTIMENT:` section with rules for each field.
2. Given a transcript AI response with a valid SENTIMENT block, `parse_sentiment_block()` returns a populated `TranscriptSentiment` with all 7 fields.
3. Given a response missing the SENTIMENT block entirely, `parse_sentiment_block()` returns `None` (graceful degradation for transcripts processed before this change).
4. Invalid enum values are rejected: `"angry"` for overall sentiment returns `None` for that field, not a crash.
5. A capture of type `"sentiment"` is stored in the DB for each transcript with valid sentiment data.
6. A `transcript_sentiment` signal is emitted via the signal bus with confidence 0.8.
7. `competitor_mentions: "Salesforce, HubSpot"` parses to `vec!["Salesforce", "HubSpot"]`; `"none"` parses to empty vec.

### Interaction Dynamics
8. Process a transcript with clear speaker labels. `InteractionDynamics` populated with talk balance, per-speaker sentiment, engagement signals.
9. Transcript mentioning a competitor → `competitor_mentions` populated with competitor name and context. Signal emitted.
10. Transcript with escalation language ("concerned about," "reconsidering") → `escalation_signals` populated. Signal emitted.
11. Transcript where decision-maker is silent → `decision_maker_active: "no"` in engagement signals.
12. On next enrichment cycle after transcript processing, `competitive_context` in intelligence.json includes the competitor mention (from dynamics data, not re-reading the raw transcript).
13. Transcript without clear speaker labels → dynamics that require attribution return "unclear" rather than hallucinating.

### Shared
14. Existing transcript extraction (summary, discussion, actions, wins, risks, decisions) is unchanged — no regression.
15. No new AI call — sentiment + dynamics extracted in the same `process_transcript()` prompt.
16. New unit tests cover: valid sentiment parsing, missing block, partial block, invalid enum values, competitor list parsing, dynamics parsing.

## Relationship to Other Issues

- **Absorbs I501** — transcript sentiment extraction (7-field SENTIMENT block, capture storage, signal emission) is merged here
- **Depends on I508a** — `competitive_context` and engagement-aware `stakeholder_insights` need schema fields from I508a
- **Depends on I503** — `TranscriptSentiment` type defined there
- **Enhances I499** — health engine's signal momentum dimension benefits from richer per-meeting signals (transcript_sentiment, competitor mentions, escalation signals)
- **Enhances I504** — AI relationship inference can use per-speaker sentiment to assess relationship quality, not just co-occurrence
- **Enhances I490** — Renewal Readiness report can reference champion engagement, competitor mentions, escalation signals from dynamics data
- **Independent of Glean** — works entirely with local transcript data. Glean transcripts (Gong via Glean) get the same extraction.

## Out of Scope

- Speaker diarization (relies on transcript tool's speaker labels)
- Audio-level analysis (pitch, pace, interruption patterns — would require audio access, not just transcript)
- Sales coaching features (Gong's lane — talk-to-listen coaching, question technique, closing language)
- Historical reprocessing of existing transcripts (new extractions apply to newly processed transcripts only; backfill is a separate decision)
- Custom competitor trackers (configurable list of competitors to watch for — future enhancement, v1.2+)
