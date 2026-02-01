# /week - Monday Review

Start the week with full context and a plan. Acts as your "chief of staff" for weekly planning.

## When to Use

Run Monday morning (or Sunday evening for prep). This command:
- **Prompts for your weekly priorities** (interactive)
- Shows all customer meetings this week
- Surfaces overdue and due-this-week action items
- Flags accounts needing attention (hygiene alerts)
- Pre-populates weekly impact template
- **Analyzes open time blocks and suggests task scheduling**
- **Creates calendar events for approved time blocks**

## Philosophy

`/week` does strategic planning for the whole week. `/today` handles tactical execution each day. Together they act as your admin assistant, keeping you on top of schedule and priorities.

## Three-Phase Execution

This command uses a three-phase approach for efficiency:

```
┌─────────────────────────────────────────────────────────────────┐
│                    THREE-PHASE COMMAND FLOW                     │
├─────────────────────────────────────────────────────────────────┤
│  Phase 1: PREPARATION (Python Script)                           │
│  • Calculate week dates, archive previous week                  │
│  • Fetch week's calendar, classify meetings                     │
│  • Fetch account data, check hygiene                            │
│  • Aggregate action items, identify time gaps                   │
│  • Output: _today/.week-directive.json                          │
│                                                                 │
│  Phase 2: AI ENRICHMENT (Claude)                                │
│  • Prompt for weekly priorities                                 │
│  • Generate meeting overview tables                             │
│  • Create agenda tasks for Foundation accounts                  │
│  • Summarize critical hygiene alerts                            │
│  • Propose time block schedule                                  │
│                                                                 │
│  Phase 3: DELIVERY (Python Script)                              │
│  • Write week-00 through week-04 files                          │
│  • Create weekly impact template                                │
│  • Optional: Create calendar events                             │
└─────────────────────────────────────────────────────────────────┘
```

## Execution Steps

### Phase 1: Run Preparation Script

**ALWAYS RUN THIS FIRST:**

```bash
python3 /Users/jamesgiroux/Documents/VIP/_tools/prepare_week.py
```

This script performs all deterministic operations:
- Calculates week number and Mon-Fri date range
- Archives previous week's `week-*` files to `archive/W[NN]/`
- Checks previous week's impact file status
- Fetches account data from Google Sheet
- Fetches 5-7 days of calendar events, filters to current week
- Classifies all meetings by type (customer, internal, project)
- Aggregates action items (overdue + due this week)
- Checks account hygiene (contact gaps, renewal dates, stale dashboards)
- Identifies time gaps for task scheduling

**Output:** `_today/.week-directive.json` containing structured data for Phase 2.

**Options:**
- `--skip-archive` - Don't archive previous week
- `--output FILE` - Custom output path

### Phase 2: AI Enrichment (Claude Tasks)

After the script completes, read the directive and execute AI tasks:

```bash
# Read the directive
cat /Users/jamesgiroux/Documents/VIP/_today/.week-directive.json
```

**Execute these AI tasks from directive['ai_tasks']:**

#### Priority Setting (`prompt_priorities`)

Use AskUserQuestion to prompt for weekly priorities:

```
Question 1: "What are your top 3 priorities this week?"
Header: "Priorities"
Options:
- "Customer meetings / account work"
- "Strategic project work (Agentforce, Bullseye, etc.)"
- "Administrative catch-up (actions, hygiene, emails)"
- "Other" (free text)
multiSelect: true
```

```
Question 2: "Any specific focus areas or constraints?"
Header: "Focus"
Options:
- "Deep work blocks needed"
- "Customer outreach push"
- "Deadline-driven (specify in notes)"
- "Light week - catch up mode"
multiSelect: false
```

#### Critical Alert Summary (`summarize_critical_alerts`)

For accounts with critical hygiene alerts, generate concise summaries:
- What's the risk?
- What action is needed?
- What's the deadline?

#### Time Block Approval (`prompt_time_blocks`)

Present suggested time blocks and get approval:

