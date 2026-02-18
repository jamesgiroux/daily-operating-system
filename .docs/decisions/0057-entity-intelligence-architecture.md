# ADR-0057: Entity Intelligence Architecture

**Date:** 2026-02-09
**Status:** Accepted
**Supersedes:** ADR-0047 enrichment model (on-demand, 4-field web search)
**Builds on:** [ADR-0047](0047-entity-dashboard-architecture.md) (two-file pattern), [ADR-0048](0048-three-tier-data-model.md) (three-tier data model), [ADR-0042](0042-per-operation-pipelines.md) (per-operation pipelines), [ADR-0043](0043-meeting-intelligence-is-core.md) (meeting intelligence is core), [ADR-0046](0046-entity-mode-architecture.md) (entity-mode architecture)

## Context

The account detail page shows metadata (ARR, health, lifecycle, a Wikipedia-quality company description from web search) while 600+ lines of deep operational intelligence sits in workspace files the app ignores. Users have been building account dashboards with executive summaries, risk assessments with source citations, stakeholder maps with relationship dynamics, value tracking with before/after evidence, discovery gaps, and strategic context — all in markdown. The app's enrichment prompt does a web search and returns 4 fields (description, industry, size, headquarters).

This is the same disease the weekly page had before ADR-0052: a dashboard when it should be a briefing. The account detail page is a Grafana panel. It should be an intelligence document.

### Forces

- **The intelligence exists.** Workspace files contain exactly what users need — the app just doesn't surface it. Call transcripts, meeting summaries, success plans, and hand-built account dashboards hold months of accumulated intelligence.
- **Enrichment produces trivia.** "Nielsen is a media analytics company" is Wikipedia. "Nielsen's exit clause request is procurement hygiene under new leadership, not active churn" is intelligence. The enrichment pipeline produces the former while the latter sits in a file.
- **Factual data needs protection.** ARR, lifecycle, health, renewal dates are facts that may come from CSV imports or CRM integrations. AI enrichment must never overwrite these.
- **Intelligence should auto-update.** A user adds a call transcript to an account directory. The intelligence for that account should refresh without manual intervention. Cost is minimal — one file/week/account, one Claude call.
- **Incremental updates must not erase signals.** If a stakeholder mentioned Parse.ly six months ago and subsequent updates pushed that signal out, the intelligence missed a trend. The pipeline must be able to reach back to raw source files, not just see its own prior synthesis.
- **Briefings consume entity intelligence.** The daily briefing, weekly briefing, and meeting prep are rendering surfaces. They should pull from per-entity intelligence, not generate their own. Cross-entity synthesis ("your busiest account this week is Nielsen, but Heroku's renewal is closer") is the briefing layer's job.

## Decision

### Three-file entity pattern

Extend ADR-0047's two-file pattern. Every entity has three files in its workspace directory. This pattern applies to **all entity types** — accounts, projects, and people:

```
Accounts/Acme Corp/                Projects/Platform Migration/
├── dashboard.json                 ├── dashboard.json
├── intelligence.json              ├── intelligence.json
└── dashboard.md                   └── dashboard.md

# dashboard.json   → Mechanical: factual data (app + external tools write)
# intelligence.json → Synthesized: AI-generated intelligence (enrichment writes)
# dashboard.md     → Rich artifact: generated from both + SQLite live data
```

**`dashboard.json`** — factual, importable/exportable, user-editable, CSV-compatible. Schema varies by entity type — accounts have ARR/health/lifecycle, projects have status/milestone/owner — but the pattern is identical:
```json
{
  "version": 1,
  "entityType": "account",
  "structured": {
    "arr": 556250,
    "health": "yellow",
    "lifecycle": "evolution",
    "renewalDate": "2026-12-01",
    "nps": 70,
    "csm": "James Giroux",
    "champion": "Efthymia"
  },
  "strategicPrograms": [
    { "name": "Rebranding Initiative", "status": "in_progress", "notes": "April-May launch" }
  ],
  "notes": "User's freeform notes"
}
```

These are facts. They can be bulk-imported from CSV, edited in-app, or written by external tools. AI enrichment never touches this file.

