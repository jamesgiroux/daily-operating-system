---
description: "Generate ready-to-use deliverables — status updates, QBR narratives, board decks"
---

# /produce

Generate polished, ready-to-use deliverables grounded in workspace data. These are not drafts for editing — they are finished documents you can copy-paste into the target format. Structure matches how these documents look in the real world.

## Arguments

- `$ARGUMENTS[0]` — Deliverable type (required). See types by preset below.
- `$ARGUMENTS[1]` — Entity name or scope (optional). Single entity or "portfolio" for cross-entity deliverables.

## Deliverable Types by Role Preset

| Preset | Deliverable Types |
|---|---|
| Customer Success | QBR narrative, health report, success plan, executive business review, renewal proposal |
| Sales | Deal memo, proposal summary, competitive positioning, pipeline review, forecast narrative |
| Partnerships | Partner review, joint business plan, co-sell summary, integration status |
| Agency | Client report, project status, creative brief, campaign recap, retainer review |
| Consulting | Engagement update, milestone report, recommendations deck, impact assessment |
| Product | Feature brief, launch plan, adoption report, roadmap narrative, market analysis |
| Leadership | Board contribution, team update, strategic review, initiative status, quarterly narrative |
| The Desk | Auto-detect from context and deliverable type |

## Workflow

### Step 1: Identify Deliverable and Scope

Parse the arguments to determine:
- **Deliverable type** — Match against the preset's deliverable types. If the user says something close but not exact (e.g., "board update" for "board contribution"), map it.
- **Scope** — Single entity (read one entity directory) or portfolio (read across all entities). Infer from context if not explicit.

### Step 2: Read Workspace Context

For single-entity deliverables:
- Entity dashboard.json, intelligence.json, stakeholders.md
- Filtered actions from data/actions.json
- Recent _archive/ meeting summaries for this entity
- People/ profiles for key stakeholders
- Email signals from data/emails.json related to this entity

For portfolio-wide deliverables:
- All entity dashboards (scan Accounts/ and Projects/)
- Portfolio-level metrics (aggregate health, ARR, renewal timeline)
- Cross-entity patterns from intelligence files
- data/actions.json for portfolio-wide action status

### Step 3: Structure the Deliverable

Match the real-world format for this deliverable type. These are not generic reports — they follow the conventions of the actual document type.

### Step 4: Write with Evidence

Every claim must be grounded in workspace data. Include quantitative evidence where available (metrics, dates, completion rates) and qualitative evidence where appropriate (meeting tone, stakeholder sentiment, trajectory observations).

**Flag data gaps.** If a section requires data that does not exist in the workspace, do not fabricate it. Instead: "Note: Q4 NPS data not available in workspace. Recommend sourcing from [team/system] before finalizing."

### Step 5: Quality Check

The deliverable must be ready to use as-is:
- Correct tone for the audience (external-facing docs are professional and polished, internal docs can be more direct)
- No placeholder language ("insert metric here", "[TBD]") — use actual data or flag the gap
- Structure matches how this document type looks in professional settings
- Voice is appropriate for the role preset

### Step 6: Output

Present the complete deliverable.

**Example: Board Contribution (Leadership preset, single entity)**

```markdown
# Nielsen — Board Update
**Period:** Q1 2026 | **Owner:** {workspace owner}

## Summary
Nielsen ARR grew 18% QoQ to $2.4M, driven by platform expansion in their analytics division. Health status upgraded from Yellow to Green in January following successful EBR with their VP Data. The primary risk — champion departure — was mitigated when Sarah Chen was promoted to VP Customer Success, strengthening our executive alignment.

## Key Metrics
| Metric | Q4 2025 | Q1 2026 | Trend |
|---|---|---|---|
| ARR | $2.03M | $2.4M | +18% |
| Health | Yellow | Green | Improved |
| DAU | 620 | 890 | +44% |
| Open Actions | 8 | 3 | Resolved |

## What's Working
- Platform expansion in analytics division generated $370K incremental ARR
- Sarah Chen's promotion to VP created stronger executive sponsorship
- Adoption metrics recovered after January enablement push

## Watch Items
- Renewal conversation begins April — need to lock in multi-year before budget cycle
- Integration timeline commitment (due March 15) is the remaining dependency
- David Park (CTO) has not attended last two QBRs — re-engage before renewal

## Ask / Next Steps
- Approve dedicated CSM allocation for Q2 to support expansion motion
- Executive sponsor meeting with their CFO to discuss multi-year pricing
```

### Step 7: Loop-Back

After presenting the deliverable:

```
Would you like me to:
1. Save this to Accounts/Nielsen/board-update-2026-q1.md
2. Create actions for the "Next Steps" items in data/actions.json

Or adjust anything before finalizing?
```

## Skills That Contribute

- **entity-intelligence** — Auto-fires to load entity context
- **relationship-context** — Auto-fires for stakeholder references
- **role-vocabulary** — Determines deliverable types and voice
- **action-awareness** — Surfaces open and completed actions for evidence
- **meeting-intelligence** — Provides meeting history for narrative
- **loop-back** — Handles saving the deliverable and creating actions