```
"I've created a suggested schedule based on your priorities:

| Day | Time | Task | Duration |
|-----|------|------|----------|
| Mon | 9:30-10:30 | [Task] | 60m |
| Tue | 2:00-2:30 | [Task] | 30m |

Which blocks would you like me to create?"

Options:
- Create all suggested calendar blocks
- Let me review and select which ones to create
- Just show me the suggestions, don't create events
- Skip scheduling this week
```

Store approved blocks for Phase 3 calendar creation.

### Phase 3: Run Delivery Script

**AFTER completing AI tasks:**

```bash
python3 /Users/jamesgiroux/Documents/VIP/_tools/deliver_week.py
```

This script:
- Writes week-00-overview.md (week at a glance)
- Writes week-01-customer-meetings.md (detailed customer list)
- Writes week-02-actions.md (overdue + this week)
- Writes week-03-hygiene-alerts.md (accounts needing attention)
- Writes week-04-focus.md (prioritized task list)
- Creates weekly impact template in Leadership/02-Performance/Weekly-Impact/
- Optionally creates calendar events for approved time blocks

**Options:**
- `--skip-calendar` - Don't create calendar events
- `--skip-inbox` - Don't move archives to inbox
- `--keep-directive` - Keep directive file for debugging
- `--ai-outputs FILE` - JSON with AI outputs (priorities, approved blocks)

**Tip:** To pass AI outputs (approved time blocks) to delivery:
```json
{
  "priorities": ["Customer meetings", "Strategic projects"],
  "focus": "Deep work blocks needed",
  "approved_time_blocks": ["Task 1", "Task 2"]
}
```

---

## Legacy Reference: Detailed Process

The following sections are reference material for Phase 2 AI enrichment.

### Step 1: Determine Week Number and Dates

```python
from datetime import datetime, timedelta

today = datetime.now()
week_number = today.isocalendar()[1]
year = today.year

# Get Monday-Friday of this week
monday = today - timedelta(days=today.weekday())
friday = monday + timedelta(days=4)
```

### Step 1B: Archive Previous Week Files

Before creating new week files, archive any existing `week-*` files from the previous week.

```bash
# Check if week-* files exist
WEEK_FILES=$(ls /Users/jamesgiroux/Documents/VIP/_today/week-*.md 2>/dev/null)

if [ -n "$WEEK_FILES" ]; then
    # Determine previous week number (for archive folder naming)
    # Use the date in the first week file, or calculate from current week
    PREV_WEEK=$(printf "W%02d" $(($(date +%V) - 1)))
    ARCHIVE_DIR="/Users/jamesgiroux/Documents/VIP/_today/archive/$PREV_WEEK"

    mkdir -p "$ARCHIVE_DIR"

    # Move all week-* files to archive
    for f in /Users/jamesgiroux/Documents/VIP/_today/week-*.md; do
        mv "$f" "$ARCHIVE_DIR/" 2>/dev/null
    done

    echo "Archived previous week files to $ARCHIVE_DIR"
fi
```

**Archive structure after /week runs:**
```
_today/
├── week-00-overview.md         # NEW - current week
├── week-01-customer-meetings.md
├── week-02-actions.md
├── week-03-hygiene-alerts.md
├── week-04-focus.md
├── tasks/                      # PERSISTENT - never archived
│   └── master-task-list.md
└── archive/
    ├── W03/                    # Previous week's week-* files
    │   ├── week-00-overview.md
    │   ├── week-01-customer-meetings.md
    │   └── ...
    ├── 2026-01-17/             # Daily archives
    ├── 2026-01-16/
    └── ...
```

**Note:** Week files are archived to a folder named by week number (e.g., `W03/`), while daily files are archived by date (e.g., `2026-01-17/`). This keeps them logically separated.

### Step 2: Priority Setting (Interactive)

**Use AskUserQuestion** to prompt for weekly priorities:

```
Question 1: "What are your top 3 priorities this week?"
Header: "Priorities"
Options:
- "Customer meetings / account work" (default selection possible)
- "Strategic project work (Agentforce, Bullseye, etc.)"
- "Administrative catch-up (actions, hygiene, emails)"
- "Other" (free text)
multiSelect: true
```