**`intelligence.json`** — AI-synthesized, auto-updated, derived from source material. The schema is shared across entity types (accounts, projects, people each populate the same fields with type-appropriate content — a project's "risks" are blockers, a person's "executiveAssessment" is a relationship assessment):
```json
{
  "version": 1,
  "entityId": "nielsen",
  "enrichedAt": "2026-02-09T14:30:00Z",
  "sourceFileCount": 12,
  "executiveAssessment": "Nielsen is at a critical juncture. The exit clause request (Jan 22) signals procurement hygiene under new leadership, not active churn. The Feb 5 meeting introducing Samantha Severin — a Six Sigma Black Belt brought in to rationalize vendor spend — is the decisive moment. Lead with value delivered. Renewal is 10 months out and manageable if this meeting goes well.",
  "risks": [
    {
      "text": "Exit clause requested mid-contract — procurement policy change cited",
      "source": "2026-01-22 meeting notes",
      "urgency": "Feb 5 meeting is the critical response moment",
      "dateIdentified": "2026-01-22"
    }
  ],
  "recentWins": [
    {
      "text": "Parse.ly demo requested by content team — expansion signal",
      "source": "2026-01 monthly sync",
      "impact": "$15-25k expansion opportunity",
      "date": "2026-01"
    }
  ],
  "currentState": {
    "working": [
      "Monthly strategic cadence now established with consistent attendance",
      "Gustavo promoted to Marketing Specialist — strengthens technical partnership"
    ],
    "notWorking": [
      "Previous meeting cadence was weekly/bi-weekly with 20 people for 7-minute meetings",
      "Roadmap visibility: Efthemia never saw high-level feature launches"
    ],
    "unknowns": [
      "2026 budget availability for travel/on-site EBR",
      "Specific brand marketing KPIs for 2026",
      "Betty Junod (CMO) engagement level — no direct VIP engagement yet"
    ]
  },
  "stakeholderInsights": [
    {
      "name": "Samantha Severin",
      "role": "Head of Marketing Ops",
      "assessment": "Six Sigma Black Belt brought in to optimize and rationalize. Will respond to data and outcomes, not relationship pitches.",
      "engagement": "incoming",
      "source": "2026-01 meeting notes"
    }
  ],
  "valueDelivered": [
    {
      "date": "2025-12",
      "statement": "Strategic feature review completed — Enterprise Search, SSO, Engagement Boost, Jetpack AI",
      "source": "2025-12-15 monthly call",
      "impact": "Team actively exploring applicability"
    }
  ],
  "nextMeetingReadiness": {
    "meetingDate": "2026-02-05",
    "meetingTitle": "Monthly sync — Samantha Severin introduction",
    "prepItems": [
      "Lead with value delivered, not relationship pitches — Samantha is data-driven",
      "Address exit clause context: our read is procurement hygiene, be transparent",
      "Parse.ly demo follow-up from Jan content team request is still open"
    ]
  },
  "companyContext": {
    "description": "100-year-old media analytics and measurement company...",
    "industry": "Media Analytics & Measurement",
    "size": "Global enterprise",
    "headquarters": "New York, NY",
    "recentContext": "2020 split into Nielsen IQ + Nielsen Media. Team recently moved from performance marketing to brand marketing under communications umbrella."
  }
}
```

**`dashboard.md`** — generated from dashboard.json + intelligence.json + SQLite live queries. The rich artifact for ecosystem consumption. Regenerated on any change to either JSON file or relevant SQLite data.

### Separation of concerns

The table below uses account fields as examples. The same separation applies to all entity types — project fields (status, milestone, owner, target_date) and people fields (organization, role, relationship) follow the same rules.

| Data | File | Who writes | Protected from AI? |
|------|------|-----------|-------------------|
| Entity-specific facts (accounts: ARR, health; projects: status, milestone; people: role, org) | dashboard.json | User, CSV import, CRM sync | Yes — AI never touches |
| Strategic programs, user notes | dashboard.json | User, external tools | Yes — user-authored |
| Executive assessment, risks, wins | intelligence.json | AI enrichment pipeline | No — AI-generated, AI-updated |
| Current state, stakeholder insights | intelligence.json | AI enrichment pipeline | No — AI-generated |
| Next meeting readiness | intelligence.json | AI enrichment pipeline | No — refreshed on calendar/content changes |
| Company context | intelligence.json | AI enrichment (web search + files) | No — replaces ADR-0047 companyOverview |
| Recent meetings, open actions | SQLite (live queries) | Briefing pipeline, user actions | N/A — always live |

### Intelligence trigger pipeline

```
Source material changes → Content index updates → Intelligence refresh → Surfaces update
```

Triggers that cause an intelligence refresh:

