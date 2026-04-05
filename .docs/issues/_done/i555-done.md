# I555 — Captures Metadata Enrichment + Interaction Dynamics Persistence

**Version:** v1.0.0 Phase 4
**Depends on:** I554 (transcript prompt fidelity — produces the metadata to store)
**Type:** Enhancement — schema + parser + signal bus integration + health scoring upgrade
**Scope:** Backend: migration, DB module, transcript parser, signal emissions, health scoring, intel context, meeting prep context. No frontend changes.

---

## Problem

Two data loss issues in the transcript pipeline:

### 1. Captures lose metadata at storage time

The transcript prompt (especially after I554) extracts rich metadata per item — urgency levels on risks, sub-types on wins, impact classifications, verbatim quotes. But `persist_transcript_outcomes()` stores everything into the `captures` table as flat `content` strings. The `captures` table has no columns for urgency, sub-type, impact, or evidence quotes.

This means:
- A RED risk (champion departing) looks identical to a GREEN_WATCH (vague dissatisfaction) in the DB
- An EXPANSION win is indistinguishable from an ADVOCACY win
- Verbatim customer quotes are embedded in content text with no way to query them separately
- Reports and surfaces that consume captures can't filter or prioritize by urgency/type

### 2. Interaction dynamics extracted but discarded

The transcript prompt asks for `INTERACTION_DYNAMICS` — talk balance percentages, per-speaker sentiment with evidence, engagement signals (question density, decision-maker activity, forward-looking orientation, monologue risk), competitor mentions with context, and escalation language quotes. The parser (`parse_transcript_output()`) doesn't capture any of this. It's wasted LLM output.

This data is valuable for:
- Health scoring dimensions (engagement cadence, champion health)
- Account detail surfaces (who's talking, who's engaged)
- Meeting detail post-meeting intelligence
- Reports that need engagement quality metrics

---

## Solution

### 1. Migration: Enrich `captures` table

```sql
ALTER TABLE captures ADD COLUMN sub_type TEXT;       -- e.g., 'adoption', 'expansion', 'red', 'yellow'
ALTER TABLE captures ADD COLUMN urgency TEXT;         -- 'red', 'yellow', 'green_watch' (risks only)
ALTER TABLE captures ADD COLUMN impact TEXT;          -- 'revenue', 'cost', 'risk', 'speed' (wins/value)
ALTER TABLE captures ADD COLUMN evidence_quote TEXT;  -- verbatim customer quote
ALTER TABLE captures ADD COLUMN speaker TEXT;         -- who said it (name if identified)
```

### 2. New table: `meeting_interaction_dynamics`

