# ADR-0100 — Glean-First Intelligence Architecture

**Status:** Accepted
**Date:** 2026-03-14
**Supersedes:** Portions of ADR-0095 (dual-mode context — updates the "dual" to "Glean-primary")
**Context:** I559 validation spike confirmed Glean MCP `chat` tool produces structured JSON matching I508 schema with real cross-source data (Zendesk, Salesforce, Gong, Slack, org chart) in 10-30 seconds.

---

## Decision

When Glean is connected, it becomes the **primary intelligence computation engine**. Claude Code PTY calls become the **fallback for users without Glean**. This inverts the original dual-mode design (ADR-0095) where local was primary and Glean was supplementary.

The `chat` MCP tool replaces purpose-built Glean Agents (REST API not available on our instance) and can produce complete `IntelligenceJson`-compatible output in a single call, with data from sources DailyOS has zero local visibility into.

---

## Context

### What changed

ADR-0095 designed Glean as an additive context source — search results appended to local context, with the PTY call doing all synthesis. The I559 validation spike (2026-03-14) discovered:

1. The Glean MCP `chat` tool does multi-step reasoning, cross-source synthesis, and returns structured JSON on request
2. It accesses Salesforce (CRM), Zendesk (support), Gong (call recordings), Slack, P2, and org directories simultaneously
3. It can discover a user's accounts from their email identity alone — no manual entry needed
4. It produces output that maps directly to our I508 intelligence schema
5. Response time (10-30s) is faster than PTY calls (60-180s)
6. It's included in the enterprise Glean subscription — no per-token cost

### What this means

| Capability | Before (ADR-0095) | After (ADR-0100) |
|---|---|---|
| Entity enrichment | PTY call primary, Glean search supplements context | Glean `chat` primary, PTY fallback for non-Glean users |
| Health scoring inputs | 6 dimensions from local data only | Local dimensions + Glean-sourced support health, CRM data, engagement patterns |
| Account discovery | Manual entry during onboarding | Automatic from Glean (Salesforce + Gong + Zendesk cross-reference) |
| People enrichment | Clay + Gravatar | Glean org chart + Salesforce contacts + Gong participants |
| Competitive intel | From meeting transcripts only | From Gong + Slack + internal docs + CRM win/loss data |
| Onboarding | Requires Claude Code subscription + manual setup | Glean auth → auto-populated book of business |

---

## Architecture

### Intelligence Provider Hierarchy

```
User connects Glean?
  ├── YES → GleanIntelligenceProvider (primary)
  │         ├── Entity enrichment: chat MCP tool with I508 schema prompt
  │         ├── Account discovery: chat MCP tool with user email
  │         ├── People enrichment: chat MCP tool + search (people:)
  │         ├── Gap filling: chat MCP tool with dimension-specific prompts
  │         └── Fallback: LocalIntelligenceProvider if Glean call fails
  │
  └── NO → LocalIntelligenceProvider (standalone)
            ├── Entity enrichment: Claude Code PTY call
            ├── Account discovery: manual entry
            ├── People enrichment: Clay + Gravatar
            └── No gap filling beyond local context
```

### Signal Source Reliability Tiers

All Glean-sourced data enters the Intelligence Loop as signals with source-specific confidence levels:

| Source | Confidence | Rationale | Health Scoring Impact |
|---|---|---|---|
| Salesforce (CRM) | 0.9 | System of record. Renewal dates, ARR, deal stage, ownership. | Direct input to `financialProximity`. Renewal probability feeds `signalMomentum`. |
| Zendesk (Support) | 0.85 | Ticket data is factual. Severity, count, SLA compliance. | Direct input to new `supportHealth` sub-dimension. Ticket velocity feeds `signalMomentum`. |
| Gong (Calls) | 0.8 | Recorded calls are factual. AI summaries of calls are synthesized. | Supplements `meetingCadence` (catches calls not on calendar). Call sentiment feeds `championHealth`. |
| Glean AI synthesis | 0.7 | AI reasoning over source data. Can hallucinate. Same tier as PTY output. | Feeds narrative fields (executiveAssessment, risks, currentState). Does NOT directly set numeric scores. |
| Slack / P2 | 0.5 | Internal conversations. Valuable context but noisy. | Informs risk detection and `currentState.unknowns`. Does NOT adjust dimension scores directly. |
| Local PTY (Claude Code) | 0.7 | AI reasoning over local context. Same tier as Glean AI synthesis. | Same as today — feeds all narrative fields. |
| User input | 1.0 | User corrections are ground truth. | Overrides everything. Bayesian weights adjust all sources. |

### How Slack/Internal Conversations Are Treated

Slack and P2 mentions are **context signals, not health signals**. They inform the LLM narrative but do not directly adjust numeric health dimension scores.

**Why:** A Slack thread about an account could mean anything — an FYI, a celebration, a crisis, or idle speculation. Unlike a Zendesk ticket (which represents a real customer interaction) or a Salesforce field (which represents a committed data point), Slack is ambient internal awareness.

**How they flow:**
1. Glean's `chat` tool synthesizes Slack mentions into its narrative output
2. The narrative appears in `executiveAssessment`, `currentState`, or `risks`
3. These narrative fields are rendered on account detail and meeting prep
4. They do NOT feed into `compute_meeting_cadence()`, `compute_champion_health()`, etc.
5. Exception: Slack escalation channels — if Glean detects a thread in a known escalation channel, it surfaces as a risk at confidence 0.6

**The principle:** Slack tells you what your team knows. Zendesk tells you what the customer experienced. Gong tells you what was said. Salesforce tells you what's committed. Each has a different reliability weight.

