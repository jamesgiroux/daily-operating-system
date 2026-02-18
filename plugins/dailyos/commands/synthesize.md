---
description: "Detect cross-entity patterns and generate portfolio-level insights"
---

# /synthesize

Read across multiple entities to detect cross-cutting patterns and generate portfolio-level strategic insights. This elevates individual entity signals into observations that would be invisible looking at any single entity alone.

## Arguments

- `$ARGUMENTS[0]` — Scope (optional). "all" (default), a segment name, or a comma-separated list of entity names.
- `$ARGUMENTS[1]` — Timeframe (optional). "last month", "last quarter", "ytd". Defaults to last 90 days.

## Workflow

### Step 1: Determine Scope

**"all" or no argument:**
- Read every entity directory under Accounts/ and Projects/
- Load dashboard.json for each

**Segment name:**
- Filter entities by tier, lifecycle stage, health status, or owner (from dashboard.json fields)
- Example: "at-risk" filters to entities with Red or Yellow health

**Named list:**
- Load only the specified entities: "Nielsen, Datadog, Snowflake"

### Step 2: Portfolio-Wide Data Read

For each entity in scope, load:
- `dashboard.json` — Health, ARR/value, lifecycle, key dates, owner
- `intelligence.json` — Risks, wins, current state (read executive_assessment for each)
- Recent actions from `data/actions.json` filtered by entity
- Recent `_archive/` entries mentioning the entity (last 1-2 months)

Also read portfolio-level data:
- `data/emails.json` — Cross-entity email signal patterns
- `data/actions.json` — Portfolio-wide action completion rates

### Step 3: Pattern Detection

Analyze across entities for cross-cutting patterns:

**Health patterns:**
- How many entities are Green/Yellow/Red? What is the distribution?
- Is health trending in one direction across the portfolio?
- Are there clusters (e.g., all enterprise accounts improving, all mid-market declining)?

**Expansion patterns:**
- Which entities are showing expansion signals? What do they have in common?
- Is there a common trigger for expansion (feature adoption, champion engagement, executive alignment)?

**Churn/risk patterns:**
- Which entities are showing risk signals? What do they share?
- Is there a common root cause (product gap, competitor threat, engagement decline)?
- Are risks concentrated in a segment, tier, or owner's portfolio?

**Engagement patterns:**
- Where is meeting frequency increasing? Decreasing?
- Which entities have gone quiet (no meetings, no emails, no action activity)?
- Are there seasonal patterns visible?

**Action patterns:**
- What is the portfolio-wide action completion rate?
- Are overdue actions concentrated in certain entities or with certain owners?
- Are the same types of actions recurring (indicating systemic issues)?

**People patterns:**
- Are champions warming or cooling across the portfolio?
- Are there stakeholder gaps concentrated in a segment?
- Which relationships are the strongest drivers of entity health?

### Step 4: Elevate to Strategic Observations

Move from data patterns to strategic insights. Each observation should:
- Name the pattern specifically
- Quantify where possible
- Connect to root cause or driver
- Suggest strategic implication

**Example observations:**

- "3 of 4 enterprise accounts showing expansion signals have a dedicated CSM and monthly EBR cadence. The 2 enterprise accounts at risk have quarterly cadence. Cadence appears to be a leading indicator."
- "Competitive mentions increased 40% in Q1 across mid-market accounts. Datadog appears in 3 separate entity intelligence files. This is no longer an isolated competitive threat — it is a segment-wide pattern."
- "Action completion rate is 87% for accounts owned by Sarah, 52% for accounts owned by Marcus. The performance gap is driving the health distribution — Marcus's accounts are disproportionately Yellow."

### Step 5: Generate Synthesis

Structure the output:

```markdown
# Portfolio Synthesis
**Scope:** {description of scope}
**Period:** {timeframe}
**Date:** {today}
**Entities analyzed:** {count}

## Summary
{3-4 sentence overview of the portfolio state and the most important finding.}

### Portfolio Health
| Status | Count | Trend | Notable |
|---|---|---|---|
| Green | {N} | {stable/improving/declining} | {name any notable entities} |
| Yellow | {N} | {trend} | {notable} |
| Red | {N} | {trend} | {notable} |

**Total portfolio value:** {aggregate ARR/revenue if available}

## What's Working
{Cross-entity patterns that are driving success.}

### {Pattern 1 Title}
**Evidence:** {entities where this pattern is visible, with specifics}
**Implication:** {what this means strategically}
**Recommendation:** {how to amplify this}

### {Pattern 2 Title}
...

## Watch Items
{Cross-entity patterns that represent risk or concern.}

### {Pattern 1 Title}
**Evidence:** {entities affected, specific signals}
**Implication:** {what happens if this continues}
**Recommendation:** {specific mitigation}

### {Pattern 2 Title}
...

## Emerging Patterns
{New signals that are not yet trends but are worth monitoring.}

- {Signal} — Seen in {entities}. Too early to call a pattern, but watch for {indicator}.
- {Signal} — ...

## Strategic Recommendations
{Priority-ordered recommendations that address portfolio-level dynamics.}

1. **{Recommendation}** — Addresses {pattern}. Impact: {N} entities. Owner: {suggested}. Timeline: {when}.
2. **{Recommendation}** — ...
3. **{Recommendation}** — ...
```

### Step 6: Quality Check

Before presenting:
- Every pattern claim is backed by specific entity evidence (not "some accounts are declining" but "Nielsen, Datadog, and CoreLogic all moved from Green to Yellow in January")
- Recommendations are specific and actionable, not generic ("increase engagement")
- Emerging patterns are distinguished from established trends
- Data gaps are flagged ("3 entities have not had intelligence updates in 30+ days — synthesis may be incomplete for those")

### Step 7: Output and Loop-Back

Present the synthesis. Then offer:

```
Would you like me to:
1. Save this synthesis to data/portfolio-synthesis-2026-02.md
2. Create {N} actions from the strategic recommendations
3. Update intelligence.json for specific entities flagged in the watch items

Or adjust the analysis first?
```

**Quality exemplar approach — the Kai/search feedback synthesis pattern:**

When an individual signal (e.g., one person's feedback on search quality) appears across multiple entities and contexts, the synthesis should detect and name this as a systemic pattern rather than treating it as an isolated data point. The power of synthesize is seeing what is invisible at the individual entity level.

## Skills That Contribute

- **entity-intelligence** — Auto-fires for each entity in scope
- **role-vocabulary** — Shapes the health frame and vocabulary used in the synthesis
- **action-awareness** — Provides action completion data across the portfolio
- **relationship-context** — Informs people pattern detection
- **analytical-frameworks** — May activate when patterns require structured decomposition
- **loop-back** — Handles saving the synthesis and creating strategic actions
