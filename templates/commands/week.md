# /week - Monday Review

Start the week with full context and a plan. Acts as your "chief of staff" for weekly planning.

## When to Use

Run Monday morning (or Sunday evening for prep). This command:
- **Prompts for your weekly priorities** (interactive)
- Shows all important meetings this week
- Surfaces overdue and due-this-week action items
- Flags items needing attention (hygiene alerts)
- Pre-populates weekly impact template
- **Analyzes open time blocks and suggests task scheduling**
- **Creates calendar events for approved time blocks**

## Philosophy

`/week` does strategic planning for the whole week. `/today` handles tactical execution each day. Together they act as your admin assistant, keeping you on top of schedule and priorities.

**Zero guilt design** - If you miss a Monday, run it Tuesday. The system adapts.

## Execution Steps

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
WEEK_FILES=$(ls _today/week-*.md 2>/dev/null)

if [ -n "$WEEK_FILES" ]; then
    PREV_WEEK=$(printf "W%02d" $(($(date +%V) - 1)))
    ARCHIVE_DIR="_today/archive/$PREV_WEEK"

    mkdir -p "$ARCHIVE_DIR"

    # Move all week-* files to archive
    for f in _today/week-*.md; do
        mv "$f" "$ARCHIVE_DIR/" 2>/dev/null
    done

    echo "Archived previous week files to $ARCHIVE_DIR"
fi
```

**Archive structure after /week runs:**
```
_today/
├── week-00-overview.md         # NEW - current week
├── week-01-meetings.md
├── week-02-actions.md
├── week-03-hygiene-alerts.md
├── week-04-focus.md
├── tasks/                      # PERSISTENT - never archived
│   └── master-task-list.md
└── archive/
    ├── W03/                    # Previous week's week-* files
    │   ├── week-00-overview.md
    │   └── ...
    ├── 2026-01-17/             # Daily archives
    └── ...
```

### Step 2: Priority Setting (Interactive)

**Use AskUserQuestion** to prompt for weekly priorities:

```
Question 1: "What are your top 3 priorities this week?"
Header: "Priorities"
Options:
- "Customer/client meetings and account work"
- "Strategic project work"
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
python3 .config/google/google_api.py calendar list 5
```

Filter events to only Mon-Fri of current week.

### Step 4: Fetch Account Data (Optional)

If account tracking sheet is configured:

```bash
python3 .config/google/google_api.py sheets get "YOUR_SHEET_ID" "A1:Z50"
```

Parse columns for account lookup (name, status, last contact, renewal date, etc.).

### Step 5: Identify All Meetings and Determine Prep Type

**All meetings get prep.** The difference is what *type* of prep:
- Customer/client meetings → Full prep (context, history, actions)
- Internal meetings → Relationship prep (shared work, context)
- Project meetings → Status prep (project state, partner updates)

For each event in the week:
1. Classify using domain mapping (same as /today)
2. For ALL meetings, note:
   - Date/time
   - Account/Meeting name
   - Category (Customer, Internal, Project, Personal)
   - Meeting type (from calendar title)
   - Prep status (determined by classification logic below)
   - Agenda owner (for customer meetings)

**Prep Status Logic:**

```python
def determine_prep_status(meeting, account_data):
    """
    Determine prep status for any meeting type.
    All meetings get prep - the type differs.

    Returns: (prep_status, agenda_owner, metadata)
    """
    meeting_type = meeting.get('type', 'unknown')
    title = meeting.get('summary', '').lower()

    # Check for strategic meeting signals (override default logic)
    agenda_signals = ['renewal', 'ebr', 'qbr', 'strategic review', 'quarterly']
    is_strategic = any(signal in title for signal in agenda_signals)

    if meeting_type == 'customer':
        relationship_stage = account_data.get('stage', 'new')

        # New/early relationships or strategic meetings = you own agenda
        if relationship_stage in ['new', 'onboarding'] or is_strategic:
            return '📅 Agenda needed', 'you', {}
        else:
            # Established relationships - customer often drives
            return '📋 Prep needed', 'customer', {}

    elif meeting_type == 'project':
        return '🔄 Bring updates', 'shared', {}

    elif meeting_type == 'internal':
        return '👥 Context needed', None, {}

    else:  # external, personal, unknown
        return '👥 Context needed', None, {}
