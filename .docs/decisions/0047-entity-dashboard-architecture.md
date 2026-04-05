# ADR-0047: Entity dashboard architecture — two-file pattern with bidirectional sync

**Date:** 2026-02-07
**Status:** Accepted
**Builds on:** [ADR-0004](0004-hybrid-json-markdown-architecture.md) (JSON + Markdown), [ADR-0018](0018-hybrid-storage-markdown-sqlite.md) (Markdown + SQLite), [ADR-0028](0028-structured-document-schemas.md) (structured document schemas), [ADR-0045](0045-entity-abstraction.md) (entity abstraction), [ADR-0046](0046-entity-mode-architecture.md) (entity-mode architecture)

## Context

Entity dashboards (account pages, project pages) need to serve two audiences:

1. **The app** — renders a composite view from structured data, needs fast queries, supports in-app editing
2. **The ecosystem** — Claude Desktop, Claude Code, ChatGPT, and any future AI tool pointed at the workspace directory needs rich, queryable, AI-consumable files

The archive is the product, not the app (PHILOSOPHY.md). A query like "find 3 customers who expressed interest in analytics software" must work by searching `Accounts/*/dashboard.md` files without any knowledge of the app's internals. The markdown must be comprehensive enough to support this — not a thin export.

Meanwhile, the app needs structured data for fast rendering, queryable fields for cross-entity views (portfolio attention, account list sorting), and edit affordances that don't require parsing markdown.

ADR-0004 established JSON for machines, markdown for humans. ADR-0018 established markdown as source of truth, SQLite as disposable cache. ADR-0028 established JSON-first schemas with markdown generated from JSON. These decisions are sound but create a tension for entity dashboards: who owns what when multiple tools can write?

### Forces

- **Markdown must be rich.** An account dashboard read by Claude Desktop should contain company overview, stakeholder map, strategic programs, recent wins, open actions, engagement signals — everything needed to answer complex questions about the account.
- **The app must be fast.** Loading a dashboard shouldn't require parsing a 200-line markdown file. JSON and SQLite provide the structured query path.
- **External tools must be able to write.** Claude Desktop asked to "update Acme's strategy section" should be able to do so without understanding the app's internals.
- **Changes must flow in both directions.** An in-app edit must update the markdown. An external edit must be reflected in the app.
- **Structured fields must be queryable.** Sorting accounts by ARR or filtering by health requires SQLite, not grep.

## Decision

### Two-file pattern: JSON as write interface, markdown as read interface

Every entity dashboard consists of two files in its workspace directory:

```
Accounts/Acme Corp/
├── dashboard.json    # Canonical data (app + external tools write here)
└── dashboard.md      # Rich artifact (app generates, external tools read here)
```

**`dashboard.json`** is the canonical representation of all entity dashboard content — both structured fields and narrative sections. It is the write target for the app, for external tools, and for AI enrichment. Its schema is intentionally simple so any LLM can read or edit it.

**`dashboard.md`** is the rich, comprehensive, AI-consumable artifact. Generated from `dashboard.json` + live SQLite queries (meetings, actions, captures, stakeholder signals). This is what makes the archive valuable to the ecosystem. Regenerated on every relevant data change.

### Data ownership model

| Data type | Canonical location | Examples |
|-----------|-------------------|----------|
| **Structured fields** | `dashboard.json` → bridged to SQLite | ARR, health, ring, renewal date, NPS |
| **Narrative content** | `dashboard.json` | Company overview, strategy notes, programs, user notes |
| **AI-enriched content** | `dashboard.json` (with `enrichedAt` timestamp) | Company description, industry, public facts |
| **Live data** | SQLite (queried fresh, never stored in template) | Recent meetings, open actions, captures, stakeholder signals |

SQLite mirrors structured fields from JSON for queryability (sorting account lists by ARR, filtering by health). The JSON is canonical — if JSON and SQLite disagree, JSON wins on next sync.

### JSON schema (accounts)

```json
{
  "version": 1,
  "entityType": "account",
  "structured": {
    "arr": 1200000,
    "health": "green",
    "ring": 1,
    "renewalDate": "2025-08-01",
    "nps": 72,
    "csm": "Jamie Giroux",
    "champion": "Sarah Chen"
  },
  "companyOverview": {
    "description": "Acme Corp is a Fortune 500 technology company...",
    "industry": "Technology",
    "size": "10,000+ employees",
    "headquarters": "San Francisco, CA",
    "enrichedAt": "2026-02-07T06:00:00Z"
  },
  "strategicPrograms": [
    { "name": "Phase 2: Advanced Analytics", "status": "in_progress", "notes": "Q2 target" }
  ],
  "notes": "Key relationship observations...",
  "customSections": [
    { "title": "Renewal Strategy", "content": "Multi-year proposal with 10% growth clause..." }
  ]
}
```

### JSON schema (projects)