```
Question 2: "Any specific focus areas or constraints?"
Header: "Focus"
Options:
- "Deep work blocks needed"
- "Customer outreach push"
- "Deadline-driven (specify in notes)"
- "Light week - catch up mode"
multiSelect: false
```

Store responses for use in time block suggestions.

### Step 3: Fetch This Week's Calendar

```bash
python3 /Users/jamesgiroux/Documents/VIP/.config/google/google_api.py calendar list 5
```

Filter events to only Mon-Fri of current week.

### Step 4: Fetch Account Data from Google Sheet

```bash
python3 /Users/jamesgiroux/Documents/VIP/.config/google/google_api.py sheets get "1edLlG0rkPj9QRT5mWQmCh_L-qy4We9fBLJ4haMZ_14g" "A1:AB50"
```

Parse columns for account lookup:

| Column | Field | Usage |
|--------|-------|-------|
| A | Account | Display name |
| D | Lifecycle Ring | Hygiene thresholds, context |
| F | Last Engagement Date | Contact gap alerts |
| I | 2025 ARR | Display |
| P | Next Renewal Date | Renewal countdown alerts |
| X | Meeting Cadence | Compare expected vs actual |
| Y | Success Plan Exists | Hygiene check |
| Z | Success Plan Last Updated | Staleness check |
| AB | Email Domain | Domain → Account mapping |

Build lookup dictionary with all fields for each account.

### Step 5: Identify All Meetings This Week and Determine Prep Type

**All meetings get prep.** The difference is what *type* of prep:
- Customer meetings → Full prep (dashboard, history, actions, Clay intel)
- Internal meetings → Relationship prep (Clay intel, shared accounts, political context)
- Project meetings → Status prep (project state, partner updates)

For each event in the week:
1. Classify using domain mapping (same as /today)
2. For ALL meetings, note:
   - Date/time
   - Account/Meeting name
   - Ring or meeting category
   - Meeting type (extracted from calendar event title)
   - Prep status (determined by classification logic below)
   - Agenda owner (for customer meetings)

**Prep Status Logic:**

```python
def determine_prep_status(meeting, account_data, colleague_accounts):
    """
    Determine prep status for any meeting type.
    All meetings get prep - the type differs.

    Returns: (prep_status, agenda_owner, metadata)
    """
    meeting_type = meeting.get('type', 'unknown')
    title = meeting.get('summary', '').lower()
    attendees = meeting.get('attendees', [])

    # Check for strategic meeting signals (override ring-based logic)
    agenda_signals = ['renewal', 'ebr', 'qbr', 'strategic review', 'quarterly']
    is_strategic = any(signal in title for signal in agenda_signals)

    if meeting_type == 'customer':
        ring = account_data.get('ring', 'Foundation')

        # Foundation or strategic meetings = you own agenda
        if ring == 'Foundation' or is_strategic:
            return '📅 Agenda needed', 'you', {'ring': ring}
        else:
            # Evolution, Influence, Summit - customer typically drives
            return '📋 Prep needed', 'customer', {'ring': ring}

    elif meeting_type == 'project':
        return '🔄 Bring updates', 'shared', {}

    elif meeting_type == 'internal':
        # Check if attendee shares accounts with you
        shared_accounts = find_shared_accounts(attendees, colleague_accounts)
        if shared_accounts:
            # 1:1 with someone who shares accounts - richer prep
            return '👥 Context needed', None, {'shared_accounts': shared_accounts}
        else:
            return '👥 Context needed', None, {}

    else:  # external, personal, unknown
        return '👥 Context needed', None, {}
```

**Prep Status Types:**

| Meeting Type | Ring/Category | Prep Status | Agenda Owner | What Prep Includes |
|--------------|---------------|-------------|--------------|-------------------|
| Customer | Foundation | `📅 Agenda needed` | You | Full prep + agenda draft for you to send |
| Customer | Evolution, Influence, Summit | `📋 Prep needed` | Customer | Dashboard, history, actions, attendee intel |
| Customer | Any ring + strategic signals | `📅 Agenda needed` | You | Full prep + agenda draft (EBR, QBR, renewal) |
| Project | (Agentforce, etc.) | `🔄 Bring updates` | Shared | Project status, partner updates, blockers |
| Internal | 1:1s, team syncs | `👥 Context needed` | N/A | Clay intel, shared accounts, political context |