### Health Scoring Integration

The 6 existing algorithmic dimensions gain Glean-sourced inputs:

```
compute_meeting_cadence()
  + Gong call count for this account (catches calls not on user's calendar)
  + Zoom meeting frequency from Glean search

compute_email_engagement()
  + (unchanged — Gmail is personal, Glean doesn't add here)

compute_stakeholder_coverage()
  + Salesforce contact list for the account (verifies coverage against actual org)
  + Glean org chart data (identifies roles we should be covering)

compute_champion_health()
  + Gong call engagement for champion (sentiment, talk ratio from call recordings)
  + Zendesk ticket activity from champion (are they escalating?)

compute_financial_proximity()
  + Salesforce renewal stage, probability %, deal value
  + Pipeline data (expansion opportunities in CRM)

compute_signal_momentum()
  + Zendesk ticket volume trends (support escalation velocity)
  + Gong call frequency changes (engagement acceleration/deceleration)
```

Each Glean input is stored as a signal in `signal_events` with `source = "glean"` and the appropriate confidence tier. The Bayesian feedback system (I529/I530) tracks Glean source reliability separately.

### Onboarding Flow (Glean-Connected)

```
1. User launches DailyOS for the first time
2. Settings → Connect Glean (OAuth with mcp + search + chat + people scopes)
3. Background: chat("Find all customer accounts for {email}") → account list
4. User sees: "We found 12 accounts. Review and confirm."
   - Checkboxes to include/exclude
   - Role auto-detected (TAM, CSM, owner)
5. User confirms → entities created in DB
6. Background: per-account chat() calls → full intelligence per account
   - Health scores, risks, stakeholders, competitive context, support health
   - All written to entity_assessment, signals emitted to signal bus
7. User opens app → fully populated book of business with intelligence
   - No Claude Code needed
   - No manual entry
   - Minutes, not hours
```

### Enrichment Pipeline (Glean-Connected)

```
Enrichment trigger (scheduler, signal, manual)
  │
  ├── Glean connected?
  │     ├── YES: GleanIntelligenceProvider
  │     │     1. Build structured prompt with I508 schema + entity context
  │     │     2. Call chat MCP tool (timeout: 60s)
  │     │     3. Parse JSON response into IntelligenceJson
  │     │     4. Merge with local data (calendar, transcripts, user edits)
  │     │     5. Write to entity_assessment
  │     │     6. Emit signals (source="glean", confidence per tier)
  │     │     7. Invalidate meeting prep
  │     │     8. On failure: fall back to LocalIntelligenceProvider
  │     │
  │     └── NO: LocalIntelligenceProvider
  │           1. Build context from local DB (meetings, emails, captures)
  │           2. PTY call to Claude Code (timeout: 180s)
  │           3. Parse response into IntelligenceJson
  │           4. Write to entity_assessment
  │           5. Emit signals (source="local_enrichment", confidence 0.7)
  │
  └── Health scoring runs after either path
        - Algorithmic dimensions computed from signals + DB
        - Glean-sourced signals included at their confidence tier
        - Result written to health_json
```

---

## What This Does NOT Change

1. **DailyOS stays local-first.** SQLite is the canonical data store. All intelligence is persisted locally. Glean is a computation source, not a storage backend.
2. **User corrections are ground truth.** A user edit overrides both Glean and PTY output. Bayesian weights penalize the source that was wrong.
3. **The Intelligence Loop is unchanged.** Signals → propagation → health scoring → intel context → enrichment → prep → callouts → feedback. Glean is a new input to this loop, not a replacement for it.
4. **Transcript extraction stays local.** We have the full transcript text; Gong only has Gong-recorded calls. Local PTY extraction remains the primary path for transcript intelligence. Gong data supplements.
5. **Meeting prep assembly stays mechanical.** `MeetingPrepQueue` consumes `entity_assessment` regardless of whether it was filled by Glean or PTY. No change.
6. **ADR-0098 data governance applies.** Glean-sourced data is tagged with `data_source = "glean"`. `purge_source(DataSource::Glean)` removes all Glean-sourced data on disconnect.

---

## Consequences

### Positive
- Intelligence quality dramatically improves for Glean-connected users (cross-source data)
- Onboarding goes from hours of manual setup to minutes of automatic discovery
- Claude Code subscription no longer required for Glean-connected users
- Cost shifts from per-token Claude usage to enterprise Glean subscription (already paid)
- Enrichment is faster (10-30s vs 60-180s)

### Negative
- Two distinct intelligence quality tiers emerge (Glean vs non-Glean users)
- Glean dependency for the best experience (but graceful degradation to local-only)
- Token refresh/expiry needs robust handling (DCR registrations can expire)
- Glean's AI synthesis can hallucinate — same risk as PTY calls, but from a different source

### Risks
- Glean rate limiting could throttle bulk enrichment during onboarding
- Glean MCP API is not versioned — breaking changes possible
- Internal Slack data in intelligence output could surface sensitive information
- Different Glean instances have different connected apps — feature parity varies by org

---

## Related ADRs

- **ADR-0095** (Dual-mode context) — this ADR updates the "dual" from "local primary + Glean supplementary" to "Glean primary + local fallback"
- **ADR-0097** (Health scoring) — unchanged architecturally, but gains richer inputs from Glean-sourced signals
- **ADR-0098** (Data governance) — Glean data tagged and purgeable per existing design
- **ADR-0099** (Remote-first) — WITHDRAWN. ADR-0100 achieves the benefits of remote intelligence without the server-sync architecture that ADR-0099 proposed.
