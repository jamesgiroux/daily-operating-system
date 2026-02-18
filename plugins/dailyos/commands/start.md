---
description: "Initialize workspace fluency and surface today's priorities"
---

# /start

Initialize the DailyOS workspace, load the role preset, and surface what needs attention today.

## Arguments

No arguments required. This command auto-detects the workspace.

## Workflow

### Step 1: Detect DailyOS Workspace

Look for the DailyOS workspace markers at the current working directory or its parents:
- `data/` directory containing JSON schema files
- `Accounts/` and/or `Projects/` directories
- `People/` directory
- `_archive/` directory

If not found, inform the user: "This doesn't look like a DailyOS workspace. Expected to find data/, Accounts/ or Projects/, and People/ directories."

### Step 2: Read Workspace Manifest

Read `data/manifest.json` for workspace configuration:
- `workspace_name` — Display name for the workspace
- `role_preset` — Active role preset identifier
- `entity_mode` — Whether this workspace uses accounts, projects, or both
- `owner` — Workspace owner name

Load the role preset vocabulary (role-vocabulary skill activates).

### Step 3: Load Today's Schedule

Read `data/schedule.json` for today's calendar:
- Count total meetings
- Identify meeting types (QBR, customer call, one-on-one, etc.)
- Note any meetings with at-risk entities (cross-reference entity dashboard health)
- Flag meetings happening in the next 3 hours as immediate

### Step 4: Load Today's Prep

Read `data/prep.json` for prepared meeting intelligence:
- Note which meetings have prep generated and which do not
- Identify gaps where deep prep might be needed (QBRs, sensitive meetings)

### Step 5: Surface High-Priority Items

Scan for items requiring immediate attention:

1. **Overdue actions** — Read `data/actions.json`, find items where `due_date` is in the past and `status` is not "completed"
2. **At-risk entities with meetings today** — Cross-reference schedule with entity dashboards for Yellow/Red health
3. **Renewals this week** — Scan entity dashboards for `renewal_date` within the next 7 days
4. **Email signals** — Read `data/emails.json` for escalations or items flagged as `requires_action`
5. **Cooling relationships** — Check People/ profiles for temperature signals on today's meeting attendees

### Step 6: Output

Format the initialization message:

```
Ready. {Role preset} workspace loaded.

{N} meetings today{, including {high-priority meeting note if applicable}}.
{N} open actions ({N} overdue).
{Additional priority items if any}.
```

**If high-priority items exist**, suggest a specific next action:

```
Ready. Customer Success workspace loaded.

5 meetings today, including the Nielsen QBR at 2pm.
12 open actions (3 overdue).
Nielsen renewal is in 47 days with Yellow health.

Nielsen QBR in 3 hours with declining health. Want me to draft talking points or run an assessment first?
```

**If nothing urgent**, keep it clean:

```
Ready. Sales workspace loaded.

3 meetings today. 8 open actions, none overdue.
No urgent items. What would you like to work on?
```

### Step 7: Ready State

After initialization, the workspace-fluency skill is fully active. Entity-intelligence, relationship-context, and all other skills will auto-fire as entities and people are mentioned in subsequent conversation. The user can now invoke any command or ask freeform questions with full workspace context available.

## Loop-Back

This command does not produce a deliverable, so no loop-back is needed. It sets the stage for everything else.