**Override Signals (upgrade to "📅 Agenda needed"):**
- Calendar description contains "agenda" + your name
- Meeting title contains "renewal", "EBR", "QBR", "strategic review"

**Agenda Task Creation:**
When prep status is "📅 Agenda needed", create a task in master-task-list.md:

```python
for meeting in customer_meetings:
    if meeting['agenda_owner'] == 'you':
        create_agenda_task(
            account=meeting['account'],
            due=meeting['date'] - timedelta(days=1),  # Day before
            meeting_date=meeting['date'],
            meeting_time=meeting['time']
        )
```

**Task format:**
```markdown
- [ ] **Send agenda: [Account] [meeting type]** `[date]-agenda-001`
  - Account: [Account]
  - Due: [1 day before meeting]
  - Owner: James
  - Source: /week W[NN] planning
  - Meeting: [date time]
  - Draft: _today/90-agenda-needed/[account-lowercase]-[date].md
  - Priority: P2
```

### Step 6: Aggregate Action Items

Scan all account action files:

```
Glob: Accounts/*/04-Action-Items/*.md
Grep: "- [ ]" (unchecked items)
```

Categorize by:
- **Overdue**: Past due date
- **Due This Week**: Due Mon-Fri of current week
- **Related to This Week's Meetings**: Actions for accounts with meetings

### Step 7: Check Account Hygiene

**Reference**: `.claude/skills/daily-csm/PORTFOLIO-HEALTH.md`
**Reference**: `.claude/skills/daily-csm/HEALTH-SIGNALS.md` (ring thresholds)
**Reference**: `.claude/skills/daily-csm/RENEWAL-COUNTDOWN.md` (renewal phases)

Follow the PORTFOLIO-HEALTH "Weekly Triage Routine":

```
Glob: Accounts/*/01-Customer-Information/*-dashboard.md
```

**Primary checks** (from HEALTH-SIGNALS.md):

