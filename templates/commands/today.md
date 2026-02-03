# /today - Daily Operating System

Populate `_today/` with everything needed for today's work.

## When to Use

Run every morning during "Daily Prep" calendar block. This command:
- Preps you for all meetings
- Surfaces action items due today
- Generates draft agendas for upcoming customer meetings
- Suggests focus areas for downtime

---

## First-Run Check

**Before running any scripts, check if this is a fresh workspace:**

```bash
# Check for workspace configuration
ls _config/workspace.json 2>/dev/null || echo "FIRST_RUN"
```

### If FIRST_RUN (no workspace.json found):

**Welcome to /today!** This appears to be your first time running this command. Here's what you need to know:

**Required for basic operation:**
- ✅ Workspace directory structure (created during setup)
- ✅ `_today/` folder for daily files

**Optional but recommended:**
- 📅 **Google Calendar** - Automatically fetches today's meetings
- 📧 **Gmail** - Surfaces important emails for triage
- 📊 **Google Sheets** - Loads account data for meeting classification

**Current status check:**
```bash
# Check Google API setup
ls .config/google/token.json 2>/dev/null && echo "Google API: Configured" || echo "Google API: Not configured (manual mode)"
```

**If Google API is not configured:**
That's okay! The command will run in **manual mode**:
- You can add meetings manually to your prep files
- Action items still work from your master task list
- You can set up Google API later with `/setup --google`

**Ready to continue?** Proceed to Phase 1 below. The script will clearly indicate which features are available.

---

## Three-Phase Execution

This command uses a three-phase approach for efficiency:

```
┌─────────────────────────────────────────────────────────────────┐
│                    THREE-PHASE COMMAND FLOW                     │
├─────────────────────────────────────────────────────────────────┤
│  Phase 1: PREPARATION (Python Script)                           │
│  • Fetch calendar events, classify meetings                     │
│  • Aggregate action items, check files                          │
│  • Fetch emails, identify agenda needs                          │
│  • Output: _today/.today-directive.json                         │
│                                                                 │
│  Phase 2: AI ENRICHMENT (Claude)                                │
│  • Generate meeting prep content                                │
│  • Analyze email tone, draft responses                          │
│  • Synthesize action priorities                                 │
│  • Create agenda drafts                                         │
│                                                                 │
│  Phase 3: DELIVERY (Python Script)                              │
│  • Write files to _today/                                       │
│  • Update week overview prep status                             │
│  • Optional: Create calendar blocks                             │
└─────────────────────────────────────────────────────────────────┘
```

## Execution Steps

### Phase 1: Run Preparation Script

**ALWAYS RUN THIS FIRST:**

```bash
python3 _tools/prepare_today.py
```