1. **New/changed file in entity directory** — Watcher detects file → content_index updates → intelligence refresh queued
2. **Inbox processes file to entity** — Inbox pipeline routes a file to account directory → same watcher flow
3. **Post-meeting capture** — New transcript or capture recorded → triggers refresh for associated entity
4. **Calendar change** — Next meeting for entity changes → nextMeetingReadiness section refreshed
5. **Manual refresh** — User clicks "Refresh Intelligence" on detail page

Intelligence refresh is **debounced** — multiple file additions within a short window produce one refresh, not N refreshes.

### Incremental enrichment with raw file access

Intelligence updates are incremental, not full rebuilds:

1. Pipeline reads existing `intelligence.json` (the prior synthesis)
2. Pipeline reads the **new/changed content** that triggered the refresh
3. Pipeline reads SQLite signals (meeting frequency, action status, stakeholder engagement patterns, captures)
4. Claude produces an updated `intelligence.json` — refining the existing intelligence, not starting from scratch

However, Claude can **request raw file access** when needed. The enrichment prompt includes a manifest of all indexed files for the entity. If the model determines it needs deeper context — e.g., to trace when a stakeholder first mentioned a product, or to verify a risk that may have been resolved — it can request specific files be included in a follow-up turn.

This prevents "signal amnesia" where important early signals get pushed out by more recent content. The intelligence accumulates over time rather than being a sliding window.

**First run** is a full build: all indexed files + web search → initial intelligence.json. Subsequent runs are incremental.

### Persistent entity prep (replaces per-meeting prep)

ADR-0033's per-meeting prep files (`meeting_prep_state` table) are ephemeral — generated before a meeting, consumed once, forgotten. This ADR replaces that with **persistent entity intelligence that includes meeting readiness**.

The `nextMeetingReadiness` section in `intelligence.json` is a living prep brief:
- Automatically updated when the next meeting changes (calendar sync)
- Refreshed when new content arrives (transcript adds context)
- Updated when actions complete or become overdue
- Includes "N things to work on or consider before this meeting"

This is not prep for a specific meeting — it's the entity's readiness posture. "Your next meeting with Nielsen is in 3 weeks. Here are 3 things to work on before then."

### Briefings consume entity intelligence

The daily and weekly briefings remain separate enrichment surfaces but consume per-entity intelligence as input:

- **Daily briefing**: Pulls intelligence.json for entities with meetings today. Uses executive assessments and nextMeetingReadiness for meeting prep context. Adds cross-entity synthesis: "Your three meetings today span two accounts — Nielsen and Heroku — both in renewal window."
- **Weekly briefing**: Pulls intelligence.json for all active entities. Uses risks, wins, and readiness to compose the weekly narrative. Adds cross-entity synthesis: "Your busiest account this week is Nielsen, but Heroku's renewal is closer."
- **Meeting prep**: Pulls nextMeetingReadiness from the associated entity's intelligence.json. No separate prep generation needed.

Entity intelligence is the **data layer**. Briefings are the **presentation layer** that synthesizes across entities.

### SQLite as intelligence graph

SQLite holds the connection graph that feeds intelligence enrichment:

```
meetings_history ←→ meeting_entities ←→ accounts/projects
                ←→ meeting_attendees ←→ people
                                    ←→ entity_people ←→ accounts/projects
actions ←→ accounts (account_id) ←→ people (person_id)
captures ←→ meetings (meeting_id) ←→ accounts (account_id)
content_index ←→ entities (entity_id)
```

When enriching an entity, the pipeline queries:
- Meeting frequency and recency from `meeting_entities` + `meetings_history`
- Stakeholder engagement from `entity_people` + `meeting_attendees` + `people`
- Open commitments from `actions` WHERE account_id = entity
- Recent wins/risks from `captures` WHERE account_id = entity
- Source material from `content_index` WHERE entity_id = entity

These structured signals combine with file content to produce intelligence. SQLite provides the quantitative layer (meeting trends, action counts, engagement patterns). Files provide the qualitative layer (strategic context, relationship dynamics, business understanding).

### Entity detail page redesign

The page layout follows the weekly briefing patterns (ADR-0052 research): conclusions before evidence, tapering word count, "why now?" framing. This layout applies to all entity types. The example below uses accounts; projects follow the same structure with project-specific metrics (milestone progress, blockers, target date countdown instead of renewal countdown).

**Top to bottom:**

