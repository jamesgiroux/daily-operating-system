# /today - Daily Operating System

Populate `_today/` with everything needed for today's work.

## When to Use

Run every morning during your "Daily Prep" time block. This command:
- Preps you for all meetings
- Surfaces action items due today
- Generates draft agendas for upcoming meetings
- Suggests focus areas for downtime

## Philosophy

**Value shows up without asking.** The system does work before you arrive.

**Skip a day, nothing breaks.** Each run rebuilds fresh - no accumulated guilt from missed days.

## Execution Steps

### Step 0: Resilience Checks

Before starting, verify yesterday's closure and catch issues:

```python
from datetime import datetime, timedelta
import os
import glob

yesterday = (datetime.now() - timedelta(days=1)).strftime('%Y-%m-%d')
warnings = []

# 1. Check if /wrap ran yesterday (archive should exist)
archive_path = f"_today/archive/{yesterday}"
if not os.path.exists(archive_path):
    warnings.append(f"Yesterday's files not archived - /wrap may not have run")

# 2. Check for unprocessed transcripts from yesterday
yesterday_transcripts = glob.glob(f"_inbox/*{yesterday}*transcript*.md")
if yesterday_transcripts:
    warnings.append(f"{len(yesterday_transcripts)} transcripts from yesterday not processed")

# 3. Check for action items due yesterday still open
overdue_from_yesterday = check_overdue_actions(yesterday)
if overdue_from_yesterday:
    warnings.append(f"{len(overdue_from_yesterday)} action items due yesterday still open")

# 4. Check if master task list exists
if not os.path.exists("_today/tasks/master-task-list.md"):
    warnings.append("Master task list not found - will create")
```

**Display warnings in overview if any exist:**
```markdown
## Attention Needed

- Yesterday's files not archived - running cleanup now
- 2 transcripts from yesterday not processed

**Suggested:** Run `/wrap` to reconcile, or address items manually.
```

