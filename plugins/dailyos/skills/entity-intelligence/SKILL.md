---
name: entity-intelligence
description: "Auto-loads full entity context when any entity name is mentioned"
---

# Entity Intelligence

This skill fires automatically whenever an entity name is mentioned in conversation. It silently reads all available intelligence for that entity so context is loaded before any command executes. The user should never be asked for information that exists in the workspace.

## Activation Trigger

Activate when:
- A user mentions a name that matches an `Accounts/` or `Projects/` directory
- A command is invoked with an entity argument
- A meeting from `data/schedule.json` references an entity
- An action from `data/actions.json` references an entity

## Entity Resolution

1. Read directory listings of `Accounts/` and `Projects/`
2. Match the mentioned name against directory names (case-insensitive, partial match)
3. If exact match found, proceed silently
4. If multiple partial matches, ask user to clarify: "Did you mean Accounts/Nielsen or Accounts/NielsenIQ?"
5. If no match, inform user: "No entity found matching '{name}'. Would you like me to search People/ instead?"

## Silent Context Loading

When an entity is resolved, read the following files without prompting the user. Load them into context so they are available for any subsequent command or question.

### 1. dashboard.json

Read `{entity-path}/dashboard.json` for quantitative vitals:
- Financial metrics (ARR, revenue, deal size)
- Health status (Green/Yellow/Red)
- Key dates (renewal, contract end, next milestone)
- Lifecycle stage
- Owner and tier

Note any missing fields — they represent data gaps to flag if relevant.

### 2. intelligence.json

Read `{entity-path}/intelligence.json` for qualitative intelligence:
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

Read `data/actions.json` and filter for actions where the `entity` field matches. Surface:
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

1. **Silent loading.** Never announce "I'm loading entity intelligence for Nielsen." Just have it ready.
2. **No redundant asks.** If the user says "How is Nielsen doing?" and dashboard.json has the health status, answer from workspace data. Do not ask the user to tell you.
3. **Staleness flagging.** If intelligence is stale (>14 days), mention it naturally: "Based on intelligence last updated January 3rd..."
4. **Gap awareness.** If a file is missing or empty, note it internally. If the user asks about something in a missing file, explain the gap: "Nielsen doesn't have stakeholders mapped yet. Would you like me to create a stakeholder map?"
5. **Multi-entity support.** If multiple entities are mentioned, load context for each. Keep them distinct in your working memory.

## Interaction with Other Skills

- **workspace-fluency** provides the file structure knowledge this skill depends on
- **relationship-context** fires in parallel when stakeholder names are loaded
- **action-awareness** uses the filtered actions this skill surfaces
- **role-vocabulary** shapes how entity vitals are described (health frame, risk vocabulary)
- **political-intelligence** may fire if stakeholder dynamics suggest tension