1. **Hero + metrics** — Name, health/status dot, lifecycle/phase badge, engagement temperature, compact metrics row (entity-specific: accounts get ARR + renewal countdown, projects get milestone + target date)
2. **Executive Assessment** — Full-width prose paragraph, no card wrapper. The intelligence headline. Generated from intelligence.json.executiveAssessment.
3. **Attention Items** — Risks (peach), wins (sage), action-needed (gold) with source citations and temporal urgency. From intelligence.json.risks + recentWins + SQLite overdue actions.
4. **Next Meeting Readiness** — "Your next meeting with Nielsen is Feb 5. Three things to consider..." From intelligence.json.nextMeetingReadiness.
5. **Commitments** — Open actions with "why this matters" context, grouped by temporal urgency. From SQLite actions + intelligence context.
6. **Stakeholder Intelligence** — Full-width stakeholder cards with engagement level, last contact, relationship notes, gaps. From intelligence.json.stakeholderInsights + SQLite people data.
7. **Evidence & History** — Collapsed by default. Value delivered timeline, current state (working/not working/unknowns), recent meetings, captures, files. Progressive disclosure.

**Sidebar:**
- Entity metadata (editable factual fields — varies by entity type)
- Notes (freeform)
- Company/project context (web-search piece for accounts, project overview for projects — demoted from hero to reference)

### People intelligence

The same pattern applies to people. Person detail pages become relationship intelligence documents:

- **Relationship assessment**: "Sarah Chen is your primary champion at Airbnb. Engagement is warm (monthly cadence). Single-stakeholder dependency creates risk."
- **Cross-entity connections**: Which accounts/projects this person touches
- **Interaction patterns**: Meeting frequency, 1:1 vs group, topics discussed
- **Intelligence gaps**: "Role unknown. No 1:1 meetings — always in group settings."

People intelligence is generated from the same pipeline: meeting_attendees + captures + transcripts mentioning this person → synthesized relationship brief.

### Intelligence lifecycle

| Operation | Trigger | What happens |
|-----------|---------|-------------|
| **Create** | Entity creation + first file scan + web search | Full intelligence build from all available sources |
| **Ingest** | New file, transcript, capture | Incremental intelligence update |
| **Edit** | User flags intelligence as wrong/resolved | Feedback stored, incorporated in next enrichment cycle |
| **Update** | Content change, calendar change, manual refresh | Incremental refresh with raw file access if needed |
| **Retrieve** | Detail page render, briefing enrichment, meeting prep | Read from intelligence.json + SQLite |
| **Archive** | Entity archived | Intelligence preserved in archive (historical value) |

## Consequences

**Easier:**
- Entity detail pages (accounts, projects, people) become the most valuable pages in the app — surfaces months of accumulated intelligence, not just metadata
- Intelligence auto-updates when new content arrives — no manual enrichment clicks
- Briefings get richer context by consuming per-entity intelligence
- Factual data (ARR, lifecycle) is protected from AI overwrites — importable/exportable via CSV
- Stakeholder intelligence, risk tracking, win tracking emerge automatically from workspace content
- Meeting prep is always current — persistent entity readiness replaces ephemeral per-meeting prep
- The archive quality improves dramatically — intelligence.json + rich dashboard.md make the workspace a genuine knowledge base

**Harder:**
- Three-file pattern (dashboard.json + intelligence.json + dashboard.md) is more complex than two-file
- Intelligence enrichment prompt is substantially more complex than the current 4-field web search
- Incremental enrichment requires careful prompt engineering to avoid signal drift
- Raw file access means the pipeline needs a multi-turn capability (or large context window)
- Debouncing intelligence refreshes requires infrastructure (queue, rate limiting)
- Intelligence.json schema will evolve — need migration strategy for schema changes

**Trade-offs:**
- Chose incremental enrichment over full rebuild — faster, cheaper, preserves history, but risks drift. Mitigated by raw file access capability.
- Chose auto-trigger over manual-only — cost is minimal (1 call/file/account), value is immediate. First run is expensive.
- Chose separate intelligence.json over expanding dashboard.json — clear separation of concerns, but one more file to manage. Worth it for protecting factual data.
- Chose persistent entity prep over per-meeting prep — more useful (readiness posture, not one-off prep), but less targeted. Mitigated by nextMeetingReadiness section scoped to the next interaction.
- Chose entity-scoped intelligence consumed by briefings over briefing-scoped intelligence — single source of truth, but briefings need their own synthesis layer for cross-entity narrative. This is the right boundary.