**Auto-remediation:**
- If archive missing: Run archive step for yesterday before proceeding
- If task list missing: Create from template
- Other issues: Surface warnings but continue (don't block)

### Step 1: Archive Yesterday and Clear _today/

**Archive lifecycle:**
- Daily files stay in `_today/archive/YYYY-MM-DD/` during the week
- Week files (`week-*`) persist all week until next /week run
- /week processes and moves archives to `_inbox/` for canonical filing
- This provides fast 7-day access + long-term searchability

```bash
# Get yesterday's date
YESTERDAY=$(date -v-1d +%Y-%m-%d)  # macOS
# YESTERDAY=$(date -d "yesterday" +%Y-%m-%d)  # Linux

# Archive yesterday's content (if exists)
if [ -f "_today/00-overview.md" ]; then
    mkdir -p _today/archive/$YESTERDAY

    # Move all daily files EXCEPT week-* files
    for f in _today/*.md; do
        filename=$(basename "$f")
        if [[ ! "$filename" == week-* ]]; then
            mv "$f" _today/archive/$YESTERDAY/ 2>/dev/null
        fi
    done

    # Move agenda-needed drafts if any
    if [ -d "_today/90-agenda-needed" ] && [ "$(ls -A _today/90-agenda-needed/ 2>/dev/null)" ]; then
        mkdir -p _today/archive/$YESTERDAY/90-agenda-needed
        mv _today/90-agenda-needed/*.md _today/archive/$YESTERDAY/90-agenda-needed/ 2>/dev/null
    fi
fi

# Create fresh structure for today (tasks/ persists - don't recreate)
mkdir -p _today/90-agenda-needed
mkdir -p _today/tasks
```

**IMPORTANT:**
- The `tasks/` directory is NEVER archived - it persists across days.
- The `week-*` files are NEVER archived by /today - they persist until /week archives them.

### Step 1.5: Process Inbox (Clear Yesterday's Documents)

Before building today's dashboard, clear any documents from `_inbox/`:

```python
# Check if inbox has files to process
inbox_files = glob.glob("_inbox/*.md")
inbox_files = [f for f in inbox_files if not os.path.basename(f).startswith('.')]

if inbox_files:
    print(f"Found {len(inbox_files)} files in _inbox/ to process")
    # Invoke /inbox skill to process all files
    # This enriches transcripts, summaries, and routes to PARA locations
```

**Why process inbox during /today?**
- Yesterday's meeting transcripts become today's context
- Action items extracted from transcripts appear in today's actions
- Account dashboards stay current with latest meeting data
- Zero accumulated backlog - fresh start each day

**Integration with /inbox skill:**
```
1. Run Phase 1: python3 _tools/prepare_inbox.py
2. Execute Phase 2: Claude enrichment (summaries, actions, tags)
3. Run Phase 3: python3 _tools/deliver_inbox.py
```

**If inbox empty:** Skip this step and proceed.

**If processing fails:**
- Log warning in overview
- Continue with rest of /today
- Surface in "Attention Needed" section

### Step 2: Fetch Account Data (Optional)

If you have a Google Sheet with account data, fetch it:

```bash
python3 .config/google/google_api.py sheets get "YOUR_SHEET_ID" "A1:Z50"
```

Configure columns in your CLAUDE.md to map:
- Account name
- Email domain (for meeting classification)
- Key metrics (ARR, renewal date, etc.)
- Contact frequency expectations

**If no account sheet configured:** Skip this step, classify meetings based on attendee domains.

### Step 3: Fetch Today's Calendar

```bash
python3 .config/google/google_api.py calendar list 1
```

Parse JSON output. For each event extract:
- `id`: Event ID
- `summary`: Meeting title
- `start`: Start time
- `end`: End time
- `attendees`: List of email addresses

**Filter out declined events** by checking responseStatus.

### Step 3.5: Scan Email Inbox (Optional)

If Gmail API is configured:

```bash
python3 .config/google/google_api.py gmail search "is:unread in:inbox" 30
```

**Classification:**

| Priority | Criteria | Action |
|----------|----------|--------|
| **HIGH** | From customer/client domain, from leadership, action words in subject | Surface in overview with full summary |
| **MEDIUM** | Internal colleagues, meeting-related | Note count |
| **LOW** | Newsletters, automated notifications | Archive automatically |

Create `83-email-summary.md` with HIGH priority email summaries.

### Step 4: Classify Each Meeting

```
STEP 1: Check for known PROJECTS first

IF meeting title contains known project name:
    type = "project"
    → Generate project meeting prep

STEP 2: If not a project, classify by attendees

IF no attendees OR only you:
    type = "personal"

ELSE IF all attendees are internal (your organization's domain):
    type = "internal"

ELSE (external attendees present):
    Match external domains to accounts (if account data available)
    type = "customer" or "external"
```

### Step 5: Generate Meeting Files (Numbered by Time)

Create files in chronological order with naming convention:
`[NN]-[HHMM]-[type]-[name].md`

**File numbering:**
- `00` = overview (always first)
- `01-79` = meetings in chronological order
- `80-89` = reference documents (actions, focus, email summary)
- `90-99` = action-needed items (agendas)

**Time-aware behavior:**

```python
from datetime import datetime

current_time = datetime.now()

def get_meeting_status(meeting_start, meeting_end):
    if current_time > meeting_end:
        return 'past'
    elif current_time >= meeting_start:
        return 'in_progress'
    else:
        return 'upcoming'
```

| Status | Icon | Action |
|--------|------|--------|
| Past | Done | Skip prep generation, minimal file |
| In Progress | Active | Link to existing prep if available |
| Upcoming | Pending | Generate full prep |

#### For Customer/Client Meetings

Generate comprehensive prep by reading from account documentation:

```markdown
# [Account] Call Prep
**[Date] | [Meeting Title]**

## Quick Context

| Metric | Value |
|--------|-------|
| **Status** | [Account status] |
| **Key Metric** | [e.g., ARR, contract value] |
| **Next Milestone** | [Date] |
| **Last Contact** | [Date] - [Topic] |

## Attendees

| Name | Role | Notes |
|------|------|-------|
| [Name] | [Role] | [Context] |

## Since Last Meeting

[Summary from most recent meeting notes]

## Open Action Items

- [ ] **[Action]** - Owner: [name] - Due: [date]
  - **Context**: [Why this action exists]
  - **Source**: [file path]

## Suggested Talking Points

1. **Follow up on**: [from open actions]
2. **Check in on**: [from recent discussions]
3. **Explore**: [opportunity or concern]

## Questions to Ask

- [Discovery question]
- [Follow-up from previous discussion]

## Key References
| Document | Path | Last Updated |
|----------|------|--------------|
| Account Dashboard | `path/to/dashboard.md` | [Date] |
| Last Meeting | `path/to/meeting.md` | [Date] |
```

#### For Project Meetings

```markdown
# [Project] Sync
**[Date] | [Meeting Title]**

## Project Context
- **Project**: [Project Name]
- **Status**: [Current status]
- **Partners**: [List partners if applicable]

## Attendees
- [Name] ([Organization]) - [Role in project]

## Recent Activity
[Summary from recent project files]

## Open Items
- [ ] [Item from project tracking]

## Discussion Topics
1. [Topic from recent activity]
2. [Blocker or decision needed]
```

#### For Internal Meetings

Internal meetings still deserve prep, just with a different focus on relationship context and shared work.

**1:1 meetings with colleagues you work closely with:**

```markdown
# [Colleague] 1:1 Prep
**[Time] | Internal**

## Relationship Context
*(From CRM or recent interactions)*

| Metric | Value |
|--------|-------|
| **Last Interaction** | [Date] |
| **Working Together On** | [Shared projects/accounts] |
| **Recent Notes** | [Brief context] |

## Shared Work Status
*(Projects or accounts you both touch)*

| Project/Account | Your Actions | Their Actions | Status |
|-----------------|--------------|---------------|--------|
| [Shared item] | [Your pending items] | [Their pending items] | [Active/On hold] |

## Pre-Read Check

- [ ] Pre-read shared in calendar invite?
- [ ] Linked doc needs review?

## Potential Topics

Based on shared context:
1. [Shared work updates]
2. [Coordination needs]
3. [Feedback or asks]
4. [Open questions]

## Notes

```

**Team syncs and larger group meetings:**

```markdown
# [Meeting Title]
**[Time] | Internal**

## Attendees
- [List from calendar]

## Your Updates to Share

**Progress/Wins:**
- [Recent accomplishments worth sharing]
- [Milestones reached]

**Blockers/Needs:**
- [Where you need input or support]
- [Dependencies on others]

**FYIs:**
- [Announcements or updates]

## Notes

```

#### For Personal Meetings

Generate minimal placeholder:

```markdown
# [Meeting Title]
**[Time] | Personal**

## Notes

```

### Step 5B: Update Week Overview with Prep Status

After generating meeting files, update week-00-overview.md to reflect prep progress.

**Key principle:** This happens automatically. User sees updated status without doing anything.

```python
def update_week_prep_status(today_date):
    """
    Update week overview to reflect prep files generated today.
    If week overview doesn't exist, create a minimal one.
    """
    week_overview_path = '_today/week-00-overview.md'

    # Fallback: Create minimal week overview if /week wasn't run
    if not os.path.exists(week_overview_path):
        create_minimal_week_overview(calendar_events, today_date)

    week_overview = read_file(week_overview_path)

    for meeting in todays_meetings:
        # Determine new status based on prep file generation
        if meeting['type'] == 'customer':
            if meeting.get('agenda_owner') == 'you':
                # Agenda draft was generated
                old_status = '📅 Agenda needed'
                new_status = '✏️ Draft ready'
            else:
                # Prep file was generated
                old_status = '📋 Prep needed'
                new_status = '✅ Prep ready'

        elif meeting['type'] == 'project':
            old_status = '🔄 Bring updates'
            new_status = '✅ Prep ready'

        elif meeting['type'] == 'internal':
            old_status = '👥 Context needed'
            new_status = '✅ Prep ready'

        else:
            continue  # Skip personal/unknown

        # Update the table row in week overview
        week_overview = update_table_row(
            week_overview,
            match_columns={'Day': format_day(today_date), 'Account/Meeting': meeting['name']},
            update_column='Prep Status',
            new_value=new_status
        )

    write_file(week_overview_path, week_overview)


def create_minimal_week_overview(calendar_events, today_date):
    """
    Create a lightweight week overview from calendar if /week wasn't run.
    This ensures /today can work independently.
    """
    week_num = get_week_number(today_date)
    monday = get_monday_of_week(today_date)
    friday = monday + timedelta(days=4)

    overview_content = f"""# Week Overview: W{week_num:02d} - {monday.strftime('%B %d')}-{friday.strftime('%d, %Y')}

*Minimal overview created by /today - run /week for full planning*

## This Week's Meetings

| Day | Time | Account/Meeting | Category | Prep Status | Meeting Type |
|-----|------|-----------------|----------|-------------|--------------|
"""

    # Add meetings from calendar
    for event in calendar_events:
        meeting_date = parse_date(event['start'])
        if monday <= meeting_date <= friday:
            prep_status = determine_initial_prep_status(event)
            overview_content += f"| {format_day(meeting_date)} | {format_time(event['start'])} | {event.get('account', event['summary'])} | {event.get('category', '-')} | {prep_status} | {event.get('meeting_type', 'Unknown')} |\n"

    overview_content += """
---

*Run /week for complete planning including action items, hygiene alerts, and time blocking.*
"""

    write_file('_today/week-00-overview.md', overview_content)
```

**Resilience:** If /week hasn't been run:
- /today still works - reads directly from Google Calendar
- Creates a minimal week overview on-the-fly
- All prep files still generated correctly
- Impact: No week-at-a-glance view, but daily operations unaffected

### Step 6: Aggregate Action Items

Scan master task list and any distributed action files:

```
Read: _today/tasks/master-task-list.md (PRIMARY SOURCE)
Glob: [Account folders]/action-items/*.md (if applicable)
Grep: "- [ ]" (unchecked items)
```

**CRITICAL: Filter by Owner**

Only include items where you are the owner. Items owned by others are their responsibility.

Create `80-actions-due.md`:

```markdown
# Action Items - [Date]

## Overdue

- [ ] **[Action]** - [Account/Project] - Due: [Date] (X days overdue)
  - **Context**: [Why this action exists]
  - **Source**: [file path]

## Due Today

- [ ] **[Action]** - [Account/Project]
  - **Context**: [Why]
  - **Source**: [file path]

## Related to Today's Meetings

### [Account/Project Name] (Meeting at [Time])
- [ ] **[Action]** - Due: [Date]
  - **Status update to share**: [What progress to report]

## Due This Week

- [ ] **[Action]** - [Account/Project] - Due: [Date]

## Waiting On (Delegated)

| Who | What | Asked | Days | Context |
|-----|------|-------|------|---------|
| [Name] | [Action delegated to them] | [Date asked] | [Days waiting] | [Brief context] |

## Upcoming (Next 2 Weeks)

- [ ] **[Action]** - [Account/Project] - Due: [Date]
```

### Step 7: Look-Ahead for Agendas (3-4 Business Days)

Fetch next 5 calendar days:
```bash
python3 .config/google/google_api.py calendar list 5
```

For each customer/client meeting in look-ahead window:

**Check if agenda exists:**
1. Calendar event description contains Google Doc link → EXISTS
2. Calendar event description has substantial text (>100 chars) → EXISTS
3. Agenda file exists in account folder → EXISTS
4. None of above → **NEEDS AGENDA**

**If agenda needed:**
- Create draft in `_today/90-agenda-needed/[account]-[date].md`
- Use agenda-generator agent if available

### Step 8: Generate Suggested Focus

Create `81-suggested-focus.md`:

```markdown
# Suggested Focus Areas - [Date]

## Priority 1: Pre-Meeting Prep
- [ ] Review [Account] prep before [time] call

## Priority 2: Overdue Items
- [ ] Address [action] for [Account] (X days overdue)

## Priority 3: Agenda Sending
- [ ] Review and send agenda for [Account] ([date] meeting)
  - Draft: 90-agenda-needed/[filename]

## Priority 4: Account Hygiene (If Time)
- [ ] Refresh [Account] dashboard (last updated [date])

## Energy-Aware Notes
- Morning (high energy): Strategic prep, important calls
- Afternoon (lower energy): Admin capture, follow-ups
```

### Step 9: Generate Overview

Create `00-overview.md`:

```markdown
# Today: [Day, Month Date, Year]

## Schedule

| Time | Event | Type | Prep Status |
|------|-------|------|-------------|
| 9:00 AM | Daily Prep | Personal | - |
| 10:00 AM | Manager 1:1 | Internal | ✅ Prep ready |
| 11:00 AM | Project Sync | Project | ✅ Prep ready |
| 12:00 PM | **Client Meeting** | **Customer** | ✅ Prep ready - See [filename] |
| 2:00 PM | **Client B** | **Customer** | ✏️ Draft ready - agenda in 90-agenda-needed/ |

## Customer Meetings Today

### [Account] ([Time])
- **Status**: [Status]
- **Key Metric**: [Value]
- **Prep**: See [filename]

## Email - Needs Attention (if email scan enabled)

### HIGH Priority ([count])

| From | Subject | Type | Notes |
|------|---------|------|-------|
| [sender] | [subject] | [type] | Brief summary |

*Full details: 83-email-summary.md*

## Action Items - Quick View

### Overdue
- [ ] [Action] - [Account] - Due: [Date]

### Due Today
- [ ] [Action] - [Account]

## Agenda Status (Next 3-4 Business Days)

| Meeting | Date | Status | Action |
|---------|------|--------|--------|
| [Account] | [Date] | 📅 Agenda needed | Draft in 90-agenda-needed/ |
| [Account] | [Date] | ✅ Exists | None |
| [Account] | [Date] | ✏️ Draft ready | Review and send |

## Suggested Focus for Downtime

1. [Top priority from 81-suggested-focus.md]
2. [Second priority]
```

## Output Structure

After running `/today`:

```
_today/
├── 00-overview.md                      # Today's dashboard
├── 01-0900-personal-daily-prep.md      # First meeting
├── 02-1100-internal-project.md         # Second meeting
├── 03-1200-customer-client-prep.md     # Customer meeting (FULL PREP)
├── 80-actions-due.md                   # Action items
├── 81-suggested-focus.md               # Focus suggestions
├── 83-email-summary.md                 # Email triage results (if enabled)
├── 90-agenda-needed/                   # Draft agendas
│   └── client-jan-12.md               # Agenda to review and send
├── tasks/                              # Persistent task tracking
│   └── master-task-list.md            # Global task list (survives archive)
└── archive/                           # Previous days (processed by /week)
    └── 2026-01-07/
```

## Dependencies

**APIs (Optional but recommended):**
- Google Calendar (read + write)
- Google Sheets (read) - for account data
- Gmail (read) - for email triage

**Data Sources:**
- `_today/tasks/master-task-list.md`
- Account/project documentation folders
- Calendar events

## Error Handling

**If Google API unavailable:**
- Proceed with calendar data from cache if available
- Show warning in overview

**If account folder doesn't exist:**
- Create minimal prep with available calendar info
- Note: "Account folder not found - limited context"

**If action file is stale (>30 days):**
- Show warning: "Action file last updated [date] - review for accuracy"

## Related Commands

- `/wrap` - End-of-day closure and reconciliation
- `/week` - Monday weekly review
- `/month` - Monthly roll-up
- `/email-scan` - Email inbox triage (standalone)