| Check | Source | Threshold | Alert Level |
|-------|--------|-----------|-------------|
| **Stale Dashboard** | Dashboard "Last Updated" | >60 days | Medium |
| **No Recent Contact** | Sheet Column F | Varies by ring* | High |
| **Upcoming Renewal** | Sheet Column P | <4 months | Critical |
| **Overdue Actions** | 04-Action-Items/*.md | Any overdue | High |
| **Open Risks** | Dashboard "Risks" section | Unchecked | High |

**Additional checks** (from Sheet Columns X, Y, Z):

| Check | Source | Condition | Alert Level |
|-------|--------|-----------|-------------|
| **Meeting Cadence Gap** | Column X + Calendar | Expected cadence not met | Medium |
| **Success Plan Missing** | Column Y | Ring ≥ Evolution + No plan | High |
| **Success Plan Stale** | Column Z | Exists but >90 days old | Medium |

*Ring-based contact thresholds (from HEALTH-SIGNALS.md):
| Ring | Contact Threshold | Dashboard Refresh |
|------|-------------------|-------------------|
| Foundation | 90+ days | Quarterly |
| Evolution | 45+ days | Monthly |
| Influence | 30+ days | Monthly |
| Summit | 14+ days | Bi-weekly |

**Renewal phase alerts** (from RENEWAL-COUNTDOWN.md):

| Months Out | Alert | Required Action |
|------------|-------|-----------------|
| 6 | Inform | 6-month renewal assessment due |
| 4 | Warn | EBR planning required |
| 3 | Critical | RM alignment needed |

### Step 8: Check Previous Week's Impact

Before creating new impact template, check for continuity:

```bash
# Find previous week's impact file
ls Leadership/06-Professional-Development/01-Weekly-Impact/ | grep -E "$(date -v-7d +%Y)-W$(printf '%02d' $(($(date -v-7d +%V))))"
```

**If previous week exists:**
- Note: "✅ Previous week (W[X]) impact captured"
- Extract "Action Items for Next Week" section if present
- Carry forward incomplete items to new template

**If previous week missing:**
- Add warning to week-00-overview.md: "⚠️ Previous week (W[X]) impact not captured"
- Note this gap for pattern awareness

**If previous week is draft status:**
- Add reminder: "Previous week (W[X]) impact still in draft - finalize"

### Step 9: Pre-Populate Weekly Impact Template

**Reference**: `.claude/skills/daily-csm/IMPACT-REPORTING.md` (category definitions)
**Reference**: `_templates/weekly-impact-template.md` (format)

Create `Leadership/06-Professional-Development/01-Weekly-Impact/[YYYY]-W[NN]-impact-capture.md`:

```markdown
---
area: Leadership
doc_type: impact
status: draft
date: [YYYY-MM-DD]
week: W[NN]
tags: [impact, weekly, [year]]
privacy: internal
---

# Weekly Impact Capture - W[NN] ([Month Day-Day, Year])

## Customer Meetings This Week

| Day | Account | Meeting Type | Outcome |
|-----|---------|--------------|---------|
| Mon | | | |
| Tue | [Account] | Monthly Sync | |
| Wed | | | |
| Thu | [Account] | Technical Review | |
| Fri | | | |

## Value Delivered (Pillar 1)

### Customer Wins
-

### Technical Outcomes
-

## Relationship Progress (Pillar 2)

### Stakeholder Engagement
-

### Executive Access
-

## Expansion Progress (Pillar 3)

### Opportunities Identified
-

### Pipeline Movement
-

## Risk Management (Pillar 4)

### Issues Resolved
-

### Risks Mitigated
-

## Cross-Functional Contributions

-

## Key Learnings

-

---
*To be completed throughout the week and finalized by [Friday date]*
```

### Step 10: Time Block Analysis and Task Scheduling

This is the core planning step. Analyze open time, match tasks to slots, and create calendar blocks.

#### 10.1: Identify Open Time Blocks

Parse calendar events and find gaps:

```python
# For each day Mon-Fri:
# 1. Mark all existing events as "blocked"
# 2. Identify gaps of 30+ minutes
# 3. Classify time periods:
#    - Morning (9am-12pm): Complex/thought-heavy work
#    - Afternoon (1pm-5pm): Straightforward/execution work

# Output: List of available slots with classification
available_slots = [
    {"day": "Monday", "start": "09:30", "end": "11:00", "type": "morning", "duration": 90},
    {"day": "Monday", "start": "14:00", "end": "15:30", "type": "afternoon", "duration": 90},
    # etc.
]
```

#### 10.2: Task Effort Estimation

For each task from master-task-list.md and action items, estimate effort:

| Size | Duration | Examples |
|------|----------|----------|
| **Small** | 15 min | Send email, quick Slack, file update |
| **Medium** | 30 min | Draft agenda, review document, update dashboard |
| **Large** | 60 min | Write proposal, deep analysis, customer prep |

Tasks are sized based on:
- P1 tasks → typically Medium or Large
- P2 tasks → typically Small or Medium
- P3 tasks → typically Small

#### 10.3: Time Preference Matching

Match tasks to slots based on complexity:

| Task Type | Preferred Time | Reason |
|-----------|----------------|--------|
| Strategic thinking, writing | Morning (9-12) | Best focus, James's preference |
| Customer prep, analysis | Morning | Complex, needs focus |
| Email catch-up, Slack | Afternoon | Straightforward execution |
| Dashboard updates | Afternoon | Routine, low cognitive load |
| Action item follow-ups | Afternoon | Quick wins, clearing queue |

#### 10.4: Generate Suggested Schedule

Create a proposed time-blocked schedule:

```markdown
## Suggested Time Blocks

### Monday
| Time | Duration | Task | Priority | Type |
|------|----------|------|----------|------|
| 9:30-10:30 | 60m | Nielsen docs (overdue) | P2 | Large - Morning |
| 14:00-14:30 | 30m | Email Joanna re: Compliance | P2 | Medium - Afternoon |
| 14:30-15:00 | 30m | Dashboard refresh: Crowley | P3 | Medium - Afternoon |

### Tuesday
| Time | Duration | Task | Priority | Type |
|------|----------|------|----------|------|
| 9:00-10:00 | 60m | Agentforce timeline research | P2 | Large - Morning |
[etc.]
```

#### 10.5: User Approval Gate

**Use AskUserQuestion** to confirm the schedule:

```
Question: "I've created a suggested schedule based on your priorities and available time. Would you like me to:"
Header: "Schedule"
Options:
- "Create all suggested calendar blocks"
- "Let me review and select which ones to create"
- "Just show me the suggestions, don't create events"
- "Skip scheduling this week"
multiSelect: false
```

If "Let me review": Present each block for individual approval.

#### 10.6: Create Calendar Events

For approved blocks, create calendar events:

```bash
# Event naming convention: [Task Type] [Brief Description]
# Examples:
# "Focus: Nielsen Documentation"
# "Admin: Dashboard Updates"
# "Prep: Cox Parse.ly Demo"

python3 /Users/jamesgiroux/Documents/VIP/.config/google/google_api.py calendar create \
  "[Task Type]: [Description]" \
  "2026-01-12T09:30:00-05:00" \
  "2026-01-12T10:30:00-05:00" \
  "Task from master-task-list: [full description]. Source: /week planning."
```

**Event categories** (for naming):
- `Focus:` - Deep work, writing, strategic thinking
- `Prep:` - Customer meeting preparation
- `Admin:` - Administrative tasks, follow-ups
- `Review:` - Dashboard updates, document reviews

### Step 11: Generate Week Overview

Create `_today/week-00-overview.md`:

```markdown
# Week Overview: W[NN] - [Month Day-Day, Year]

## This Week's Meetings

| Day | Time | Account/Meeting | Ring | Prep Status | Meeting Type |
|-----|------|-----------------|------|-------------|--------------|
| Tue | 10:30 AM | Heroku | Influence | 📋 Prep needed | Monthly sync |
| Tue | 11:30 AM | Hilton | Foundation | 📅 Agenda needed | Contract renewal |
| Tue | 1:00 PM | Renan 1:1 | Internal | 👥 Context needed | 1:1 |
| Wed | 10:00 AM | Agentforce | Project | 🔄 Bring updates | Weekly sync |
| Wed | 12:00 PM | Airbnb | Foundation | 📅 Agenda needed | Strategic check-in |
| Thu | 9:00 AM | Salesforce DMT | Summit | 📋 Prep needed | Technical call |

**Prep Status Guide:**

| Icon | Status | Meaning | Next Step |
|------|--------|---------|-----------|
| `📋 Prep needed` | Initial | Customer meeting prep required | /today generates prep file |
| `📅 Agenda needed` | Initial | You own agenda, need to create and send | /today creates draft, you review and send |
| `🔄 Bring updates` | Initial | Project meeting, come with status | Review project state before meeting |
| `👥 Context needed` | Initial | Internal meeting, relationship prep | /today adds Clay intel, shared accounts |
| `✅ Prep ready` | Progress | Prep file generated | Review before meeting |
| `✏️ Draft ready` | Progress | Agenda draft created | Review, customize, and send |
| `✅ Done` | Complete | Meeting completed | /wrap marks as done |

## Action Items Summary

### Overdue ([count])
- [ ] **[Action]** - [Account] - Due: [Date] (X days overdue)

### Due This Week ([count])
- [ ] **[Action]** - [Account] - Due: [Day]

### Related to This Week's Meetings
- [ ] **[Action]** - [Account] - Due: [Date]

## Account Hygiene Alerts

### 🔴 Critical
- **[Account]** - Renewal in [X] months, [concern]

### 🟡 Needs Attention
- **[Account]** - Dashboard stale ([X] days)
- **[Account]** - No contact in [X] days ([Ring] account)

### 🟢 Healthy
- [X] accounts with no alerts

## Weekly Impact Template

Pre-populated template created:
`Leadership/06-Professional-Development/01-Weekly-Impact/[YYYY]-W[NN]-impact-capture.md`

**Reminder**: Capture impacts throughout the week, not Friday afternoon.

## Suggested Calendar Blocks

| Block | When | Purpose |
|-------|------|---------|
| Agenda Prep | [Day] AM | Send agenda for [Account] ([meeting day]) |
| Dashboard Refresh | [Day] | Update [Account] dashboard (stale) |
| Impact Capture | Wed/Thu | Mid-week impact notes |
| Action Follow-up | [Day] | Address [X] overdue items |

*Use AskUserQuestion to confirm before creating calendar events*

## Suggested Weekly Priorities

1. **Customer Prep**: Ensure agendas sent for all meetings
2. **Overdue Items**: Address [X] overdue actions
3. **Hygiene**: Refresh [X] stale dashboards
4. **Impact**: Keep impact template updated daily
```

### Step 12: Generate Supporting Files

Create additional files in `_today/`:

**`week-01-customer-meetings.md`**:
```markdown
# Customer Meetings - W[NN]

## [Day]: [Account] - [Meeting Title]

**Time**: [Time]
**Ring**: [Ring]
**ARR**: $[Amount]
**Renewal**: [Date]

**Meeting Type**: [Type from calendar event]
**Prep Status**: [⚠️ Needs preparation | Come with updates | No prep required]

**Context**:
- Last meeting: [Date] - [Topic]
- Open actions: [Count]

---
[Repeat for each customer meeting]
```

**`week-02-actions.md`**:
```markdown
# Action Items - W[NN]

## Overdue

### [Account]
- [ ] **[Action]** - [Owner] - Due: [Date] (X days overdue)
  - Source: [file]

## Due This Week

### Monday
- [ ] **[Action]** - [Account] - [Owner]

### Tuesday
[etc.]

## No Due Date (Review Needed)
- [ ] **[Action]** - [Account] - [Owner]
```

**`week-03-hygiene-alerts.md`**:
```markdown
# Account Hygiene Alerts - W[NN]

## Critical (Act This Week)

### [Account]
- **Issue**: Renewal in [X] months
- **Ring**: [Ring]
- **Last Contact**: [Date]
- **Action**: Schedule renewal planning with RM

## Needs Attention

### [Account]
- **Issue**: Dashboard stale ([X] days)
- **Ring**: [Ring]
- **Action**: Refresh before next customer interaction

### [Account]
- **Issue**: No contact in [X] days
- **Ring**: [Ring] (threshold: [X] days)
- **Action**: Schedule touchpoint this week

## Healthy Accounts

| Account | Ring | Last Contact | Dashboard Updated |
|---------|------|--------------|-------------------|
| [Account] | Foundation | [Date] | [Date] |
```

**`week-04-focus.md`**:
```markdown
# Weekly Focus Priorities - W[NN]

## Must Do This Week

1. [ ] Send agendas for all customer meetings
2. [ ] Address [X] overdue action items
3. [ ] Prep for [Account] renewal conversation ([X] months out)

## Should Do This Week

4. [ ] Refresh [Account] dashboard (stale [X] days)
5. [ ] Schedule touchpoint with [Account] (no contact [X] days)
6. [ ] Update weekly impact template (don't wait for Friday)

## Could Do This Week

7. [ ] Review [Account] success plan progress
8. [ ] Clean up action items without due dates

## Time Allocation Intent

| Category | Hours | Notes |
|----------|-------|-------|
| Customer Meetings | [X] | [X] meetings scheduled |
| Meeting Prep | [X] | ~30 min per customer meeting |
| Administrative | [X] | Actions, hygiene, impact capture |
| Focus Work | [X] | Deep work on [priority] |
```

### Step 13: Process Weekly Archives to Inbox

Move the week's archived daily files to `_inbox/` for canonical processing:

```bash
# Move all archived days to inbox for processing
if [ -d "/Users/jamesgiroux/Documents/VIP/_today/archive" ]; then
    for day_dir in /Users/jamesgiroux/Documents/VIP/_today/archive/*/; do
        if [ -d "$day_dir" ]; then
            day_name=$(basename "$day_dir")

            # Create inbox folder for this day's files
            mkdir -p "/Users/jamesgiroux/Documents/VIP/_inbox/daily-archive-$day_name"

            # Move all files
            mv "$day_dir"* "/Users/jamesgiroux/Documents/VIP/_inbox/daily-archive-$day_name/" 2>/dev/null

            # Remove empty archive folder
            rmdir "$day_dir" 2>/dev/null
        fi
    done
