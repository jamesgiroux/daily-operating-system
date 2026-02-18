---
description: "Create time-bound strategies with milestones connected to the action system"
---

# /plan

Create a time-bound strategy with milestones, actions, and owners — all connected to the workspace action system. Plans are grounded in current entity intelligence and shaped by relationship context, not generated from templates.

## Arguments

- `$ARGUMENTS[0]` — Entity name (required). Resolved against Accounts/ and Projects/.
- `$ARGUMENTS[1]` — Plan type (optional). Auto-detected from role preset if omitted.

## Plan Types by Role Preset

| Preset | Default Plan Type | Focus |
|---|---|---|
| Customer Success | Renewal/Retention Plan | Path from current health to successful renewal |
| Sales | Deal Strategy | Close plan with stakeholder engagement sequence |
| Partnerships | Joint Business Plan | Mutual value delivery roadmap |
| Agency | Project Plan / Engagement Plan | Delivery milestones and client management |
| Consulting | Engagement Roadmap | Milestone delivery with stakeholder alignment |
| Product | Launch Plan / Adoption Plan | Feature rollout and success criteria |
| Leadership | Initiative Plan | Strategic execution with team accountability |
| The Desk | Auto-detect | Selects based on entity type and context |

## Workflow

### Step 1: Read Entity Intelligence

Resolve the entity. The entity-intelligence skill loads full context:
- `dashboard.json` — Current vitals, key dates, lifecycle stage
- `intelligence.json` — Risks, wins, current state, stakeholder insights
- `stakeholders.md` — Relationship map with engagement levels
- Filtered actions from `data/actions.json` — What has been promised, delivered, is overdue
- Recent `_archive/` meeting summaries — Trajectory and recent developments

### Step 2: Assess Current State

Before planning forward, establish where things stand:

**What is the current health/status?** Read from dashboard.json.

**What is the trajectory?** Compare recent meeting summaries — is the entity improving, stable, or declining?

**What has been tried?** Read the action trail. What actions were completed? What keeps appearing as overdue? What was promised and not delivered?

**Who are the key players?** Read stakeholders.md and load People/ profiles. Who is engaged? Who is disengaged? Who has power over the outcome?

**What are the constraints?** Key dates (renewal, contract end, milestone), budget cycles, stakeholder availability, competitive timelines.

### Step 3: Determine Planning Frame

Based on role preset and entity state:
- **Renewal/Retention Plan (CS):** X days to renewal, current health is Y, path from here to successful renewal
- **Deal Strategy (Sales):** Current stage, key milestones to close, stakeholder engagement sequence
- **Joint Business Plan (Partnerships):** Mutual commitments, shared milestones, value delivery timeline
- **Project Plan (Agency):** Deliverable milestones, client review points, resource allocation
- **Initiative Plan (Leadership):** Strategic milestones, team accountabilities, success metrics

### Step 4: Generate the Plan

Structure the plan with these sections:

```markdown
# {Entity Name} — {Plan Type}
**Created:** {today}
**Owner:** {workspace owner}
**Timeline:** {start date} to {end date}
**Key Date:** {the critical date driving this plan}

## Objective
{One clear statement of what success looks like at the end of this plan.}

## Current State Assessment
**Health:** {status} | **Trajectory:** {direction}

{2-3 sentences grounding the plan in reality. What is working, what is not, what is the gap between current state and objective.}

## Milestones

### Milestone 1: {Title} — by {date}
**Owner:** {name}
**Actions:**
- {Specific action} — {owner}, by {date}
- {Specific action} — {owner}, by {date}
**Success Criteria:** {How you know this milestone is achieved}
**Dependencies:** {What must happen first or in parallel}

### Milestone 2: {Title} — by {date}
**Owner:** {name}
**Actions:**
- {Specific action} — {owner}, by {date}
- {Specific action} — {owner}, by {date}
**Success Criteria:** {How you know}
**Dependencies:** {What must happen}

### Milestone 3: {Title} — by {date}
...

## Risk Mitigations
| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---|---|---|---|
| {Risk from intelligence.json or newly identified} | High/Med/Low | {Impact} | {Specific mitigation action} | {Name} |

## Stakeholder Engagement Plan
{How to engage key stakeholders throughout the plan timeline.}

| Stakeholder | Role | Engagement | Cadence | Key Message |
|---|---|---|---|---|
| {Name} | {Role} | {Current level} | {Planned frequency} | {What they need to hear} |

## Success Criteria
{How you will know the plan succeeded. Specific, measurable where possible.}
```

### Step 5: Connect to Reality

Ground every element in workspace evidence:

- Milestones should reference actual relationship context: "Based on last 3 meetings, Sarah responds well to data-driven presentations. The March EBR should lead with metrics rather than narrative."
- Risk mitigations should address real risks from intelligence.json, not hypothetical ones
- Stakeholder engagement should reflect actual temperature and preferences from People/ profiles
- Dates should account for known constraints (budget cycles, vacation patterns, other entity commitments)

### Step 6: Quality Check

Before presenting:
- Is every milestone achievable given current resources and relationships?
- Do the dates account for dependencies (milestone 2 cannot start before milestone 1 finishes)?
- Are owners identified for every action? If not, flag: "Owner TBD — needs assignment"
- Does the plan address the top risks from intelligence.json?
- Is the stakeholder engagement realistic given their current temperature and engagement level?

### Step 7: Output and Loop-Back

Present the plan. Then offer:

```
Would you like me to:
1. Save this plan to Accounts/Nielsen/renewal-plan-2026-q1.md
2. Create {N} actions in data/actions.json from the milestones
   - "Prepare adoption metrics deck for EBR" — You, due Feb 28
   - "Schedule executive alignment meeting with Elena" — You, due Mar 5
   - "Complete integration testing review" — Sarah Chen, due Mar 12
   - ... (list all)
3. Update Accounts/Nielsen/intelligence.json with the plan's risk mitigations

Create all, or adjust first?
```

Actions created from plans become trackable workspace items. When the user runs /assess on this entity later, the action trail will show whether the plan is being executed.

## Skills That Contribute

- **entity-intelligence** — Provides the full entity picture the plan is built on
- **relationship-context** — Informs stakeholder engagement strategy and communication approach
- **political-intelligence** — Enriches stakeholder strategy when dynamics are sensitive
- **action-awareness** — Provides the action trail (what has been tried) and receives new plan actions
- **analytical-frameworks** — May activate when the planning requires structured decomposition of options
- **role-vocabulary** — Shapes the plan type, terminology, and milestone framing
- **loop-back** — Handles saving the plan and creating trackable actions
