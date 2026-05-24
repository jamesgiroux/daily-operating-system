---
name: entity-intelligence
description: "Auto-loads full entity context when any entity name is mentioned"
---

# Entity Intelligence

This skill fires automatically whenever an entity name is mentioned in conversation. It silently loads available DailyOS runtime context for that entity so context is available before any command executes. The user should never be asked for information that DailyOS runtime or MCP tools can provide.

## Activation Trigger

Activate when:
- A user mentions a name that matches an `Accounts/` or `Projects/` directory
- A command is invoked with an entity argument
- A DailyOS meeting/schedule tool references an entity
- A DailyOS action/work tool references an entity

## Entity Resolution

1. Use DailyOS MCP/runtime entity resolution when available.
2. Match by id, slug, or renderable name.
3. If exact match found, proceed silently.
4. If multiple matches, ask the user to clarify.
5. If runtime tools are unavailable, directory names under `Accounts/` and `Projects/` may be used as a fallback locator, not as authority.

## Silent Context Loading

When an entity is resolved, load the following context without prompting the user. Use DailyOS runtime/MCP tools first. Generated JSON and markdown files are export projections for portability; read them only when runtime tools are unavailable or the user explicitly asks for file artifacts. User-authored transcripts, notes, and documents remain source material.

### 1. Runtime Vitals

Use DailyOS entity tools for quantitative vitals:
- Financial metrics (ARR, revenue, deal size)
- Health status (Green/Yellow/Red)
- Key dates (renewal, contract end, next milestone)
- Lifecycle stage
- Owner and tier

Note any missing fields — they represent data gaps to flag if relevant.

### 2. Runtime Intelligence

Use DailyOS entity intelligence tools for qualitative intelligence:
- Executive assessment — the current narrative
- Risks — each with evidence source and impact level
- Wins — recent positive signals and their significance
- Current state — what is happening right now
- Stakeholder insights — relationship dynamics
- Last updated timestamp — check for staleness

If `last_updated` is older than 14 days, note internally that intelligence may be stale.

### 3. stakeholders.md

Read `{entity-path}/stakeholders.md` for the relationship map:
- Champion — who is the internal advocate
- Executive sponsor — who signs off
- Economic buyer — who controls budget
- Technical buyer — who evaluates implementation
- Influencers and blockers
- Engagement levels and sentiment for each stakeholder

Cross-reference stakeholder names against `People/` directories for deeper profiles.

### 4. Filtered Actions

Use DailyOS action/work tools and filter for actions where the entity matches. Surface:
- Open actions (especially overdue ones)
- Recently completed actions
- Actions assigned to specific people
- Actions from recent meetings

### 5. Recent Archive Entries

Scan `_archive/` for recent meeting summaries mentioning this entity:
- Read the two most recent monthly directories (`_archive/YYYY-MM/`)
- Search filenames and content for entity name references
- Load relevant summaries for meeting history context

This provides the trajectory — not just where the entity is now, but the direction it has been moving.

## Context Assembly

After loading, the following context is available to any command:

```
Entity: {name}
Type: Account | Project
Path: {entity-path}

Vitals:
  - Health: {status}
  - ARR/Value: {amount}
  - Renewal/End: {date}
  - Lifecycle: {stage}
  - Owner: {name}

Intelligence:
  - Assessment: {executive_assessment}
  - Risks: {count} identified
  - Wins: {count} recent
  - Last Updated: {date}

Stakeholders:
  - Champion: {name}
  - Exec Sponsor: {name}
  - {count} mapped stakeholders

Actions:
  - {count} open ({count} overdue)
  - {count} completed recently

Meeting History:
  - {count} meetings in last 60 days
  - Last meeting: {date} — {summary}
  - Trajectory: {pattern}
```

## Behavior Rules

1. **Silent loading.** Never announce "I'm loading entity intelligence for Acme Corp." Just have it ready.
2. **No redundant asks.** If the user says "How is Acme Corp doing?" and DailyOS runtime has the health status, answer from runtime data. Do not ask the user to tell you.
3. **Staleness flagging.** If intelligence is stale (>14 days), mention it naturally: "Based on intelligence last updated January 3rd..."
4. **Gap awareness.** If a file is missing or empty, note it internally. If the user asks about something in a missing file, explain the gap: "Acme Corp doesn't have stakeholders mapped yet. Would you like me to create a stakeholder map?"
5. **Multi-entity support.** If multiple entities are mentioned, load context for each. Keep them distinct in your working memory.

## Interaction with Other Skills

- **workspace-fluency** provides the file structure knowledge this skill depends on
- **relationship-context** fires in parallel when stakeholder names are loaded
- **action-awareness** uses the filtered actions this skill surfaces
- **role-vocabulary** shapes how entity vitals are described (health frame, risk vocabulary)
- **political-intelligence** may fire if stakeholder dynamics suggest tension