fi
```

**What gets processed:**
- Customer prep files → `Accounts/[Account]/02-Meetings/YYYY-MM-DD-prep-[account].md`
- Project prep files → `Projects/[Project]/02-Meetings/...`
- Action summaries → Reference for updating `Accounts/*/04-Action-Items/`
- Overview files → Can be discarded (ephemeral)

**Inbox processing determines canonical location based on:**
- File naming convention (e.g., `customer-nielsen-prep.md` → Nielsen account)
- Frontmatter if present
- Content analysis

**This completes the lifecycle:**
```
/today creates → _today/*.md
/today archives → _today/archive/YYYY-MM-DD/
/week references → archive for context
/week moves → _inbox/daily-archive-YYYY-MM-DD/
inbox-processing → canonical locations in Accounts/, Projects/
```

### Step 14: Monday Coordination

If today is Monday, coordinate with /today:

**Option A: Prompt for /today**
```
After generating week files, ask:
"Week overview complete. Today is Monday - would you like me to run /today
to generate today's meeting prep files?"
```

**Option B: Include Monday prep inline**
For Monday customer meetings, generate full prep in `week-05-monday-prep.md`:
- Follow the same CUSTOMER MEETING prep format as /today
- Pull from account dashboard (PRIMARY source)
- Include attendees, risks, wins, talking points
- This avoids requiring two commands on Monday morning

**Recommended**: Use Option A (prompt) to keep commands focused and avoid duplicate work.

## Output Structure

After running `/week`:

```
_today/
├── week-00-overview.md             # Week at a glance
├── week-01-customer-meetings.md    # All customer meetings
├── week-02-actions.md              # Action items for the week
├── week-03-hygiene-alerts.md       # Accounts needing attention
├── week-04-focus.md                # Suggested priorities
└── [daily files if /today also run]

Leadership/06-Professional-Development/01-Weekly-Impact/
└── [YYYY]-W[NN]-impact-capture.md  # Pre-populated impact template
```

## Dependencies

**APIs:**
- Google Calendar (full access - read and write)
- Google Sheets (read)

**Skills/Workflows:**
- daily-csm/PORTFOLIO-HEALTH.md - Hygiene checking
- daily-csm/IMPACT-REPORTING.md - Impact template

**Templates:**
- _templates/weekly-impact-template.md

**Data Sources:**
- Google Sheet: `1edLlG0rkPj9QRT5mWQmCh_L-qy4We9fBLJ4haMZ_14g`
- Accounts/*/01-Customer-Information/*-dashboard.md
- Accounts/*/04-Action-Items/*.md
- _today/tasks/master-task-list.md

## Task Sizing Reference

When estimating task effort for time blocking:

| Size | Duration | Typical Tasks |
|------|----------|---------------|
| **Small** | 15 min | Quick email, Slack message, file rename, simple update |
| **Medium** | 30 min | Draft agenda, update dashboard, review document, follow-up email |
| **Large** | 60 min | Customer prep, write proposal, strategic analysis, deep research |

## Time Preference Reference

| Time of Day | Best For | Avoid |
|-------------|----------|-------|
| **Morning (9am-12pm)** | Complex thinking, writing, customer prep, strategic work | Routine admin |
| **Afternoon (1pm-5pm)** | Execution tasks, emails, dashboard updates, quick wins | Deep focus work |

## Related Commands

- `/today` - Daily operating system (run this first on Monday)
- `/month` - Monthly roll-up (first Monday of month)
- `/quarter` - Quarterly pre-population