```json
{
  "version": 1,
  "entityType": "project",
  "structured": {
    "status": "in_progress",
    "milestone": "Beta launch",
    "owner": "Jamie Giroux",
    "targetDate": "2026-04-15"
  },
  "description": "Redesigning the customer onboarding flow...",
  "milestones": [
    { "name": "Design complete", "date": "2026-03-01", "status": "done" },
    { "name": "Beta launch", "date": "2026-04-15", "status": "planned" }
  ],
  "notes": "...",
  "customSections": []
}
```

### Markdown generation

`dashboard.md` is generated by combining:

1. **Header** from `dashboard.json` structured fields (name, health badge, tier)
2. **Narrative sections** from `dashboard.json` (overview, programs, notes, custom sections)
3. **Live sections** from SQLite queries (recent meetings, open actions, captures, stakeholder map, intelligence signals)

Live sections are marked with a generation comment so external tools know not to edit them:

```markdown
## Recent Meetings
<!-- auto-generated from meeting history — edits here will be overwritten -->
| Date | Meeting | Key Outcomes |
|------|---------|-------------|
| 2026-02-05 | Weekly Sync | Phase 2 timeline confirmed |
```

Regeneration triggers:
- After briefing delivery (new meetings, actions)
- After meeting capture (new wins/risks/decisions)
- After in-app edits (strategy notes, programs, structured fields)
- After AI enrichment (company overview refresh)
- After external JSON edit detected (see sync model below)

### Sync model — three-way bridge

```
dashboard.json ←→ SQLite (structured fields)
      ↓
dashboard.md (generated output)
```

**App → JSON → SQLite → Markdown:**
1. User edits in app (e.g., changes health to "yellow")
2. App writes to `dashboard.json` `structured.health`
3. App syncs to SQLite `accounts.health`
4. App regenerates `dashboard.md`

**External tool → JSON → App:**
1. External tool writes to `dashboard.json` (e.g., Claude Desktop updates strategy notes)
2. App detects JSON change (file watcher or mtime check on next access)
3. App reads updated JSON, syncs structured fields to SQLite
4. App regenerates `dashboard.md`
5. App UI reflects changes on next render

**External tool → Markdown (graceful handling):**
1. External tool edits `dashboard.md` directly (not recommended but will happen)
2. App detects markdown mtime is newer than last generation timestamp
3. App shows "externally modified" indicator on the entity dashboard
4. User can trigger reconciliation: app uses AI to extract changes from markdown diff, applies to JSON, regenerates markdown
5. Without reconciliation, next markdown regeneration overwrites external changes (with a warning)

The recommended external write path is always JSON. The markdown path is a fallback with explicit user-triggered reconciliation.

### AI enrichment

On account creation or on-demand refresh, the app spawns Claude Code to websearch the company name:

1. User creates account "Acme Corp" or clicks "Refresh" on company overview
2. App spawns Claude Code with `--print`: "Research Acme Corp. Return JSON with description, industry, size, headquarters, and 3-5 key facts."
3. Claude returns structured JSON
4. App writes to `dashboard.json` `companyOverview` section with `enrichedAt` timestamp
5. App regenerates `dashboard.md`
6. If enrichment fails, the company overview section is empty — the dashboard still renders with all other data (fault-tolerant, per ADR-0042 pattern)

Enrichment is **on-demand only** in v1. No scheduled enrichment in the morning briefing. Future: enrichment for accounts with meetings today could be triggered as part of prep generation.

### Account directory as entity hub

The dashboard is one artifact in a directory that grows over time:

```
Accounts/Acme Corp/
├── dashboard.json          # Working format
├── dashboard.md            # Rich artifact
├── success-plan.json       # Future: app-maintained
├── success-plan.md         # Future: rendered
├── Projects/               # Sub-initiatives linked to this account
│   └── phase-2-analytics/
└── (transcripts, meeting notes, etc. filed by the system)
```

The app maintains structured files. Users and external tools can add their own files. Everything in the directory is part of the account's operational intelligence.

## Consequences

**Easier:**
- External AI tools get rich, comprehensive, always-current markdown for every entity
- The archive quality improves automatically as the app maintains it
- JSON write interface is simple enough for any LLM to edit
- Structured fields are queryable in SQLite for portfolio views
- Enrichment uses proven PTY/Claude Code pattern
- Account directories become self-contained knowledge hubs

**Harder:**
- Three-way sync (JSON ↔ SQLite ↔ markdown) is more complex than one-way
- File watcher infrastructure needed for external edit detection
- Markdown regeneration must be efficient (runs frequently)
- AI-powered reconciliation for markdown edits is a future investment
- External tools that edit markdown instead of JSON create a reconciliation debt

**Trade-offs:**
- Chose JSON as canonical over markdown as canonical — avoids fragile markdown parsing (ADR-0004 rationale), accepts that external write path requires JSON knowledge
- Chose regeneration over incremental update for markdown — simpler, avoids merge conflicts, accepts re-generation cost
- Chose on-demand enrichment over scheduled — avoids morning briefing latency, accepts staler company data
- Chose explicit reconciliation over automatic for markdown edits — honest about the complexity, doesn't pretend bidirectional markdown sync is free
- The `<!-- auto-generated -->` comments in markdown are a convention, not enforcement — external tools could still edit those sections, and those edits would be lost on regeneration