```sql
CREATE TABLE IF NOT EXISTS meeting_interaction_dynamics (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    talk_balance_customer_pct INTEGER,      -- 0-100, customer talk percentage
    talk_balance_internal_pct INTEGER,      -- 0-100, internal talk percentage
    speaker_sentiments_json TEXT,           -- JSON array: [{name, sentiment, evidence}]
    question_density TEXT,                  -- 'high', 'moderate', 'low'
    decision_maker_active TEXT,             -- 'yes', 'no', 'unclear'
    forward_looking TEXT,                   -- 'high', 'moderate', 'low'
    monologue_risk INTEGER DEFAULT 0,       -- boolean
    competitor_mentions_json TEXT,          -- JSON array: [{competitor, context}]
    escalation_language_json TEXT,          -- JSON array: [{quote, speaker}]
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 3. New table: `meeting_champion_health`

```sql
CREATE TABLE IF NOT EXISTS meeting_champion_health (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    champion_name TEXT,
    champion_status TEXT NOT NULL,          -- 'strong', 'weak', 'lost', 'none'
    champion_evidence TEXT,
    champion_risk TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 4. New table: `captured_commitments` (absorbed from I551)

```sql
CREATE TABLE captured_commitments (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    owned_by TEXT CHECK(owned_by IN ('us', 'them', 'joint')),
    success_criteria TEXT,
    target_date TEXT,
    source_type TEXT NOT NULL,          -- 'transcript', 'ai-inbox', 'enrichment'
    source_id TEXT,                     -- meeting_id or file path
    source_label TEXT,                  -- Human-readable source (meeting title, filename)
    consumed INTEGER DEFAULT 0,         -- 1 = already used to create/suggest an objective
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_commitments_account ON captured_commitments(account_id);
```

This is a staging table for I551's objective suggestion pipeline. Raw commitments captured from transcripts feed into `get_objective_suggestions()`.

### 5. Schema addition: `entity_assessment.success_plan_signals_json` (absorbed from I551)

```sql
ALTER TABLE entity_assessment ADD COLUMN success_plan_signals_json TEXT;
```

Stores the synthesized `SuccessPlanSignals` output from entity intelligence enrichment (I554 adds the prompt schema; this column stores the result).

### 6. New table: `meeting_role_changes`

```sql
CREATE TABLE IF NOT EXISTS meeting_role_changes (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    person_name TEXT NOT NULL,
    old_status TEXT,
    new_status TEXT,
    evidence_quote TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 7. Parser updates

Update `parse_transcript_output()` in `processor/transcript.rs` to:

1. **Wins**: Parse `[SUB_TYPE]` prefix and `#"quote"` suffix. Pass `sub_type`, `impact`, `evidence_quote` to `persist_transcript_outcomes()`.
2. **Risks**: Parse `[RED|YELLOW|GREEN_WATCH]` prefix and `#"quote"` suffix. Pass `urgency`, `sub_type`, `evidence_quote`.
3. **Decisions**: Parse `[COMMITMENT_TYPE]` prefix, `@owner`, `#"quote"` suffix.
4. **Interaction dynamics**: Parse the full `INTERACTION_DYNAMICS` block into a struct. Write to `meeting_interaction_dynamics`.
5. **Champion health**: Parse `CHAMPION_HEALTH` block. Write to `meeting_champion_health`.
6. **Role changes**: Parse `ROLE_CHANGES` block. Write to `meeting_role_changes`.
7. **Commitments**: Parse `COMMITMENTS` block. Write to `captured_commitments` with `source_type='transcript'`, `source_id=meeting_id`.

### 8. Rust types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptCapture {
    pub capture_type: String,       // win, risk, decision
    pub content: String,
    pub sub_type: Option<String>,   // adoption, expansion, red, yellow, etc.
    pub urgency: Option<String>,    // red, yellow, green_watch
    pub impact: Option<String>,     // revenue, cost, risk, speed
    pub evidence_quote: Option<String>,
    pub speaker: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionDynamics {
    pub talk_balance_customer_pct: Option<i32>,
    pub talk_balance_internal_pct: Option<i32>,
    pub speaker_sentiments: Vec<SpeakerSentiment>,
    pub question_density: Option<String>,
    pub decision_maker_active: Option<String>,
    pub forward_looking: Option<String>,
    pub monologue_risk: bool,
    pub competitor_mentions: Vec<CompetitorMention>,
    pub escalation_language: Vec<EscalationQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSentiment {
    pub name: String,
    pub sentiment: String,   // positive, neutral, cautious, negative, mixed
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorMention {
    pub competitor: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationQuote {
    pub quote: String,
    pub speaker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionHealthAssessment {
    pub champion_name: Option<String>,
    pub champion_status: String,  // strong, weak, lost, none
    pub champion_evidence: Option<String>,
    pub champion_risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleChange {
    pub person_name: String,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub evidence_quote: Option<String>,
}
```

---

## Files

| File | Changes |
|------|---------|
| `src-tauri/src/migrations/0XX_captures_metadata_and_dynamics.sql` | New migration: ALTER captures + CREATE meeting_interaction_dynamics + meeting_champion_health + meeting_role_changes + captured_commitments + ALTER entity_assessment (success_plan_signals_json) |
| `src-tauri/src/migrations.rs` | Register new migration |
| `src-tauri/src/db/meetings.rs` | Add CRUD for interaction_dynamics, champion_health, role_changes tables |
| `src-tauri/src/db/types.rs` | Add `InteractionDynamics`, `ChampionHealthAssessment`, `RoleChange` structs |
| `src-tauri/src/processor/transcript.rs` | Update parser to extract metadata from new prompt format. Update `persist_transcript_outcomes()` to write enriched captures + new tables. |

---

## Architecture Integration

This is NOT a display-only persistence issue. The new tables must be first-class participants in the existing signal bus, health scoring, intel context, meeting prep, and callout systems. The architecture already has integration points — most just need signal emissions or query additions.

### A. Signal Bus Integration

All signal emissions go through ServiceLayer per I512 pattern. Post-transcript processing in `services/meetings.rs` already emits 5 signal types — this adds 4 more in the same block.

**1. Champion health → person-level signal → existing propagation chain**

When `meeting_champion_health.champion_status` is `weak` or `lost`, resolve the champion's `person_id` from `account_stakeholders` (role = 'champion') and emit:

```rust
// Emit on the PERSON entity (not account) — this triggers existing rule_champion_sentiment
emit_signal_and_propagate(db, &engine, "person", &champion_person_id, SignalEvent {
    signal_type: "negative_sentiment",
    source: "transcript",
    value: json!({"champion_status": status, "evidence": evidence}).to_string(),
    confidence: match status { "lost" => 0.9, "weak" => 0.7, _ => 0.0 },
})
```

This wires into the **existing** `rule_champion_sentiment` propagation rule (propagation.rs:152), which derives `champion_risk` on linked accounts where the person's role is "champion". The `champion_risk` signal is already in `CALLOUT_SIGNAL_TYPES` (callouts.rs:42), so it surfaces in the morning briefing automatically. **Zero new rules or callout handlers needed.**

When status is `strong`, emit a positive signal to balance the Bayesian weights:
```rust
emit_signal(db, "person", &champion_person_id, SignalEvent {
    signal_type: "champion_engagement_confirmed",
    source: "transcript",
    confidence: 0.8,
})
```

**2. Meeting frequency → reactivate dead rule**

`rule_meeting_frequency_drop` is fully implemented in `rules.rs` but **unregistered** in `default_engine()` (propagation.rs:145 — commented out because "no code emits `meeting_frequency` signals"). After processing a transcript, emit:

```rust
emit_signal_and_propagate(db, &engine, "account", &account_id, SignalEvent {
    signal_type: "meeting_frequency",
    source: "transcript",
    value: json!({"meeting_count_30d": count}).to_string(),
    confidence: 0.9,
})
```

Then **register** `rule_meeting_frequency_drop` in `default_engine()`. This derives `engagement_warning` on accounts with declining meeting cadence, which is already in `CALLOUT_SIGNAL_TYPES`.

**3. Urgency-tiered risk signals**

RED risks need to trigger faster response. Emit at graduated confidence:

```rust
// In persist_transcript_outcomes(), per risk capture:
let confidence = match urgency.as_deref() {
    Some("red") => 0.9,
    Some("yellow") => 0.6,
    Some("green_watch") => 0.3,
    _ => 0.5,
};
emit_signal_and_propagate(db, &engine, "account", &account_id, SignalEvent {
    signal_type: "risk_detected",
    source: "transcript",
    value: json!({"urgency": urgency, "sub_type": sub_type, "content": content}).to_string(),
    confidence,
})
```

RED risks at confidence 0.9 → callout severity "critical" (callouts.rs:177). YELLOW at 0.6 → "info". GREEN_WATCH at 0.3 → below callout threshold (won't surface in briefing but still feeds `signalMomentum`).

**4. Role changes → stakeholder_change signal**

```rust
emit_signal_and_propagate(db, &engine, "account", &account_id, SignalEvent {
    signal_type: "stakeholder_change",
    source: "transcript",
    value: json!({"person": name, "old": old_status, "new": new_status}).to_string(),
    confidence: 0.8,
})
```

`stakeholder_change` is already in `CALLOUT_SIGNAL_TYPES` and triggers `rule_person_job_change` in the propagation chain, which derives signals on linked accounts and invalidates meeting prep.

**5. Commitments as dual-write (captures + captured_commitments)**

Write commitments to BOTH:
- `captured_commitments` table (structured, for I551's objective suggestion pipeline)
- `captures` table with `capture_type = "commitment"` (flat, for automatic flow into intel prompts and meeting prep via existing `get_captures_for_account()` queries)

This dual-write means commitments appear in entity intelligence context and meeting prep briefings with **zero additional query code** — the existing `recent_captures` assembly in `build_intelligence_context()` and `gather_account_context()` already pulls all capture types.

### B. Health Scoring Upgrade

Three dimension functions in `intelligence/health_scoring.rs` get surgical upgrades:

**1. `compute_champion_health()` — behavioral, not structural**

Current: checks if champion role exists (+60), if any meetings in 30d (+20), if any email signals (+20). This is a structural check that tells you nothing about actual champion engagement.

Upgrade:
```rust
// Replace the generic "any meetings" check with per-champion engagement data
let champion_meetings = db.query(
    "SELECT mch.champion_status, mch.champion_evidence, m.start_time
     FROM meeting_champion_health mch
     JOIN meetings m ON m.id = mch.meeting_id
     JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ?
     WHERE mch.champion_name IS NOT NULL
     ORDER BY m.start_time DESC LIMIT 5",
    [account_id]
);

// Score based on recent champion engagement trend
// strong in last meeting = 90, weak = 40, lost = 10, none = 20
// Trend: improving (weak→strong) = +10, declining (strong→weak) = -20
```

Evidence becomes specific: "Sarah Chen attended 4 of last 5 meetings, champion status: strong in 3, weak in 1" instead of "Active in recent meetings."

**2. `compute_stakeholder_coverage()` — attendance-verified**

Current: counts whether champion/executive/technical roles exist in `account_stakeholders`. A role filled by someone who hasn't attended a meeting in 6 months scores the same as an active attendee.

Upgrade: cross-reference `account_stakeholders` with `meeting_attendees` to verify recent attendance:
```rust
let verified_coverage = db.query(
    "SELECT ast.role, MAX(m.start_time) as last_seen
     FROM account_stakeholders ast
     JOIN meeting_attendees ma ON ma.person_id = ast.person_id
     JOIN meetings m ON m.id = ma.meeting_id
     JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ast.account_id
     WHERE ast.account_id = ?
     GROUP BY ast.person_id, ast.role",
    [account_id]
);
// Roles with last_seen > 90d ago count at 50% weight
// Roles with last_seen > 180d ago count at 0%
```

**3. `compute_meeting_cadence()` — quality modifier**

Current: raw meeting count + recency bonus.

Upgrade: add a quality multiplier from `meeting_interaction_dynamics`:
```rust
let dynamics = db.query(
    "SELECT mid.question_density, mid.decision_maker_active, mid.forward_looking
     FROM meeting_interaction_dynamics mid
     JOIN meeting_entities me ON me.meeting_id = mid.meeting_id AND me.entity_id = ?
     ORDER BY mid.created_at DESC LIMIT 3",
    [account_id]
);
// High question density + decision maker active + forward looking = quality multiplier 1.2
// Low question density + decision maker inactive + not forward looking = quality multiplier 0.7
```

### C. Intel Queue Context Enrichment

Add new context blocks to `build_intelligence_context()` in `intelligence/prompts.rs`:

**1. Recent interaction dynamics** (last 5 meetings with this entity):

```
## Meeting Engagement Patterns (last 5 meetings)
- 2026-03-10 | Acme Weekly Sync | Talk: 45% customer / 55% internal | Champion: present, engaged | Forward-looking: high
- 2026-03-03 | Acme QBR Prep | Talk: 30% customer / 70% internal | Champion: not present | Forward-looking: low
```

This gives the LLM behavioral patterns that narrative summaries often omit.

**2. Champion health trend** (last 5 assessments):

```
## Champion Health Trend
Sarah Chen — strong (3/10), strong (3/3), weak (2/24), strong (2/17), strong (2/10)
Trend: stable-strong with one dip on 2/24 (evidence: "delegated to junior, didn't speak")
```

**3. Unfulfilled commitments**:

```
## Open Commitments (from prior meetings)
- "Deliver migration plan by end of February" — owned_by: us, from Acme Weekly 2/17, target: 2026-02-28 [OVERDUE]
- "Share Q1 usage report" — owned_by: them, from Acme QBR 3/3, target: 2026-03-15
```

### D. Meeting Prep Context Enrichment

Add to `gather_account_context()` in `prepare/meeting_context.rs`:

**1. Prior meeting dynamics** — what were the dynamics last time?

```
## Last Meeting Dynamics (Acme Weekly Sync, March 10)
Talk balance: 45% customer / 55% internal
Champion present: yes, engaged
Decision maker active: yes
Escalation language: none
```

This is exactly what a chief-of-staff would brief you on: "Last time, the champion was engaged and the decision maker was active — expect a similar setup today."

**2. Open commitments from prior meetings**:

```
## Open Commitments to Address
- We committed to delivering migration plan by Feb 28 (OVERDUE)
- They committed to sharing Q1 usage report by Mar 15
```

This flows into the proposed agenda as follow-up items.

### E. Callout Integration (no code changes needed)

The signal emissions in Section A automatically activate existing callout handlers:

| Signal | Callout Type | Severity | Briefing Text |
|---|---|---|---|
| `champion_risk` (derived from `negative_sentiment` on person) | `champion_risk` | Warning (conf 0.80) | Already has handler in `build_callout_text()` |
| `engagement_warning` (derived from `meeting_frequency` via reactivated rule) | `engagement_warning` | Info/Warning | Already has handler |
| `stakeholder_change` (from role changes) | `stakeholder_change` | Info (conf 0.8) | Already has handler |
| `risk_detected` RED | — | Critical (conf 0.9) | Needs new handler in `build_callout_text()` — add case for `risk_detected` |

Only `risk_detected` needs a new callout text handler and addition to `CALLOUT_SIGNAL_TYPES`. All others wire through existing infrastructure.

---

## Files

Updated to reflect full architecture integration:

| File | Changes |
|------|---------|
| `src-tauri/src/migrations/0XX_captures_metadata_and_dynamics.sql` | New migration: ALTER captures + CREATE meeting_interaction_dynamics + meeting_champion_health + meeting_role_changes + captured_commitments + ALTER entity_assessment (success_plan_signals_json) |
| `src-tauri/src/migrations.rs` | Register new migration |
| `src-tauri/src/db/meetings.rs` | Add CRUD for interaction_dynamics, champion_health, role_changes tables. Add queries for dynamics-by-entity (last N), champion-health-trend, commitments-by-account. |
| `src-tauri/src/db/types.rs` | Add `InteractionDynamics`, `ChampionHealthAssessment`, `RoleChange` structs |
| `src-tauri/src/processor/transcript.rs` | Update parser to extract metadata from new prompt format. Update `persist_transcript_outcomes()` to write enriched captures + new tables. |
| `src-tauri/src/services/meetings.rs` | Add signal emissions in post-transcript processing block: `negative_sentiment` on champion person, `meeting_frequency`, urgency-tiered `risk_detected`, `stakeholder_change`, `champion_engagement_confirmed` |
| `src-tauri/src/signals/propagation.rs` | Register `rule_meeting_frequency_drop` in `default_engine()` (already implemented in rules.rs) |
| `src-tauri/src/signals/callouts.rs` | Add `risk_detected` to `CALLOUT_SIGNAL_TYPES`. Add callout text handler for `risk_detected`. |
| `src-tauri/src/intelligence/health_scoring.rs` | Upgrade `compute_champion_health()` with per-champion meeting engagement data. Upgrade `compute_stakeholder_coverage()` with attendance verification. Add quality modifier to `compute_meeting_cadence()`. |
| `src-tauri/src/intelligence/prompts.rs` | Add interaction dynamics, champion health trend, and open commitments as new context blocks in `build_intelligence_context()`. |
| `src-tauri/src/prepare/meeting_context.rs` | Add prior meeting dynamics and open commitments to `gather_account_context()`. |

---

## Out of Scope

- Frontend rendering of new data (I557, I558)
- Prompt changes (I554 — this issue consumes I554's output)
- Report pipeline changes (I556)
- New propagation rules beyond reactivating the existing `rule_meeting_frequency_drop`

---

## Acceptance Criteria

### Schema + Persistence
1. `captures` table has `sub_type`, `urgency`, `impact`, `evidence_quote`, `speaker` columns after migration.
2. `meeting_interaction_dynamics` table exists with talk balance, speaker sentiments, engagement signals, competitor mentions, escalation language.
3. `meeting_champion_health` table exists with champion_name, status, evidence, risk.
4. `meeting_role_changes` table exists with person_name, old/new status, evidence.
5. `captured_commitments` table exists. Processing a transcript with commitments stores them in BOTH `captured_commitments` (structured) AND `captures` with `capture_type='commitment'` (for automatic flow into intel context and meeting prep).
6. `entity_assessment.success_plan_signals_json` column exists (nullable, populated on next enrichment cycle after I554 lands).
7. Processing a transcript with a RED risk stores `urgency='red'` in captures. A GREEN_WATCH stores `urgency='green_watch'`.
8. Processing a transcript with an EXPANSION win stores `sub_type='expansion'` in captures.
9. Verbatim quotes from transcript extraction stored in `evidence_quote` column, not embedded in `content`.
10. Interaction dynamics, champion health, and role changes persisted per meeting after transcript processing.
11. Existing captures without metadata remain valid (all new columns are nullable).

### Signal Bus Integration
12. Champion health `weak` or `lost` emits `negative_sentiment` signal on the champion's **person_id** entity (not the account) via `emit_signal_and_propagate()`. This triggers the existing `rule_champion_sentiment` → `champion_risk` propagation chain.
13. Champion health `strong` emits `champion_engagement_confirmed` signal on the champion's person_id (positive reinforcement for Bayesian weights).
14. `rule_meeting_frequency_drop` registered in `default_engine()`. `meeting_frequency` signal emitted after transcript processing. Accounts with declining meeting cadence get `engagement_warning` derived signal.
15. RED risks emit `risk_detected` signal at confidence 0.9 → callout severity "critical". YELLOW at 0.6. GREEN_WATCH at 0.3 (below callout threshold but feeds signalMomentum).
16. `risk_detected` added to `CALLOUT_SIGNAL_TYPES` with callout text handler.
17. Role changes emit `stakeholder_change` signal on account via `emit_signal_and_propagate()` — triggers existing `rule_person_job_change` chain and prep invalidation.
18. Morning briefing callouts surface `champion_risk`, `engagement_warning`, `stakeholder_change`, and critical `risk_detected` items — verified with real transcript processing, not mock signals.

### Health Scoring Upgrade
19. `compute_champion_health()` queries `meeting_champion_health` for the champion's person_id. Evidence includes specific meeting dates, champion status per meeting, and trend direction. Score reflects behavioral engagement, not just role existence.
20. `compute_stakeholder_coverage()` cross-references `account_stakeholders` roles with `meeting_attendees` to verify recent attendance. A role filled by someone not seen in 90+ days counts at 50% weight; 180+ days at 0%.
21. `compute_meeting_cadence()` applies quality modifier from `meeting_interaction_dynamics` — high engagement quality multiplies the cadence score; low quality discounts it.

### Intel Context + Meeting Prep
22. `build_intelligence_context()` includes "Meeting Engagement Patterns" block with last 5 meetings' interaction dynamics for the entity (talk balance, champion presence, forward-looking orientation).
23. `build_intelligence_context()` includes "Champion Health Trend" block with per-champion status across recent meetings.
24. `build_intelligence_context()` includes "Open Commitments" block from `captured_commitments` WHERE `consumed = 0` (shows unfulfilled commitments for the LLM to reference in assessments).
25. Commitments written with `capture_type='commitment'` to `captures` table automatically appear in existing `recent_captures` context blocks in both intel prompts and meeting prep — verified with zero new query code in those paths.
26. `gather_account_context()` in meeting prep includes prior meeting dynamics (last meeting's talk balance, champion presence, engagement quality) and open commitments.

### Mock Data
27. Mock data (`full` scenario) seeds enriched captures with sub_type, urgency, impact, and evidence_quote values. At least: 2 RED risks, 2 YELLOW risks, 1 GREEN_WATCH, 3 sub-typed wins (ADOPTION, EXPANSION, VALUE_REALIZED), and 2 commitments.
28. Mock data seeds `meeting_interaction_dynamics` for at least 3 meetings (varying talk balance and engagement quality).
29. Mock data seeds `meeting_champion_health` for at least 2 meetings (one strong, one weak).
30. Mock data seeds `captured_commitments` with at least 3 entries (mix of us/them/joint ownership, 1 consumed + 2 unconsumed).
31. After applying `full` mock scenario, all new surfaces (I557 account detail chapters, I558 meeting post-intelligence) render correctly with mock data — no console errors, no blank sections.

### General
32. `cargo test` passes. `cargo clippy -- -D warnings` clean. Migration is forward-only with safe nullable defaults.