```

**Prep Status Types:**

| Meeting Type | Prep Status | Agenda Owner | What Prep Includes |
|--------------|-------------|--------------|-------------------|
| Customer (new/strategic) | `📅 Agenda needed` | You | Full prep + agenda draft for you to send |
| Customer (established) | `📋 Prep needed` | Customer | Context, history, actions, attendee intel |
| Project | `🔄 Bring updates` | Shared | Project status, partner updates, blockers |
| Internal (1:1s, syncs) | `👥 Context needed` | N/A | Relationship context, shared work |

**Override Signals (upgrade to "📅 Agenda needed"):**
- Meeting title contains "renewal", "EBR", "QBR", "strategic review"
- Calendar description mentions you owning the agenda

**Agenda Task Creation:**
When prep status is "📅 Agenda needed", create a task in master-task-list.md:

```markdown
- [ ] **Send agenda: [Account] [meeting type]** `[date]-agenda-001`
  - Account: [Account]
  - Due: [1 day before meeting]
  - Owner: [You]
  - Source: /week W[NN] planning
  - Meeting: [date time]
  - Draft: _today/90-agenda-needed/[account-lowercase]-[date].md
  - Priority: P2
```

### Step 6: Aggregate Action Items

Scan master task list and distributed action files:

```
Read: _today/tasks/master-task-list.md
Glob: [Account folders]/action-items/*.md (if applicable)
Grep: "- [ ]" (unchecked items)
```

Categorize by:
- **Overdue**: Past due date
- **Due This Week**: Due Mon-Fri of current week
- **Related to This Week's Meetings**: Actions for accounts with meetings

### Step 7: Check Account Hygiene (if account tracking enabled)

Scan account dashboards for issues:

| Check | Source | Threshold | Alert Level |
|-------|--------|-----------|-------------|
| **Stale Dashboard** | Dashboard "Last Updated" | >60 days | Medium |
| **No Recent Contact** | Sheet/Dashboard | Varies by tier | High |
| **Upcoming Milestone** | Sheet/Dashboard | <30 days | Critical |
| **Overdue Actions** | Action files | Any overdue | High |
| **Open Risks** | Dashboard "Risks" section | Unchecked | High |

### Step 8: Check Previous Week's Impact

Before creating new impact template, check for continuity:

```bash
# Find previous week's impact file
ls Leadership/impact/ | grep -E "$(date -v-7d +%Y)-W$(printf '%02d' $(($(date -v-7d +%V))))"
```

**If previous week exists:**
- Note: "Previous week (W[X]) impact captured"
- Carry forward incomplete items to new template

**If previous week missing:**
- Add warning: "Previous week (W[X]) impact not captured"

### Step 9: Pre-Populate Weekly Impact Template

Create `Leadership/impact/[YYYY]-W[NN]-impact-capture.md`:

```markdown
---
area: Leadership
doc_type: impact
status: draft
date: [YYYY-MM-DD]
week: W[NN]
tags: [impact, weekly]
---

# Weekly Impact Capture - W[NN] ([Month Day-Day, Year])

## Meetings This Week

| Day | Account/Project | Meeting Type | Outcome |
|-----|-----------------|--------------|---------|
| Mon | | | |
| Tue | [Account] | Monthly Sync | |
| Wed | | | |
| Thu | [Account] | Technical Review | |
| Fri | | | |

## Value Delivered

### Customer/Client Wins
-

### Technical Outcomes
-

## Relationship Progress

### Stakeholder Engagement
-

### Executive Access
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

available_slots = [
    {"day": "Monday", "start": "09:30", "end": "11:00", "type": "morning", "duration": 90},
    {"day": "Monday", "start": "14:00", "end": "15:30", "type": "afternoon", "duration": 90},
]
```

#### 10.2: Task Effort Estimation

| Size | Duration | Examples |
|------|----------|----------|
| **Small** | 15 min | Send email, quick Slack, file update |
| **Medium** | 30 min | Draft agenda, review document, update dashboard |
| **Large** | 60 min | Write proposal, deep analysis, customer prep |

#### 10.3: Time Preference Matching

| Task Type | Preferred Time | Reason |
|-----------|----------------|--------|
| Strategic thinking, writing | Morning (9-12) | Best focus |
| Customer prep, analysis | Morning | Complex, needs focus |
| Email catch-up, Slack | Afternoon | Straightforward execution |
| Dashboard updates | Afternoon | Routine, low cognitive load |
| Action item follow-ups | Afternoon | Quick wins, clearing queue |

#### 10.4: Generate Suggested Schedule

```markdown
## Suggested Time Blocks

### Monday
| Time | Duration | Task | Priority | Type |
|------|----------|------|----------|------|
| 9:30-10:30 | 60m | Client docs (overdue) | P2 | Large - Morning |
| 14:00-14:30 | 30m | Email follow-up | P2 | Medium - Afternoon |
| 14:30-15:00 | 30m | Dashboard refresh | P3 | Medium - Afternoon |
```

#### 10.5: User Approval Gate

**Use AskUserQuestion** to confirm the schedule:

```
Question: "I've created a suggested schedule. Would you like me to:"
Header: "Schedule"
Options:
- "Create all suggested calendar blocks"
- "Let me review and select which ones"
- "Just show suggestions, don't create events"
- "Skip scheduling this week"
```

#### 10.6: Create Calendar Events

For approved blocks:

```bash
python3 .config/google/google_api.py calendar create \
  "[Task Type]: [Description]" \
  "2026-01-12T09:30:00-05:00" \
  "2026-01-12T10:30:00-05:00" \
  "Task from master-task-list. Source: /week planning."
```

**Event categories:**
- `Focus:` - Deep work, writing, strategic thinking
- `Prep:` - Meeting preparation
- `Admin:` - Administrative tasks, follow-ups
- `Review:` - Dashboard updates, document reviews

### Step 11: Generate Week Overview

Create `_today/week-00-overview.md`:

```markdown
# Week Overview: W[NN] - [Month Day-Day, Year]

## This Week's Meetings

| Day | Time | Account/Meeting | Category | Prep Status | Meeting Type |
|-----|------|-----------------|----------|-------------|--------------|
| Tue | 10:30 AM | Client A | Customer | 📋 Prep needed | Monthly sync |
| Tue | 11:30 AM | Client B | Customer | 📅 Agenda needed | Renewal discussion |
| Tue | 1:00 PM | Manager 1:1 | Internal | 👥 Context needed | 1:1 |
| Wed | 10:00 AM | Project X | Project | 🔄 Bring updates | Weekly sync |
| Wed | 12:00 PM | Client C | Customer | 📅 Agenda needed | Strategic check-in |
| Thu | 9:00 AM | Client D | Customer | 📋 Prep needed | Technical call |

**Prep Status Guide:**

| Icon | Status | Meaning | Next Step |
|------|--------|---------|-----------|
| `📋 Prep needed` | Initial | Customer meeting prep required | /today generates prep file |
| `📅 Agenda needed` | Initial | You own agenda, need to create and send | /today creates draft, you review and send |
| `🔄 Bring updates` | Initial | Project meeting, come with status | Review project state before meeting |
| `👥 Context needed` | Initial | Internal meeting, relationship prep | /today adds context, shared work |
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

### Agenda Tasks Created
- [ ] **Send agenda: Client B renewal** - Due: [Day before meeting]
- [ ] **Send agenda: Client C check-in** - Due: [Day before meeting]

## Hygiene Alerts

### Critical
- **[Account]** - Milestone in [X] months, [concern]

### Needs Attention
- **[Account]** - Dashboard stale ([X] days)
- **[Account]** - No contact in [X] days

### Healthy
- [X] accounts with no alerts

## Weekly Impact Template

Pre-populated template created:
`Leadership/impact/[YYYY]-W[NN]-impact-capture.md`

**Reminder**: Capture impacts throughout the week, not Friday afternoon.

## Suggested Calendar Blocks

| Block | When | Purpose |
|-------|------|---------|
| Agenda Prep | [Day] AM | Send agenda for [Account] (📅 meetings) |
| Meeting Prep | [Day] AM | Review prep for [Account] (📋 meetings) |
| Dashboard Refresh | [Day] | Update [Account] dashboard |
| Impact Capture | Wed/Thu | Mid-week impact notes |

## Weekly Priorities

1. **Agendas**: Send agendas for 📅 meetings (you own these)
2. **Prep Review**: Review prep files before 📋 meetings
3. **Overdue Items**: Address [X] overdue actions
4. **Hygiene**: Refresh [X] stale dashboards
5. **Impact**: Keep impact template updated daily
```

### Step 12: Generate Supporting Files

**`week-01-meetings.md`**: Detailed meeting list
**`week-02-actions.md`**: Full action item breakdown by day
**`week-03-hygiene-alerts.md`**: Detailed hygiene issues
**`week-04-focus.md`**: Priority ranking with Must Do / Should Do / Could Do

### Step 13: Process Weekly Archives to Inbox

Move the week's archived daily files to `_inbox/` for canonical processing:

```bash
if [ -d "_today/archive" ]; then
    for day_dir in _today/archive/*/; do
        if [ -d "$day_dir" ]; then
            day_name=$(basename "$day_dir")
            mkdir -p "_inbox/daily-archive-$day_name"
            mv "$day_dir"* "_inbox/daily-archive-$day_name/" 2>/dev/null
            rmdir "$day_dir" 2>/dev/null
        fi
    done