This script performs all deterministic operations:
- Resilience checks (yesterday's archive, unprocessed transcripts)
- Archive yesterday's files if needed
- Fetch account data from Google Sheet (if configured)
- Fetch today's calendar and classify meetings (if configured)
- Aggregate action items from master task list
- Fetch and classify emails (if configured)
- Identify agendas needed for look-ahead
- Check existing files in _today/

**Output:** `_today/.today-directive.json` containing structured data for Phase 2.

**Options:**
- `--skip-archive` - Don't archive yesterday's files
- `--skip-email` - Don't fetch emails
- `--output FILE` - Custom output path

#### Understanding Script Output

The script will clearly indicate what's available:

**Full Mode** (Google API configured):
```
Step 2: Fetching account data from Google Sheets...
  Loaded 35 accounts
Step 3: Fetching calendar events...
  Found 6 events for today
Step 7: Fetching and classifying emails...
  Found 12 emails
```

**Manual Mode** (Google API not configured):
```
Step 2: Fetching account data from Google Sheets...
  Skipped (Google API unavailable: token.json not found)
Step 3: Fetching calendar events...
  Skipped (Google API unavailable: token.json not found)
  NOTE: Add meetings manually or complete Google API setup
```

**This is normal for new workspaces!** The script still creates the directive file with:
- Action items from your master task list
- Existing file inventory
- Empty meeting slots you can populate manually

To set up Google API later: `/setup --google`

### Phase 2: AI Enrichment (Claude Tasks)

After the script completes, read the directive and execute AI tasks:

```bash
# Read the directive
cat _today/.today-directive.json
```

**For each task in directive['ai_tasks'], execute based on type:**

#### Customer Meeting Prep (`generate_customer_prep`)

For each customer meeting in the directive, generate prep using these sources:
1. **Account Dashboard** (PRIMARY) - `Accounts/[Account]/01-Customer-Information/*-dashboard.md`
2. **Recent Meetings** - `Accounts/[Account]/02-Meetings/*.md` (last 2-3)
3. **Action Items** - `Accounts/[Account]/04-Action-Items/current-actions.md`
4. **Clay** (if available) - Attendee relationship intel

**Generate prep file:** `_today/[NN]-[HHMM]-customer-[account]-prep.md`

Reference: The original Step 4-5 sections below for detailed prep format.

#### Internal Meeting Prep (`generate_internal_prep`)

Generate relationship-aware prep:
- Clay lookup for attendees
- Shared accounts context
- Political intelligence if relevant

#### Project Meeting Prep (`generate_project_prep`)

Generate project-focused prep:
- Project status from `Projects/[Project]/00-Index.md`
- Recent activity and blockers

#### Email Summarization (`summarize_email`)

For high-priority emails:
- Fetch full thread if applicable
- Classify: OPPORTUNITY / INFO / RISK / ACTION NEEDED
- Extract specific asks for [Your Name]
- Recommend action and owner

#### Agenda Drafts (`generate_agenda_draft`)

For meetings in look-ahead window needing agendas:
- Generate draft in `_today/90-agenda-needed/[account]-[date].md`

### Phase 3: Run Delivery Script

**AFTER completing AI tasks:**

```bash
python3 _tools/deliver_today.py
```

This script:
- Writes 00-overview.md with schedule and summaries
- Writes 80-actions-due.md with action items
- Writes 83-email-summary.md with email triage
- Writes 81-suggested-focus.md with priorities
- Updates week overview with prep status
- Cleans up directive file

**Options:**
- `--skip-calendar` - Don't create calendar blocks
- `--keep-directive` - Keep directive file for debugging

---

## Legacy Reference: Detailed Prep Formats

The following sections are reference material for Phase 2 AI enrichment.

### Step 1: Archive Yesterday and Clear _today/

**Archive lifecycle:**
- Daily files stay in `_today/archive/YYYY-MM-DD/` during the week
- Week files (`week-*`) persist all week until next /week run
- /week processes and moves archives to `_inbox/` for canonical filing
- This provides fast 7-day access + long-term searchability

```bash
# Get yesterday's date
YESTERDAY=$(date -v-1d +%Y-%m-%d)

# Archive yesterday's content (if exists)
if [ -f "_today/00-overview.md" ]; then
    mkdir -p _today/archive/$YESTERDAY

    # Move all daily files EXCEPT week-* files (NOT archive, tasks, or 90-agenda-needed folders)
    # IMPORTANT: Preserve week-* files - they persist until /week archives them
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

**Archive structure:**
```
_today/
├── 00-overview.md              # Today's files (archived daily)
├── 01-1100-project-agentforce.md
├── ...
├── week-00-overview.md         # PERSISTENT - archived only by /week
├── week-01-customer-meetings.md
├── week-02-actions.md
├── week-03-hygiene-alerts.md
├── week-04-focus.md
├── 90-agenda-needed/           # Today's draft agendas
├── tasks/                      # PERSISTENT - never archived
│   └── master-task-list.md
└── archive/                    # Rolling archive (cleared by /week)
    ├── 2026-01-07/
    │   ├── 00-overview.md
    │   ├── 01-customer-acme-corp-prep.md
    │   └── 90-agenda-needed/
    ├── 2026-01-06/
    └── 2026-01-05/
```

**Note:** Daily archives are NOT auto-deleted by /today. They persist until /week moves them to `_inbox/` for canonical processing.

### Step 2: Fetch Account Data from Google Sheet

```bash
python3 .config/google/google_api.py sheets get "1edLlG0rkPj9QRT5mWQmCh_L-qy4We9fBLJ4haMZ_14g" "A1:AB50"
```

Parse the JSON to build account lookup and domain mapping:

| Column | Field | Usage |
|--------|-------|-------|
| A | Account name | Display name |
| D | Lifecycle Ring | Context for prep |
| I | 2025 ARR | Display in prep |
| F | Last Engagement Date | Stale contact alerts |
| P | Next Renewal Date | Renewal countdown |
| X | Meeting Cadence | Engagement expectations |
| **AB** | **Email Domain** | **Domain → Account mapping** |

**Build Domain Mapping from Sheet:**

```python
# Parse Sheet data to create domain → account lookup
domain_map = {}
for row in sheet_data[1:]:  # Skip header
    account_name = row[0]   # Column A
    email_domain = row[27] if len(row) > 27 else None  # Column AB
    if email_domain:
        domain_map[email_domain] = account_name

# Multi-BU domains (same domain, multiple accounts) - configured in workspace.json
# Example: multi_bu_domains = ['parent-company.com']
```

**Multi-BU Accounts** (require title-matching or user prompt):
Configure in `_config/workspace.json` under `accounts.multiBuParents`:
- Each parent company can have multiple business units with the same email domain
- The system prompts user to select which BU when needed

### Step 3: Fetch Today's Calendar

```bash
python3 .config/google/google_api.py calendar list 1
```

Parse JSON output. For each event extract:
- `id`: Event ID
- `summary`: Meeting title
- `start`: Start time (parse to get HHMM for filename)
- `end`: End time
- `attendees`: List of email addresses

**Filter out declined events:**
For events where you need detailed response status, use `calendar get <event_id>` to check if your responseStatus is "declined". Skip events where you declined.

```python
# Example filter logic
for event in events:
    # Get full event details if needed
    if is_multi_attendee(event):
        details = get_event_details(event['id'])
        my_response = find_my_response(details['attendees'], 'your.email@company.com')
        if my_response == 'declined':
            continue  # Skip this event
```

### Step 3.5: Scan Email Inbox

Fetch recent/unread emails and classify by priority:

```bash
python3 .config/google/google_api.py gmail search "is:unread in:inbox" 30
```

**Classification (from /email-scan logic):**

| Priority | Criteria | Action |
|----------|----------|--------|
| **HIGH** | From customer domain (match to Sheet), from leadership, action words in subject | Surface in overview with full summary |
| **MEDIUM** | Internal colleagues, meeting-related, P2 notifications | Note count |
| **LOW** | Newsletters, GitHub (no @mention), automated | Archive automatically |

**CRITICAL: For HIGH priority emails, provide actual summaries not just snippets.**

Reference: `.claude/commands/email-scan.md` → Step 4: Thread Summarization Framework

For each HIGH priority email:

1. **Check if threaded** (threadId != id means it's part of a conversation)
2. **If threaded, fetch full thread** to understand conversation arc
3. **Summarize with classification:**

| Type | Indicator | Icon |
|------|-----------|------|
| OPPORTUNITY | Expansion, new work, positive signal | 🟢 |
| INFORMATIONAL | FYI, status update, no action needed | 🟡 |
| RISK | Concern, complaint, churn signal, blocker | 🔴 |
| ACTION NEEDED | Explicit ask for [Your Name] | 🔵 |

4. **For each HIGH priority email, answer:**
   - What's the conversation arc? (Who initiated, what's discussed, current status)
   - Is there a specific ask for [Your Name]? (Yes/No, what and by when)
   - Who is the owner? ([Your Name], or someone else - e.g., "[Manager] handling, monitor only")
   - What action (if any) should [Your Name] take?

**Email Summary for Overview:**
```python
email_summary = {
    'high_priority': [],      # Need attention - with full summaries
    'customer_emails': [],    # From accounts (with context and classification)
    'action_requested': [],   # Explicit action needed
    'medium_count': 0,        # Labeled for later
    'archived_count': 0       # Noise removed
}
```

**Integration with meetings:**
If a HIGH priority email is from an account with a meeting today:
- Add email context to that meeting's prep file
- Flag: "📧 Email from [sender] received [time] - may need discussion"

**Output file:** `83-email-summary.md` - Must include:
- Conversation arc (not just snippet)
- Classification type with reasoning
- Specific ask identification
- "For [Your Name]" section with recommended action and owner

### Step 4: Classify Each Meeting

For each event, check attendee domains:

```
STEP 1: Check for known PROJECTS first (before customer classification)

Known projects with external partners (configurable in meeting_utils.py KNOWN_PROJECTS):
- Example: "Project Alpha" → Projects/Project-Alpha/ (partners: @partner.com)
- [Add other projects as needed]

IF meeting title contains known project name:
    type = "project"
    project = matched project name
    → Generate project meeting prep (not customer prep)

STEP 2: If not a project, classify by attendees

IF no attendees OR only owner:
    type = "personal"
    account = None

ELSE IF all attendees are from internal domains (defined in _config/workspace.json):
    type = "internal"
    account = None

ELSE (external attendees present):
    Extract external domains
    Match to domain mapping above

    IF exactly one account matched:
        type = "customer"
        account = matched account name

    ELSE IF multiple accounts matched (e.g., parent company with multiple BUs):
        type = "customer"
        Use AskUserQuestion: "Which BU is this meeting for? [list options]"
        account = user's answer

    ELSE (unknown external domain):
        type = "external"
        account = None
        Note the unknown domain for future mapping
```

**Project Detection Logic:**

| Project | Title Keywords | Partner Domains | Location |
|---------|---------------|-----------------|----------|
| [Project Name] | [Keywords in title] | @partner.com | Projects/[Project]/ |
| [Add others as identified] | | | |

### Step 5: Generate Meeting Files (Numbered by Time)

Create files in chronological order with naming convention:
`[NN]-[HHMM]-[type]-[name].md`

**File numbering:**
- `00` = overview (always first)
- `01-79` = meetings in chronological order
- `80-89` = reference documents (actions, focus)
- `90-99` = action-needed items (agendas)

**Time-aware behavior:**

```python
from datetime import datetime

current_time = datetime.now()

def get_meeting_status(meeting_start, meeting_end):
    """
    Returns: 'past', 'in_progress', 'upcoming'
    """
    if current_time > meeting_end:
        return 'past'
    elif current_time >= meeting_start:
        return 'in_progress'
    else:
        return 'upcoming'

# For each meeting:
status = get_meeting_status(meeting['start'], meeting['end'])

if status == 'past':
    # Don't generate full prep
    # Mark as "✓ Past" in overview
    # Still include in file list but with minimal content

elif status == 'in_progress':
    # Mark as "🔴 In Progress" in overview
    # Include prep link in case user needs quick reference

else:  # upcoming
    # Generate full prep for customer meetings
    # Mark as "Upcoming" in overview
```

**Meeting Status Display:**
| Status | Icon | Action |
|--------|------|--------|
| Past | ✓ | Skip prep generation, minimal file |
| In Progress | 🔴 | Link to existing prep if available |
| Upcoming | ⏳ | Generate full prep for customer meetings |

#### For CUSTOMER Meetings

**IMPORTANT**: The **Account Dashboard** is the PRIMARY source of truth for customer prep.

**Step-by-step:**

1. **Load Account Dashboard** (PRIMARY SOURCE):
   ```bash
   # Find account folder (handle multi-BU structure)
   ls Accounts/ | grep -i "[Account]"
   # OR for multi-BU:
   ls Accounts/[Parent-Company]/ | grep -i "[BU-Name]"
   ```
   ```
   Read: Accounts/[Account]/01-Customer-Information/[account]-account-dashboard.md
   ```

   **Key Dashboard Sections to Extract:**
   | Section | Use For |
   |---------|---------|
   | Executive Summary | Quick context, current state |
   | Quick View | ARR, Tier, Health, Renewal date, Last Contact |
   | ⚠️ Current Risks | What to address/monitor |
   | ✅ Recent Wins | What to acknowledge/build on |
   | 🎯 Next Actions | Open commitments |
   | Stakeholder Map | Who's in the meeting, their role/influence |
   | Value Gaps | Opportunities to explore |
   | Unknown/Need to Discover | Questions to ask |

2. **Check Recent Activity** (SUPPLEMENT dashboard):
   ```
   Glob: Accounts/[Account]/02-Meetings/*.md (last 3 files by date)
   Read most recent meeting summary for "Since Last Meeting" context

   Glob: Accounts/[Account]/04-Action-Items/*.md
   Read current-actions.md (or most recent) for open items
   ```

3. **Check for Stale Data**:
   - Dashboard "Last Updated" date > 30 days: add ⚠️ warning
   - Action file last modified > 30 days: add ⚠️ warning
   - If conflict between dashboard and action file: note discrepancy

3.5. **Lookup Attendees in Clay** (if Clay MCP available):
   For each external attendee email from the calendar event:

   ```python
   # Search Clay for each external attendee
   for attendee_email in external_attendees:
       # Extract name/company from email domain
       name = extract_name_from_email(attendee_email)
       company = map_domain_to_company(attendee_email)

       # Search Clay
       contact = mcp__clay__searchContacts(
           query=name,
           company_name=[company] if company else [],
           limit=1
       )

       if contact:
           # Get full details
           details = mcp__clay__getContact(contact['id'])
           clay_intel.append({
               'name': contact['name'],
               'title': contact.get('headline'),
               'last_interaction': contact.get('last_interaction_date'),
               'score': contact.get('score'),
               'linkedin': details.get('social_links', [])[0] if details.get('social_links') else None,
               'notes': details.get('notes', [])
           })
   ```

   **Staleness thresholds:**
   | Days Since Last Interaction | Alert |
   |-----------------------------|-------|
   | 0-60 | (none) - relationship is fresh |
   | 61-180 | ⚠️ May need warming |
   | >180 | 🔴 Stale - re-establish context |

   **Score interpretation:**
   | Score | Meaning |
   |-------|---------|
   | >500 | Strong relationship |
   | 100-500 | Moderate relationship |
   | <100 | Weak/new - extra prep needed |

4. **Generate Prep Summary** using this format:

```markdown
# [Account] Call Prep
**[Date] | [Meeting Title]**

## Quick Context
*(From: [account]-account-dashboard.md → Quick View)*

| Metric | Value |
|--------|-------|
| **Ring** | [Ring] - [Implication from dashboard] |
| **ARR** | $[Amount] |
| **Health** | [Score/Status] |
| **Renewal** | [Date] ([X months]) |
| **Last Contact** | [Date] - [Topic] |

## Attendees
*(Cross-reference: Dashboard → Stakeholder Map + Clay)*

| Name | Role | Influence | Notes |
|------|------|-----------|-------|
| [Name] | [Role] | [High/Med/Low] | [From stakeholder map] |

## Attendee Intelligence
*(From Clay - if available)*

| Attendee | Title | Last Interaction | Score | Alert |
|----------|-------|------------------|-------|-------|
| [Name] | [Headline from Clay] | [Date] | [Score] | [⚠️ if stale] |

**Quick Links:**
- [Name]: [LinkedIn](url) | email@company.com

**Not in Clay:** [unknown emails - consider adding after meeting]

**Recent Notes:** [Any Clay notes about key attendees]

## Since Last Meeting
*(From: [most-recent-meeting-summary.md])*

[Summary of key points, decisions, and commitments from last meeting]

**Source**: `Accounts/[Account]/02-Meetings/[filename].md`

## Open Action Items
*(From: Dashboard → Next Actions + Action Items file)*

- [ ] **[Action]** - Owner: [name] - Due: [date]
  - **Context**: [Why this action exists]
  - **Requested by**: [Who initiated]
  - **Source**: `[path/to/source-file.md]`
  - **Related to**: [Project/initiative]

Example:
- [ ] **Review Jane's Node.js/log analysis support ticket** - Owner: [Your Name] - Due: Dec 18
  - **Context**: Customer has 40GB of logs over 4 days, needs analysis tooling to diagnose performance issues
  - **Requested by**: [Manager] (on behalf of Jane at Acme Corp)
  - **Source**: `Accounts/Acme Corp/02-Meetings/2025-12-15-summary-acme-corp-monthly.md`
  - **Related to**: Node.js migration support

**⚠️ Action file last updated: [date]** (if >30 days, show warning)

## Current Risks to Monitor
*(From: Dashboard → ⚠️ Current Risks)*

- [ ] **[Risk]**
  - **Source**: `[account]-account-dashboard.md`

## Recent Wins to Acknowledge
*(From: Dashboard → ✅ Recent Wins)*

- **[Win]** ([Date])
  - **Source**: `[account]-account-dashboard.md`

## Suggested Talking Points
*(Synthesized from dashboard + recent meetings)*

1. **Follow up on**: [from open actions]
   - **Reference**: `[source-file.md]`
2. **Check in on**: [from success plan objectives]
   - **Reference**: `[account]-account-dashboard.md → Success Plan`
3. **Explore**: [expansion or value delivery opportunity]
   - **Reference**: `[account]-account-dashboard.md → Value Gaps`

## Questions to Ask
*(From: Dashboard → Unknown/Need to Discover)*

- [Discovery question]
  - **Source**: `[account]-account-dashboard.md`
- [Follow-up from previous discussion]
  - **Source**: `[meeting-summary.md]`

## Key References
| Document | Path | Last Updated |
|----------|------|--------------|
| Account Dashboard | `Accounts/[Account]/01-Customer-Information/[account]-account-dashboard.md` | [Date] |
| Last Meeting Summary | `Accounts/[Account]/02-Meetings/[filename].md` | [Date] |
| Action Items | `Accounts/[Account]/04-Action-Items/current-actions.md` | [Date] |
| Success Plan | `Accounts/[Account]/success-plan.md` | [Date] |
```

#### For PROJECT Meetings

Generate project-focused prep by reading from Projects/ directory:

1. **Load Project Context**:
   ```
   Read: Projects/[Project]/00-Index.md (or similar overview)
   Glob: Projects/[Project]/*.md (recent files)
   ```

2. **Generate Project Prep**:

```markdown
# [Project] Sync
**[Date] | [Meeting Title]**

## Project Context
- **Project**: [Project Name]
- **Status**: [From project docs]
- **Partners**: [List partner organizations]

## Attendees
- [Name] ([Company]) - [Role in project]

## Recent Activity
[Summary from recent project files]

## Open Items
- [ ] [Item from project tracking]

## Discussion Topics
1. [Topic from recent activity]
2. [Blocker or decision needed]

## Notes

```

#### For INTERNAL Meetings

Internal meetings still deserve prep, just with a different focus. Generate relationship-aware prep.

**Data Sources for Internal Prep:**
- Clay: Relationship score, last interaction, notes
- Google Sheet: Find colleague's accounts (match by domain or name)
- Account action files: Find items owned by either party
- Political intelligence folder: Any relevant context

**Step-by-step:**

1. **Lookup Attendee in Clay** (if Clay MCP available):
   ```python
   for attendee_email in internal_attendees:
       name = extract_name_from_email(attendee_email)

       contact = mcp__clay__searchContacts(
           query=name,
           company_name=['Your Company Name'],  # Configure for your organization
           limit=1
       )

       if contact:
           details = mcp__clay__getContact(contact['id'])
           clay_intel = {
               'last_interaction': contact.get('last_interaction_date'),
               'score': contact.get('score'),
               'notes': details.get('notes', [])
           }
   ```

2. **Find Shared Accounts** (if applicable):
   ```python
   # Check if this colleague shares any accounts with [Your Name]
   # Look in Google Sheet for accounts where they're listed as co-owner
   # Or check account files for their name in stakeholder/owner fields

   shared_accounts = find_shared_accounts(attendee_email, account_data)
   # Returns: [{'account': 'Account Name', 'tier': 'Tier 1', 'your_actions': [...], 'their_actions': [...]}]
   ```

3. **Check Political Intelligence** (if exists):
   ```
   Glob: Leadership/06-Political-Intelligence/*.md
   Grep: [attendee name] or [attendee email domain]
   ```

4. **Generate Internal Prep**:

```markdown
# [Colleague] 1:1 Prep
**[Time] | Internal**

## Relationship Context
*(From Clay - if available)*

| Metric | Value |
|--------|-------|
| **Last Interaction** | [Date from Clay] |
| **Relationship Score** | [Score from Clay] |
| **Recent Notes** | [Brief from Clay notes] |

## Shared Account Status
*(Accounts you both work on - if applicable)*

| Account | Ring | Your Actions | Their Actions | Status |
|---------|------|--------------|---------------|--------|
| [Account] | [Ring] | [Your pending items] | [Their pending items] | [Active/On hold] |

**Cross-Account Context:**
- [Any recent developments on shared accounts]
- [Alignment opportunities or blockers]

## Political/Relational Context
*(From Leadership/06-Political-Intelligence/ - if available)*

- [Any relevant dynamics, sensitivities, or positioning notes]
- [Working relationship observations]
- [Stakeholder alignment context]

## Pre-Read Check

- [ ] Pre-read shared in calendar invite?
- [ ] Linked doc needs review?

## Potential Topics

Based on shared context:
1. [Shared account updates]
2. [Project coordination]
3. [Feedback or coaching]
4. [Open questions or asks]

## Notes

```

**Lighter-weight internal prep** (for team syncs, large group meetings):

```markdown
# [Meeting Title]
**[Time] | Internal**

## Attendees
- [List from calendar]

## Your Updates to Share
*(Based on your accounts/projects)*

- [Recent wins or progress]
- [Blockers needing input]
- [Announcements or FYIs]

## Notes

```

#### For PERSONAL Meetings

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
            match_columns={'Day': format_day(today_date), 'Account/Meeting': meeting['account']},
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

| Day | Time | Account/Meeting | Ring | Prep Status | Meeting Type |
|-----|------|-----------------|------|-------------|--------------|
"""

    # Add meetings from calendar
    for event in calendar_events:
        meeting_date = parse_date(event['start'])
        if monday <= meeting_date <= friday:
            prep_status = determine_initial_prep_status(event)
            overview_content += f"| {format_day(meeting_date)} | {format_time(event['start'])} | {event.get('account', event['summary'])} | {event.get('ring', '-')} | {prep_status} | {event.get('meeting_type', 'Unknown')} |\n"

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

Scan master task list and account action files for items due today, overdue, or related to today's meetings:

```
Read: _today/tasks/master-task-list.md (PRIMARY SOURCE)
Glob: Accounts/*/04-Action-Items/*.md (SUPPLEMENTAL)
Grep: "- [ ]" (unchecked items)
```

Parse each action for:
- Action text
- Owner
- Due date (look for "Due:" or date patterns)

**CRITICAL: Filter by Owner**

Only include items in the main sections where **Owner = [Your Name]**. Items owned by others should NOT appear in the daily actions file - those are their responsibilities, not yours.

```python
def should_include_action(action):
    """
    Only include actions where [Your Name] is the owner.
    """
    owner = action.get('owner', '').lower()

    # Include if [Your Name] owns it
    if 'james' in owner:
        return True

    # Include if no owner specified (needs triage)
    if not owner or owner == 'unassigned':
        return True

    # Exclude if owned by someone else
    return False
```

**Owner patterns to exclude:**
- "Owner: Kim" → Kim's responsibility
- "Owner: Shilpa/Hayley" → Their responsibility
- "Owner: Alex" → Alex's responsibility
- "Owner: Abdul" → Abdul's responsibility
- Any name that isn't [Your Name]

Create `80-actions-due.md`:

```markdown
# Action Items - [Date]

## Overdue

- [ ] **[Action]** - [Account] - Due: [Date] (X days overdue)
  - **Context**: [Why this action exists]
  - **Requested by**: [Who initiated this]
  - **Source**: [file path - link to meeting/conversation]
  - **Impact if delayed**: [What's at risk]

## Due Today

- [ ] **[Action]** - [Account]
  - **Context**: [Why this action exists]
  - **Source**: [file path]

## Related to Today's Meetings

### [Account Name] (Meeting at [Time])
- [ ] **[Action]** - Due: [Date]
  - **Context**: [Why - so you can speak to it in the meeting]
  - **Requested by**: [Who to follow up with]
  - **Status update to share**: [What progress to report]

## Due This Week

- [ ] **[Action]** - [Account] - Due: [Date]
  - **Context**: [Why this action exists]
  - **Source**: [file path]

## Waiting On (Delegated)

| Who | What | Asked | Days | Context |
|-----|------|-------|------|---------|
| [Name] | [Action delegated to them] | [Date asked] | [Days waiting] | [Brief context] |

## Upcoming (Next 2 Weeks)

- [ ] **[Action]** - [Account] - Due: [Date]

## Completed Today

- [x] **[Action]** - [Account]
  - Completed: [Date]
  - Outcome: [Brief result]
```

**Note:** The "Waiting On" section tracks items [Your Name] delegated to others - these are outbound asks where [Your Name] is blocked until they respond. This is different from items others own independently.

**Parsing Action Context:**
When reading action files, look for these patterns to extract context:
- Lines after the action checkbox often contain context
- "Source:" or "From:" indicates origin meeting
- "Requested by:" or "@[name]" indicates requester
- "Owner:" indicates who is responsible (FILTER ON THIS)
- Related section headers indicate project/initiative

### Step 7: Look-Ahead for Agendas (3-4 Business Days)

Fetch next 5 calendar days:
```bash
python3 .config/google/google_api.py calendar list 5
```

**Business Day Calculation:**
```python
from datetime import datetime, timedelta

def get_business_days_ahead(start_date, num_days):
    """Return list of next N business days (skip Sat=5, Sun=6)"""
    business_days = []
    current = start_date
    while len(business_days) < num_days:
        current += timedelta(days=1)
        if current.weekday() < 5:  # Mon=0 through Fri=4
            business_days.append(current)
    return business_days

# Look 3-4 business days ahead
look_ahead_dates = get_business_days_ahead(datetime.now(), 4)
```

For each CUSTOMER meeting in look-ahead window:

**Check if agenda exists** (in priority order):
1. Calendar event description contains Google Doc link (`docs.google.com`) → EXISTS
2. Calendar event description has substantial text (>100 chars) → EXISTS
3. File exists matching pattern:
   ```bash
   ls Accounts/[Account]/02-Meetings/ | grep -i "agenda.*[YYYY-MM-DD]"
   ```
   → EXISTS
4. None of above → **NEEDS AGENDA**

**If agenda needed, invoke agenda-generator agent:**

```
Task(subagent_type="agenda-generator", prompt="
Generate a meeting agenda for:

Account: [Account Name]
Meeting: [Meeting Title from calendar]
Date: [Meeting Date]
Attendees: [List from calendar]

Account Context:
- Ring: [from Sheet]
- ARR: [from Sheet]
- Last Meeting: [from recent summary]
- Open Actions: [from action file]

Output to: _today/90-agenda-needed/[account-lowercase]-[date].md
")
```

**Agenda File Format:**
```
_today/90-agenda-needed/
├── acme-corp-2026-01-12.md
├── global-inc-2026-01-15.md
└── enterprise-co-2026-01-16.md
```

### Step 8: Generate Suggested Focus

Create `81-suggested-focus.md` based on:

1. **Priority 1: Pre-Meeting Prep**
   - Review customer prep docs before calls

2. **Priority 2: Overdue Items**
   - Actions past due date

3. **Priority 3: Agenda Sending**
   - Draft agendas in 90-agenda-needed/ to review and send

4. **Priority 4: Account Hygiene** (if time)
   - Stale dashboards
   - Accounts without recent contact

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
- Morning (high energy): Strategic prep, customer calls
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
| 11:00 AM | Agentforce Sync | Internal | - |
| 12:00 PM | **Acme Corp Monthly** | **Customer** | See 04-1200-customer-acme-corp-prep.md |

## Customer Meetings Today

### [Account] ([Time])
- **Ring**: [Ring]
- **ARR**: $[Amount]
- **Renewal**: [Date] ([X months])
- **Prep**: See [filename]

## Email - Needs Attention

### HIGH Priority ([count])

| From | Subject | Type | Notes |
|------|---------|------|-------|
| [sender] | [subject] | 🟢 OPPORTUNITY / 🟡 INFO / 🔴 RISK / 🔵 ACTION | Brief summary |

**See `83-email-summary.md` for full thread summaries including:**
- Conversation arc (who initiated, what's discussed, current status)
- Specific asks for [Your Name] (if any)
- Owner and recommended action

### Summary
- **Archived**: [X] (newsletters, GitHub, automated)
- **Labeled**: [X] (internal, P2, meetings)

*Full details: 83-email-summary.md*

## Action Items - Quick View

### Overdue
- [ ] [Action] - [Account] - Due: [Date]

### Due Today
- [ ] [Action] - [Account]

## Agenda Status (Next 3-4 Business Days)

| Meeting | Date | Status | Action |
|---------|------|--------|--------|
| [Account] | [Date] | ⚠️ Needs agenda | Draft in 90-agenda-needed/ |
| [Account] | [Date] | ✅ Exists | None |

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
├── 02-1100-internal-agentforce.md      # Second meeting
├── 03-1130-internal-donut.md           # Third meeting
├── 04-1200-customer-acme-corp-prep.md    # Customer meeting (FULL PREP)
├── 05-1245-personal-catchup.md         # Post-meeting block
├── 80-actions-due.md                   # Action items
├── 81-suggested-focus.md               # Focus suggestions
├── 83-email-summary.md                 # Email triage results
├── 90-agenda-needed/                   # Draft agendas
│   └── enterprise-co-jan-12.md            # Agenda to review and send
├── tasks/                              # Persistent task tracking
│   └── master-task-list.md             # Global task list (survives archive)
└── archive/                            # Previous days (processed by /week)
    └── 2026-01-07/
```

**Note:** `tasks/` directory is NOT archived daily - it persists and is updated by both `/today` and `/wrap`.

## Dependencies

**APIs:**
- Google Calendar (read + write)
- Google Sheets (read)
- Gmail (read, draft, labels)

**Skills/Workflows:**
- Meeting prep patterns defined in Step 4
- inbox/MEETING-TYPE-DETECTION.md - Classification patterns

**Agents:**
- agenda-generator - Draft agendas for look-ahead meetings

**Data Sources:**
- Google Sheet: `1edLlG0rkPj9QRT5mWQmCh_L-qy4We9fBLJ4haMZ_14g`
- Accounts/*/00-Index.md
- Accounts/*/01-Customer-Information/*-dashboard.md
- Accounts/*/02-Meetings/*.md
- Accounts/*/04-Action-Items/*.md

## Error Handling

**If Google Sheet unavailable:**
- Fall back to `_reference/tam-account-list.csv`
- Show warning in overview

**If account folder doesn't exist:**
- Create minimal prep with available calendar info
- Note: "Account folder not found - limited context"

**If action file is stale (>30 days):**
- Show warning: "⚠️ Action file last updated [date] - review for accuracy"

**If domain not recognized:**
- Classify as "external"
- Note unknown domain in overview for future mapping

## Calendar Write (Optional)

After generating suggested focus areas, offer to create calendar blocks:

### Step 10: Offer Calendar Blocks

**Use AskUserQuestion to confirm before creating:**

```python
suggested_blocks = []

# Post-meeting catch-up blocks (15 min after customer meetings)
for meeting in customer_meetings:
    if meeting['status'] == 'upcoming':
        catch_up_start = meeting['end']
        catch_up_end = meeting['end'] + timedelta(minutes=15)
        suggested_blocks.append({
            'title': f"Catch-up: {meeting['account']} notes",
            'start': catch_up_start,
            'end': catch_up_end,
            'purpose': 'Document meeting outcomes, update action items'
        })

# Agenda prep blocks (for meetings needing agendas)
for agenda in agendas_needed:
    # Schedule for 3 days before meeting
    prep_date = agenda['meeting_date'] - timedelta(days=3)
    suggested_blocks.append({
        'title': f"Agenda: {agenda['account']} ({agenda['meeting_date']})",
        'start': prep_date.replace(hour=9, minute=0),
        'end': prep_date.replace(hour=9, minute=30),
        'purpose': 'Review and send meeting agenda'
    })
```

**AskUserQuestion format:**
```
"I've identified [X] calendar blocks to create:

1. Catch-up: Acme Corp notes (12:30 PM - 12:45 PM)
   Purpose: Document meeting outcomes, update action items

2. Agenda: Enterprise Co (Jan 9, 9:00 AM)
   Purpose: Review and send meeting agenda

Which blocks would you like me to create?"

Options:
- Create all
- Create catch-up blocks only
- Create agenda prep blocks only
- None
```

**If confirmed, create events:**
```bash
python3 .config/google/google_api.py calendar create "[title]" "[start_datetime]" "[end_datetime]" "[description]"
```

**Event description template:**
```
Purpose: [purpose]
Related: [account/meeting]
Created by: /today command

DO NOT DELETE - auto-generated focus block
```

---

### Step 11: Chief of Staff Layer (Optional)

After completing all tactical steps, optionally invoke `/cos` to add executive-level decision support.

**When to invoke:**
- User explicitly requests `/cos`
- User runs `/today --cos` or `/today --full`

**What /cos adds:**
- **DECIDE**: Decisions needed with options + recommendation
- **WAITING ON**: Delegated items awaiting response
- **PORTFOLIO ALERTS**: Accounts/projects needing attention
- **CANCEL/PROTECT**: Meetings safe to skip with draft decline
- **SKIP TODAY**: Items that don't need attention

**Integration flow:**
```
1. /today completes Steps 0-10
2. /today creates *-daily-overview.md
3. If /cos requested:
   a. Parse master task list for decisions due + delegations stale
   b. Pull portfolio data from Google Sheets
   c. Scan calendar for cancelable meetings
   d. Generate 60-second scannable CoS briefing
4. Result: Tactical overview + strategic CoS briefing
```

**Output format:** Concise sections (DECIDE, WAITING ON, PORTFOLIO ALERTS, CANCEL/PROTECT, SKIP TODAY)

**See also:** `.claude/commands/cos.md` for full /cos workflow

---

## Multi-BU Learning

When user answers a multi-BU prompt, store the mapping for future automatic classification.

### BU Classification Cache

**Location**: `_reference/bu-classification-cache.json`

```json
{
  "version": 1,
  "mappings": [
    {
      "domain": "example-corp.com",
      "attendee_pattern": "john.doe@example-corp.com",
      "bu": "Enterprise-Division",
      "confidence": "user_confirmed",
      "created": "2026-01-08",
      "source": "/today classification"
    },
    {
      "domain": "global-inc.com",
      "title_pattern": "Marketing",
      "bu": "Marketing-Team",
      "confidence": "title_match",
      "created": "2026-01-08"
    }
  ],
  "default_bus": {
    "example-corp.com": "Enterprise-Division",
    "global-inc.com": "Corporate"
  }
}
```

### Classification Flow with Learning

```python
def classify_multi_bu_meeting(domain, attendees, title, cache):
    """
    1. Check for exact attendee match in cache
    2. Check for title pattern match in cache
    3. Use default BU for domain
    4. If no match, prompt user and save answer
    """

    # 1. Check attendee patterns
    for attendee in attendees:
        cached = find_attendee_mapping(cache, attendee)
        if cached:
            return cached['bu']

    # 2. Check title patterns
    for mapping in cache.get('mappings', []):
        if mapping.get('title_pattern') and mapping['title_pattern'].lower() in title.lower():
            return mapping['bu']

    # 3. Try default BU
    default = cache.get('default_bus', {}).get(domain)
    if default:
        return default, 'suggest_confirmation'

    # 4. Prompt user
    bu = prompt_user_for_bu(domain, attendees, title)

    # Save for future
    save_mapping(cache, {
        'domain': domain,
        'attendee_pattern': attendees[0],  # Primary attendee
        'bu': bu,
        'confidence': 'user_confirmed',
        'created': datetime.now().isoformat(),
        'source': '/today classification'
    })

    return bu
```

### Update Step 4 Classification

When classifying multi-BU meetings, add this check before prompting:

```python
# Load BU cache
bu_cache_path = '_reference/bu-classification-cache.json'
bu_cache = load_json(bu_cache_path) if file_exists(bu_cache_path) else {'mappings': [], 'default_bus': {}}

# For multi-BU domains (configured in workspace.json)
multi_bu_domains = load_multi_bu_config()
if domain in multi_bu_domains:
    bu = classify_multi_bu_meeting(domain, attendees, title, bu_cache)
    account = f"{parent_company}/{bu}"
```

---

## Related Commands

- `/cos` - Chief of Staff strategic layer (can be invoked after /today)
- `/wrap` - End-of-day closure and reconciliation
- `/week` - Monday weekly review
- `/month` - Monthly roll-up
- `/quarter` - Quarterly pre-population
- `/email-scan` - Email inbox triage (standalone, also integrated here)
