---
description: "Generate evidence-backed assessments — risk reports, health checks, deal reviews"
---

# /assess

Generate a rigorous, evidence-backed assessment of an entity. Every claim sourced. No generic AI filler. The quality bar: could you send this to your VP?

## Arguments

- `$ARGUMENTS[0]` — Entity name (required). Resolved against Accounts/ and Projects/.
- `$ARGUMENTS[1]` — Assessment type (optional). Auto-detected from role preset if omitted.

## Assessment Types by Role Preset

| Preset | Default Assessment Type | Focuses On |
|---|---|---|
| Customer Success | Health & Renewal Risk | Retention signals, adoption, stakeholder engagement, renewal readiness |
| Sales | Deal Review | Pipeline velocity, competitive position, stakeholder alignment, close probability |
| Partnerships | Partnership Health | Mutual value delivery, alignment, joint execution, relationship depth |
| Agency | Client Satisfaction | Delivery quality, scope alignment, relationship health, growth potential |
| Consulting | Engagement Health | Milestone delivery, stakeholder satisfaction, follow-on potential |
| Product | Product/Feature Health | Adoption metrics, user satisfaction, market fit, technical health |
| Leadership | Initiative Review | Strategic execution, team performance, resource alignment |
| The Desk | Auto-detect | Selects based on entity type and context |

## Workflow

### Step 1: Resolve Entity

Search `Accounts/` and `Projects/` for the named entity. The entity-intelligence skill will auto-fire and load full context:
- `dashboard.json` — Quantitative vitals
- `intelligence.json` — Qualitative intelligence
- `stakeholders.md` — Relationship map
- Filtered actions from `data/actions.json`
- Recent `_archive/` meeting summaries

If entity not found, list available entities and ask the user to clarify.

### Step 2: Determine Assessment Frame

Read the role preset from `data/manifest.json`. Select the assessment frame:
- Use the preset's default assessment type (see table above)
- Override if the user specified a type in arguments
- Apply the preset's `healthFrame`, `riskVocabulary`, and `winVocabulary`

### Step 3: Deep Context Read

Go beyond what entity-intelligence auto-loads:

1. **Stakeholder deep-dive** — For each person in `stakeholders.md`, load their People/ profile (relationship-context skill fires). Note temperature, engagement patterns, follow-through history.
2. **Action trail analysis** — Not just open actions, but the pattern. Are actions being completed on time? Are the same issues recurring? Is there a completion rate trend?
3. **Meeting trajectory** — Read the last 3-5 meeting summaries from `_archive/`. Are meetings getting more or less productive? Are topics progressing or recycling? Is attendance strengthening or thinning?
4. **Email signals** — Check `data/emails.json` for recent signals related to this entity. Escalations, sentiment shifts, volume changes.
5. **Timeline pressure** — Calculate days to key dates (renewal, contract end, next milestone). Flag if within critical windows.

### Step 4: Generate Assessment

Structure the assessment with these sections:

```markdown
# {Entity Name} Assessment
**Date:** {today}
**Type:** {assessment type}
**Assessed by:** {workspace owner from manifest}

## Executive Summary
{2-3 sentence overview of current state and trajectory. Lead with the most important thing.}

**Health:** {status} | **Trajectory:** {improving/stable/declining} | **Key Date:** {renewal/milestone} in {N} days

## Risk Factors

### {Risk 1 Title}
**Evidence:** {Specific source — meeting date, signal, data point}
**Impact:** {What happens if this risk materializes}
**Recommended Action:** {Specific next step}

### {Risk 2 Title}
...

## Strengths to Leverage

### {Strength 1 Title}
**Evidence:** {Specific source}
**Opportunity:** {How to build on this}

### {Strength 2 Title}
...

## Stakeholder Assessment

| Stakeholder | Role | Engagement | Temperature | Key Signal |
|---|---|---|---|---|
| {Name} | {Role} | {Active/Passive/Disengaged} | {Warming/Stable/Cooling} | {One-line signal} |

## Action Trail
- **Completion rate:** {X}% of actions completed on time in last 90 days
- **Overdue items:** {N} ({list most critical})
- **Recurring patterns:** {Any issues that keep appearing}

## Recommended Actions
{Priority-ordered list with specific actions, owners, and dates}

1. **{Action}** — Owner: {name}, by {date}. Rationale: {why this matters now}
2. **{Action}** — Owner: {name}, by {date}. Rationale: {why}
3. **{Action}** — Owner: {name}, by {date}. Rationale: {why}

## Timeline
{Key dates and milestones ahead, with what needs to happen before each}
```

### Step 5: Quality Check

Before presenting the assessment, verify:
- Every risk factor has a specific evidence source (not "seems like" but "per January 15 meeting summary")
- Every recommended action has an owner and date
- The executive summary matches the detail (no contradictions)
- Data gaps are flagged explicitly ("No email signals available — last sync was December")
- Staleness is noted where relevant ("Intelligence last updated 3 weeks ago")

The quality bar is: could you paste this into an email to your VP? If any section reads like generic AI output, rewrite it with specifics.

### Step 6: Output and Loop-Back

Present the assessment to the user. Then offer loop-back options:

```
Would you like me to:
1. Save this assessment to Accounts/Nielsen/health-assessment-2026-02.md
2. Update Accounts/Nielsen/intelligence.json with the revised risk factors and executive assessment
3. Create {N} actions in data/actions.json from the recommendations

Or adjust anything first?
```

## Skills That Contribute

- **entity-intelligence** — Auto-fires to load all entity data
- **relationship-context** — Auto-fires for each stakeholder to load people profiles
- **political-intelligence** — Auto-fires if stakeholder dynamics suggest tension (enriches the assessment but political reads stay implicit unless the user is the sole audience)
- **action-awareness** — Provides the action trail analysis
- **role-vocabulary** — Shapes the assessment frame, risk terms, and win terms
- **analytical-frameworks** — May activate if conflicting signals require structured decomposition
- **loop-back** — Handles the post-assessment save/create workflow
