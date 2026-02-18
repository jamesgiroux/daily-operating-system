---
name: loop-back
description: "Routes deliverables back to the workspace — save, create actions, update intelligence — offer always, force never"
---

# Loop-Back

This skill fires when Claude produces a deliverable of any kind. It teaches where to save different output types, how to create actions from recommendations, how to update intelligence from new insights, and how to archive meeting outputs. The core convention: always offer, never force.

## Activation Triggers

Activate when any of the following are produced:
- An assessment, report, or analysis (from assess, decide, synthesize)
- A communication draft (from compose)
- A plan with milestones and actions (from plan)
- Meeting prep or post-meeting processing (from meeting-intelligence, capture)
- A deliverable document (from produce)
- Any enrichment of entity or person intelligence (from enrich)
- Any new insights that update the workspace picture

## The Loop-Back Convention

**Offer, never force.**

After producing a deliverable, present the user with specific options for what could be saved and where. Be precise about file paths. Let the user confirm, modify, or decline.

**Format:**
```
Would you like me to:
1. Save this report to Accounts/Nielsen/risk-assessment-2026-02.md
2. Create 3 actions in data/actions.json from the recommendations
3. Update Accounts/Nielsen/intelligence.json with the revised risk assessment

Or would you prefer to handle these manually?
```

Never silently write to workspace files. Never assume the user wants artifacts saved. Always ask.

## Routing Rules

### Entity Reports and Assessments
**Destination:** `{entity-path}/`
- Risk assessments, health checks, deal reviews save as named markdown files in the entity directory
- Filename convention: `{type}-{YYYY-MM}.md` (e.g., `risk-assessment-2026-02.md`)
- If the assessment updates the executive narrative, offer to update `intelligence.json`

### Entity Intelligence Updates
**Destination:** `{entity-path}/intelligence.json`
- When new risks are identified, offer to append to the risks array
- When new wins are identified, offer to append to the wins array
- When the executive assessment changes, offer to update it
- Always update `last_updated` timestamp when writing

### Person Intelligence Updates
**Destination:** `People/{name}/person.md`
- New meeting signals, temperature shifts, communication observations
- Append to the relevant section rather than overwriting
- Preserve the narrative arc — add to it, do not replace it

### Meeting Outputs
**Destination:** `_archive/YYYY-MM/`
- Meeting summaries, post-meeting debriefs, capture outputs
- Filename convention: `{meeting-title}-{YYYY-MM-DD}.md`
- Use the current month's directory; create it if it does not exist

### Meeting Prep
**Destination:** Typically not saved (consumed immediately), but offer if deep prep was generated
- If saving: `_archive/YYYY-MM/{meeting-title}-prep-{YYYY-MM-DD}.md`

### Plans and Strategies
**Destination:** `{entity-path}/`
- Renewal plans, deal strategies, engagement plans save in the entity directory
- Filename convention: `{plan-type}-{YYYY-MM}.md`

### Portfolio Syntheses
**Destination:** `data/` or workspace root
- Cross-entity syntheses that do not belong to a single entity
- Filename convention: `synthesis-{topic}-{YYYY-MM}.md`

### Communications
**Destination:** Typically not saved (sent via external tool), but offer the option
- If saving: `_archive/YYYY-MM/draft-{recipient}-{YYYY-MM-DD}.md`

### Actions
**Destination:** `data/actions.json`
- When recommendations include specific next steps, offer to create action items
- Each action needs: text, entity (if applicable), person (if applicable), due_date (if stated), status "open", source context
- Generate a unique id for each new action
- Append to the existing array — never overwrite

## Creating Actions from Recommendations

When a deliverable includes recommendations or next steps, parse them into trackable actions:

1. Extract the specific action from the recommendation text
2. Identify the entity from context
3. Identify the person/owner from context or ask
4. Extract or infer due date (if "next week" convert to specific date, if no date stated, leave as null)
5. Set source to the deliverable type and date
6. Present the proposed actions for confirmation:

```
I can create these actions from the assessment:

1. "Schedule EBR with Sarah to address adoption concerns" — Nielsen, Sarah Chen, due Feb 28
2. "Prepare competitive comparison deck for QBR" — Nielsen, due Mar 5
3. "Follow up on integration timeline commitment" — Nielsen, David Park, due Feb 21

Create all three, or adjust first?
```

## Updating Intelligence from New Insights

When analysis produces new understanding that changes the entity or person picture:

### Entity Intelligence
- If a new risk was identified, offer to add it to `intelligence.json` risks array
- If a risk was resolved, offer to move it to resolved/historical
- If the executive assessment narrative has changed, offer to update it
- If stakeholder dynamics shifted, offer to update `stakeholders.md`

### Person Intelligence
- If temperature changed (warming/cooling signal detected), offer to update `person.md`
- If new communication preferences were learned, offer to add them
- If meeting patterns shifted, offer to note the shift

## Archiving Meeting Outputs

After processing a meeting transcript or notes (via capture command):

1. Offer to save the summary to `_archive/YYYY-MM/`
2. Offer to create extracted actions in `data/actions.json`
3. Offer to update relevant People/ profiles with new signals
4. Offer to update entity intelligence if the meeting revealed new risks, wins, or state changes

Present all options together so the user can approve or modify the full set.

## Behavior Rules

1. **Be specific about paths.** Not "save to the workspace" but "save to Accounts/Nielsen/risk-assessment-2026-02.md"
2. **Batch related writes.** If an assessment produces a report, actions, and intelligence updates, present them all at once rather than asking three separate times.
3. **Respect the decline.** If the user says no, move on. Do not ask again.
4. **Create directories as needed.** If `_archive/2026-02/` does not exist, create it as part of the write.
5. **Preserve existing content.** When updating JSON files (actions, intelligence), read the current content, merge the new data, and write back. Never overwrite blindly.

## Interaction with Other Skills

- **workspace-fluency** provides the directory structure knowledge for routing
- **entity-intelligence** identifies which entity directory to target
- **relationship-context** identifies which People/ directory to target
- **action-awareness** manages the action creation and tracking
- All commands include loop-back as their final step