fi
```

### Step 14: Monday Coordination

If today is Monday, prompt for /today:

```
"Week overview complete. Today is Monday - would you like me to run /today
to generate today's meeting prep files?"
```

## Output Structure

After running `/week`:

```
_today/
├── week-00-overview.md             # Week at a glance
├── week-01-meetings.md             # All important meetings
├── week-02-actions.md              # Action items for the week
├── week-03-hygiene-alerts.md       # Items needing attention
├── week-04-focus.md                # Suggested priorities
├── tasks/
│   └── master-task-list.md
└── [daily files if /today also run]

Leadership/impact/
└── [YYYY]-W[NN]-impact-capture.md  # Pre-populated impact template
```

## Dependencies

**APIs:**
- Google Calendar (full access - read and write)
- Google Sheets (read) - optional, for account data

**Data Sources:**
- Account documentation/dashboards
- Action item files
- `_today/tasks/master-task-list.md`

## Task Sizing Reference

| Size | Duration | Typical Tasks |
|------|----------|---------------|
| **Small** | 15 min | Quick email, Slack message, file rename |
| **Medium** | 30 min | Draft agenda, update dashboard, review document |
| **Large** | 60 min | Customer prep, write proposal, strategic analysis |

## Time Preference Reference

| Time of Day | Best For | Avoid |
|-------------|----------|-------|
| **Morning (9am-12pm)** | Complex thinking, writing, customer prep | Routine admin |
| **Afternoon (1pm-5pm)** | Execution tasks, emails, dashboard updates | Deep focus work |

## Related Commands

- `/today` - Daily operating system (run this after /week on Monday)
- `/month` - Monthly roll-up (first Monday of month)
- `/quarter` - Quarterly pre-population
