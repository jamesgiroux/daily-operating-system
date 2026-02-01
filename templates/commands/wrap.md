# /wrap - End of Day Closure

Close out the day with proper reconciliation and cleanup.

## When to Use

Run at end of workday (or when leaving for the day). This command:
- Verifies meeting transcripts were processed
- Reconciles action items (completed, new, updated)
- Updates master task list and account-specific action files
- Captures quick wins and impacts from today
- Archives today's files for tomorrow
- Prepares `_today/` for the next `/today` run

## Philosophy

**"Close the loops"** - Every customer meeting should result in:
1. Transcript processed (if recorded)
2. Summary in canonical location
3. Action items captured and tracked
4. Dashboard updated if significant (wins, risks, decisions)
5. Impact noted for weekly capture

## Three-Phase Execution

This command uses a three-phase approach for efficiency:

```
┌─────────────────────────────────────────────────────────────────┐
│                    THREE-PHASE COMMAND FLOW                     │
├─────────────────────────────────────────────────────────────────┤
│  Phase 1: PREPARATION (Python Script)                           │
│  • Fetch today's calendar (completed meetings)                  │
│  • Check transcript existence in _inbox/                        │
│  • Parse week overview for prep status                          │
│  • Parse master task list for due-today items                   │
│  • Output: _today/.wrap-directive.json                          │
│                                                                 │
│  Phase 2: AI ENRICHMENT (Claude)                                │
│  • Prompt for task status updates                               │
│  • Capture daily impact (customer + personal)                   │
│  • Prompt for agenda sent confirmation                          │
│  • Offer transcript processing                                  │
│                                                                 │
│  Phase 3: DELIVERY (Python Script)                              │
│  • Archive today's files                                        │
│  • Update week overview prep status                             │
│  • Update master task list                                      │
│  • Write wrap summary                                           │
└─────────────────────────────────────────────────────────────────┘
```

## Execution Steps

### Phase 1: Run Preparation Script

**ALWAYS RUN THIS FIRST:**

```bash
python3 /Users/jamesgiroux/Documents/VIP/_tools/prepare_wrap.py
```

This script performs all deterministic operations:
- Fetches today's calendar and filters to completed meetings
- Classifies each meeting and checks transcript status
- Parses week overview for current prep status
- Gets tasks due today needing status updates
- Lists inbox files for potential processing

**Output:** `_today/.wrap-directive.json` containing structured data for Phase 2.

### Phase 2: AI Enrichment (Claude Tasks)

After the script completes, read the directive and execute interactive prompts:

```bash
# Read the directive
cat /Users/jamesgiroux/Documents/VIP/_today/.wrap-directive.json
```

**Execute these AI tasks from directive['ai_tasks']:**

#### Task Status Prompts (`prompt_task_status`)

For each task due today, prompt:
```
"Action items due today:

1. [ ] [Task title] - [Account]
   Due: Today | Source: [source]

   Status? [Completed / In Progress / Blocked / Deferred]"
```

Store responses in the directive for Phase 3.

#### Impact Capture (`prompt_impact_capture`)

Prompt for two-sided impact capture:

**Customer Outcomes:**
```
"CUSTOMER OUTCOMES - What value did your customers receive today?

Customer meetings completed: [count]
- [Account 1] ([time])
- [Account 2] ([time])

What did customers gain, achieve, or avoid because of your work?
[Enter customer outcomes or 'skip']"
```

**Personal Impact:**
```
"PERSONAL IMPACT - What did you move forward today?

What did you personally accomplish, deliver, or influence?
[Enter personal accomplishments or 'skip']"
```

#### Transcript Processing Offer

If directive shows transcripts in inbox:
```
"Found [X] unprocessed transcripts in _inbox/. Process them now?

Options:
- Process all
- Process individually
- Skip (will be flagged tomorrow)"
```

If "Process" selected, invoke `/inbox-processing`.

#### Agenda Sent Confirmation (`prompt_agenda_sent`)

For meetings where agenda draft existed:
```
"Agenda status for [Account] meeting:
Draft: _today/90-agenda-needed/[filename]
Did you send the agenda? [Yes / No / N/A]"
```

### Phase 3: Run Delivery Script

**AFTER completing AI prompts:**

```bash
python3 /Users/jamesgiroux/Documents/VIP/_tools/deliver_wrap.py
```

This script:
- Updates week overview with completed meeting status (✅ Done)
- Updates master task list with status changes
- Archives today's files to `archive/YYYY-MM-DD/`
- Writes wrap summary to archive

**Options:**
- `--skip-archive` - Don't archive files
- `--keep-directive` - Keep directive file for debugging
- `--ai-outputs FILE` - JSON file with AI-captured outputs (impact, etc.)

**Tip:** To pass AI outputs to delivery, save them to a JSON file:
```json
{
  "customer_outcomes": "Nielsen now has visibility into...",
  "personal_impact": "Delivered AI roadmap presentation...",
  "task_updates": [{"title": "Task 1", "new_status": "Completed"}]
}
```

---

## Legacy Reference: Detailed Process

The following sections are reference material for Phase 2 AI enrichment.

### Step 1: Identify Today's Meetings

```bash
python3 /Users/jamesgiroux/Documents/VIP/.config/google/google_api.py calendar list 1
```

Filter to customer and project meetings that already happened (check end time < now).

Build list:
```python
todays_completed_meetings = [
    {
        'account': 'Nielsen',
        'time': '10:00 AM',
        'type': 'customer',
        'recording_expected': True,  # Check calendar event for Zoom/Meet link
        'prep_file': '_today/03-1000-customer-nielsen-prep.md'
    },
    # ...
]
```

### Step 2: Check Transcript Processing

For each customer meeting:

```python
def check_transcript_status(meeting, today_date):
    """
    Check if meeting transcript was processed
    Returns: 'processed', 'in_inbox', 'missing'
    """
    account = meeting['account']

    # 1. Check canonical location
    summary_pattern = f"Accounts/{account}/02-Meetings/{today_date}-*"
    if glob.glob(summary_pattern):
        return 'processed'

    # 2. Check inbox for unprocessed transcript
    inbox_pattern = f"_inbox/*{today_date}*{account.lower()}*"
    if glob.glob(inbox_pattern):
        return 'in_inbox'

    # 3. Check if meeting had recording
    if not meeting.get('recording_expected'):
        return 'no_recording_expected'

    return 'missing'
```

**Surface status:**
```markdown
## Transcript Status

| Meeting | Time | Status | Action |
|---------|------|--------|--------|
| Nielsen | 10:00 AM | ✅ Processed | None |
| Heroku | 2:00 PM | ⚠️ In inbox | Process with /inbox-processing |
| Blackstone | 4:00 PM | ❌ Missing | Check recording, upload transcript |
```

**If transcripts in inbox:**
```
"Found 2 unprocessed transcripts in _inbox/. Would you like me to process them now?"

Options:
- Process all
- Process individually
- Skip (will be flagged tomorrow)
```

### Step 2B: Prep Completion Reconciliation

For meetings that happened today, update week overview and optionally prompt for agenda status.

**Key principle:** Only ask questions when the answer can't be determined from existing data.

#### Auto-Detection (No User Input Required)

```python
def reconcile_prep_status(completed_meetings, today_date):
    """
    Auto-update prep status for completed meetings.
    Detects meeting completion from calendar (past end time).
    """
    week_overview_path = '_today/week-00-overview.md'

    if not os.path.exists(week_overview_path):
        return []  # No week overview to update

    week_overview = read_file(week_overview_path)
    agenda_tasks_pending = []

    for meeting in completed_meetings:
        # Update week overview to mark meeting as complete
        week_overview = update_table_row(
            week_overview,
            match_columns={'Day': format_day(today_date), 'Account/Meeting': meeting['account']},
            update_column='Prep Status',
            new_value='✅ Done'
        )

        # Check if there was an agenda task that wasn't completed
        if meeting.get('agenda_owner') == 'you':
            # Check if agenda draft exists
            agenda_file_pattern = f"_today/90-agenda-needed/{meeting['account'].lower()}*"
            agenda_files = glob.glob(agenda_file_pattern)

            if agenda_files:
                # Draft exists - check if it was sent
                # Look for evidence: calendar event description updated with agenda link
                event_details = get_calendar_event(meeting['event_id'])
                description = event_details.get('description', '')

                if 'agenda' in description.lower() and ('docs.google.com' in description or len(description) > 200):
                    # Agenda appears to have been added to calendar
                    pass  # Auto-mark as complete
                else:
                    # Draft exists but may not have been sent
                    agenda_tasks_pending.append(meeting)

    write_file(week_overview_path, week_overview)
    return agenda_tasks_pending
```

#### Smart Prompting (Only When Needed)

If agenda tasks are pending and can't be auto-resolved:

```python
if agenda_tasks_pending:
    # Only ask about unresolved agenda items
    prompt = "Agenda status for completed meetings:\n\n"

    for meeting in agenda_tasks_pending:
        prompt += f"• **{meeting['account']}** ({meeting['time']})\n"
        prompt += f"  Draft: _today/90-agenda-needed/{meeting['draft_file']}\n"
        prompt += f"  Did you send the agenda?\n\n"

    # Use AskUserQuestion with options
    options = [
        {"label": "Sent all", "description": "I sent agendas for all listed meetings"},
        {"label": "Some sent", "description": "Let me specify which ones"},
        {"label": "Didn't send", "description": "Customer handled or not needed"},
        {"label": "Skip", "description": "I'll address this later"}
    ]
```

**Auto-Completion Logic:**
If agenda file exists AND calendar event description now contains agenda link:
→ Mark task as completed automatically in master-task-list.md

```python
def auto_complete_agenda_tasks(agenda_tasks_pending, master_tasks):
    """
    Check if agenda tasks can be auto-completed based on calendar evidence.
    """
    for meeting in agenda_tasks_pending:
        task_id = f"{meeting['date']}-agenda-*"
        task = find_task_by_pattern(master_tasks, task_id, meeting['account'])

        if task:
            # Check calendar for evidence agenda was sent
            event = get_calendar_event(meeting['event_id'])
            if event_has_agenda_link(event):
                mark_task_completed(task, today_date)
```

**Resilience:** If /today wasn't run:
- /wrap still marks completed meetings as "✅ Done" in week overview
- Surfaces "no prep file found" as info (not error)
- System continues gracefully

### Step 3: Reconcile Action Items

**Sources to check:**
1. Today's meeting prep files (what was supposed to be discussed)
2. Today's meeting summaries (if processed, what was decided)
3. Master task list (what was due today)
4. Account action files (distributed tracking)

**Reconciliation flow:**

```python
def reconcile_actions(today_date):
    """
    1. Load master task list
    2. For items due today: prompt for status update
    3. Extract new actions from today's meeting summaries
    4. Update master list
    5. Sync to account-specific files
    """

    master_tasks = load_master_task_list()
    updates = []
    new_tasks = []

    # Items due today
    due_today = [t for t in master_tasks if t['due'] == today_date]
    for task in due_today:
        # Check if completed (look for checkbox change or ask)
        status = check_task_status(task)
        if status != task['status']:
            updates.append({'task': task, 'new_status': status})

    # New items from today's meetings
    for meeting in todays_completed_meetings:
        if meeting['summary_exists']:
            summary = read_meeting_summary(meeting)
            extracted_actions = extract_actions_from_summary(summary)
            new_tasks.extend(extracted_actions)

    return updates, new_tasks
```

**Prompt for status updates:**
```
"Action items due today:

1. [ ] Send updated connector documentation to Heroku
   Account: Heroku | Due: Today | Source: Dec 15 meeting

   Status? [Completed / In Progress / Blocked / Deferred]

2. [ ] Schedule follow-up with Nielsen DevOps team
   Account: Nielsen | Due: Today | Source: Dec 18 call

   Status? [Completed / In Progress / Blocked / Deferred]
"
```

**New actions extracted:**
```
"New action items from today's meetings:

From Nielsen call (10:00 AM):
1. [ ] Review Q1 roadmap proposal - Due: [suggest date]
2. [ ] Send case study examples - Due: [suggest date]

From Heroku call (2:00 PM):
1. [ ] Coordinate with Engineering on connector - Due: [suggest date]

Add these to master task list? [Yes / Edit first / Skip]"
```

### Step 4: Update Master Task List

**Location:** `_today/tasks/master-task-list.md`

**Update operations:**
1. Mark completed items with completion date
2. Update status of in-progress items
3. Add new items with proper metadata
4. Sync changes to account-specific files

```python
def update_master_task_list(updates, new_tasks):
    """
    Update master list and sync to distributed files
    """
    master = load_master_task_list()

    # Apply updates
    for update in updates:
        task = find_task(master, update['task']['id'])
        task['status'] = update['new_status']
        if update['new_status'] == 'completed':
            task['completed_date'] = today_date

    # Add new tasks
    for task in new_tasks:
        task['id'] = generate_task_id()
        task['created'] = today_date
        task['source'] = task.get('source', 'manual')
        master['tasks'].append(task)

    # Save master
    save_master_task_list(master)

    # Sync to account files
    for account in get_affected_accounts(updates + new_tasks):
        sync_to_account_file(account, master)
```

### Step 5: Capture Daily Impact (Two-Sided)

Impact has two dimensions that should be captured separately:

1. **Customer Outcomes** - Value delivered TO customers (feeds EBRs, renewals, value stories)
2. **Personal Impact** - What YOU moved forward (feeds performance reviews, career narrative)

**Prompt for Customer Outcomes:**
```
"CUSTOMER OUTCOMES - What value did your customers receive today?

Customer meetings completed: 3
- Nielsen (10:00 AM)
- Heroku (2:00 PM)
- Blackstone (4:00 PM)

What did customers gain, achieve, or avoid because of your work?

Examples:
- 'Nielsen now has visibility into Parse.ly for media vertical'
- 'Heroku avoided connector delay through proactive coordination'
- 'Blackstone reduced risk through upgrade planning session'

[Enter customer outcomes or 'skip']"
```

**Prompt for Personal Impact:**
```
"PERSONAL IMPACT - What did you move forward today?

What did you personally accomplish, deliver, or influence?

Examples:
- 'Delivered AI roadmap presentation to Nielsen'
- 'Facilitated cross-functional alignment on Agentforce'
- 'Completed 3 account dashboard refreshes'
- 'Influenced pricing strategy through customer feedback'

[Enter personal accomplishments or 'skip']"
```

**If highlights provided:**
- Append customer outcomes to "Customer Outcomes" section of weekly impact
- Append personal impact to "Personal Impact" section of weekly impact
- Tag for monthly roll-up

```python
def capture_daily_impact(customer_outcomes, personal_impact, today_date):
    """
    Add today's highlights to weekly impact capture - two sections
    """
    week_num = get_week_number(today_date)
    impact_file = f"Leadership/02-Performance/Weekly-Impact/{today_date[:4]}-W{week_num:02d}-impact-capture.md"

    if os.path.exists(impact_file):
        if customer_outcomes:
            append_to_section(impact_file, "Customer Outcomes", today_date, customer_outcomes)
        if personal_impact:
            append_to_section(impact_file, "Personal Impact", today_date, personal_impact)
    else:
        # Weekly template not created yet - store in temp location
        store_pending_impact(today_date, customer_outcomes, personal_impact)
```

**The Distinction:**

| Customer Outcomes | Personal Impact |
|-------------------|-----------------|
| Customer-centric | You-centric |
| "They now have..." | "I delivered..." |
| Feeds EBRs, renewals, value stories | Feeds performance reviews, career narrative |
| Observable by customer | Observable by you/manager |

### Step 5B: Coaching Reflection

For meetings where coaching commitments were flagged (from /today's Step 9B), prompt for reflection.

**Source:** `Leadership/03-Development/active-coaching-commitments.md`
**Today's coaching opportunities:** From daily overview "Coaching Opportunities Today" section

**Reflection prompts:**

```python
def generate_coaching_reflection(daily_overview, commitments_file):
    """
    Generate reflection prompts for today's coaching opportunities
    """
    opportunities = extract_coaching_opportunities(daily_overview)

    if not opportunities:
        return None  # No coaching flagged today

    prompts = []
    for opp in opportunities:
        prompts.append({
            'commitment': opp['commitment'],
            'meeting': opp['meeting'],
            'questions': [
                f"Did you practice '{opp['practice']}'?",
                "How did it go? (Natural / Forced / Forgot)",
                "What would you do differently?",
                "Any insight about the commitment itself?"
            ]
        })

    return prompts
```

**Interactive prompt:**

```
"Coaching Reflection:

Today you had 1 coaching opportunity flagged:

1. Driver/Passenger Dynamic - Amy/James: Blackstone (11am)
   Practice: Ask 'what do you need from me?' first

   How did it go?
   - [ ] Practiced naturally
   - [ ] Practiced but felt forced
   - [ ] Forgot to practice
   - [ ] N/A (meeting canceled/rescheduled)

   Brief reflection (optional): [text input]
"
```

**If reflection provided:**
- Append to weekly impact capture under "Coaching Progress"
- If significant insight, offer to update working agreement or commitment file

**Output in wrap summary:**

```markdown
## Coaching Reflection

| Commitment | Meeting | Result | Notes |
|------------|---------|--------|-------|
| Driver/Passenger | Amy (11am) | Practiced naturally | Asked what she needed first - felt more collaborative |

*Reflection captured in weekly impact file.*
```

**Skip if:**
- No coaching opportunities were flagged today
- User declines reflection prompt

### Step 5C: Update Clay Relationship Notes

For each customer meeting completed today, offer to update Clay with relationship context.

**Prerequisites:**
- Clay MCP must be available
- Meeting must have had external attendees

**Flow:**

```python
def update_clay_after_meetings(completed_meetings, today_date):
    """
    For each customer meeting, offer to:
    1. Add notes to known contacts
    2. Create new contacts for unknowns
    """
    clay_updates = []

    for meeting in completed_meetings:
        if meeting['type'] != 'customer':
            continue

        external_attendees = meeting.get('external_attendees', [])
        meeting_summary = meeting.get('summary_key_points', '')

        for attendee in external_attendees:
            # Search Clay for this contact
            contact = mcp__clay__searchContacts(
                query=attendee['name'],
                company_name=[attendee['company']] if attendee.get('company') else [],
                limit=1
            )

            if contact:
                clay_updates.append({
                    'action': 'add_note',
                    'contact_id': contact['id'],
                    'contact_name': contact['name'],
                    'meeting': meeting['title'],
                    'account': meeting['account'],
                    'note_content': generate_meeting_note(meeting, attendee)
                })
            else:
                clay_updates.append({
                    'action': 'create_contact',
                    'email': attendee['email'],
                    'name': attendee['name'],
                    'company': attendee.get('company'),
                    'meeting': meeting['title']
                })

    return clay_updates

def generate_meeting_note(meeting, attendee):
    """Generate a concise note for Clay"""
    return f"""Meeting: {meeting['title']} ({meeting['date']})
Account: {meeting['account']}
Topics: {meeting.get('key_topics', 'N/A')}
Their stance: {meeting.get('attendee_notes', {}).get(attendee['email'], 'N/A')}
Follow-up: {meeting.get('next_steps', 'N/A')}"""
```

**Interactive prompt:**

```
"Clay Relationship Updates:

Today's customer meetings:

1. Nielsen Monthly Sync (10:00 AM)
   Known contacts:
   - Gustavo Rodrigues (contact #12345)
     → Add note about today's discussion?

   Unknown contacts:
   - new.person@nielsen.com
     → Create in Clay?

2. Cox Automotive Parse.ly Demo (11:00 AM)
   Known contacts:
   - Tomasz Nowakowski (contact #215633714)
     → Add note about Parse.ly demo feedback?

What would you like to do?
- [Add notes to all known contacts]
- [Select individually]
- [Skip Clay updates]
"
```

**Note creation:**

```python
# For known contacts
mcp__clay__createNote(
    contact_id=contact_id,
    content=f"""Meeting: {meeting_title} ({today_date})
Account: {account_name}
Key topics: {topics}
Notes: {attendee_notes}
Follow-up: {next_steps}"""
)

# For unknown contacts (if user confirms)
mcp__clay__createContact(
    email=[email],
    first_name=first_name,
    last_name=last_name,
    organization=company,
    title=title_if_known
)
```

**Output in wrap summary:**

```markdown
## Clay Updates

| Contact | Action | Meeting |
|---------|--------|---------|
| Tomasz Nowakowski | ✅ Note added | Cox Parse.ly Demo |
| Gustavo Rodrigues | ✅ Note added | Nielsen Monthly |
| new.person@nielsen.com | ➕ Created | Nielsen Monthly |
| jane.doe@salesforce.com | ⏭️ Skipped | Salesforce DX Sync |
```

**Skip if:**
- No customer meetings today
- Clay MCP not available
- User declines updates

### Step 6: Update Account Dashboards (Optional)

If significant events occurred, prompt for dashboard updates:

```
"Significant events detected:

1. Nielsen: New risk identified (migration timeline concern)
   → Update dashboard risks section?

2. Heroku: Win - connector demo successful
   → Add to Recent Wins?

3. Blackstone: Executive access - met with CTO
   → Update stakeholder map?

Update dashboards now? [Yes for all / Select individually / Skip]"
```

**If yes:**
- Open relevant dashboard files
- Add quick entries to appropriate sections
- Note "Last Updated" date

### Step 7: Archive Today's Files

```bash
TODAY=$(date +%Y-%m-%d)

# Create archive directory
mkdir -p /Users/jamesgiroux/Documents/VIP/_today/archive/$TODAY

# Move daily files EXCEPT week-* files (NOT tasks/, archive/, or week-* files)
# IMPORTANT: Preserve week-* files - they persist until /week archives them
for f in /Users/jamesgiroux/Documents/VIP/_today/*.md; do
    filename=$(basename "$f")
    if [[ ! "$filename" == week-* ]]; then
        mv "$f" /Users/jamesgiroux/Documents/VIP/_today/archive/$TODAY/ 2>/dev/null
    fi
done

# Move agenda-needed contents
if [ -d "/Users/jamesgiroux/Documents/VIP/_today/90-agenda-needed" ] && [ "$(ls -A /Users/jamesgiroux/Documents/VIP/_today/90-agenda-needed/ 2>/dev/null)" ]; then
    mkdir -p /Users/jamesgiroux/Documents/VIP/_today/archive/$TODAY/90-agenda-needed
    mv /Users/jamesgiroux/Documents/VIP/_today/90-agenda-needed/*.md /Users/jamesgiroux/Documents/VIP/_today/archive/$TODAY/90-agenda-needed/ 2>/dev/null
fi
```

**IMPORTANT:**
- `tasks/` directory is NEVER archived - it persists.
- `week-*` files are NEVER archived by /wrap - they persist until /week archives them.

### Step 8: Check Inbox for New Files

```bash
ls -la /Users/jamesgiroux/Documents/VIP/_inbox/
```

**If files found:**
```
"New files detected in _inbox/:

1. 2026-01-08-salesforce-transcript.md (transcript)
2. 2026-01-08-strategy-doc.pdf (document)

Process now or defer to tomorrow's /today?

[Process now / Defer]"
```

### Step 9: Generate Wrap Summary

Create `_today/archive/[TODAY]/wrap-summary.md`:

```markdown
# Day Wrap Summary - [Date]

## Meetings Completed
| Account | Time | Transcript | Summary | Actions |
|---------|------|------------|---------|---------|
| Nielsen | 10:00 AM | ✅ | ✅ | 2 new |
| Heroku | 2:00 PM | ✅ | ✅ | 1 new |
| Blackstone | 4:00 PM | ⚠️ Missing | ❌ | - |

## Action Items Reconciled

### Completed Today
- [x] Send connector docs to Heroku *(was due today)*
- [x] Follow up with Nielsen on timeline *(was overdue)*

### New Items Added
- [ ] Review Q1 roadmap proposal - Nielsen - Due: Jan 15
- [ ] Coordinate Engineering on connector - Heroku - Due: Jan 12

### Still Open (Carried Forward)
- [ ] Schedule DevOps follow-up - Nielsen - Due: Jan 10

## Impacts Captured
- **Value Delivered**: Heroku connector demo successful
- **Risk Identified**: Nielsen migration timeline may slip

## Inbox Status
- Processed: 0
- Deferred: 2 files

## Clay Updates
| Contact | Action | Meeting |
|---------|--------|---------|
| Tomasz Nowakowski | ✅ Note added | Cox Parse.ly Demo |
| new.person@nielsen.com | ➕ Created | Nielsen Monthly |

## Dashboard Updates
- Nielsen: Updated risks section
- Heroku: Added recent win

---
*Wrapped at: [timestamp]*
*Ready for tomorrow's /today*
```

### Step 10: Display Completion Summary

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
DAY WRAP COMPLETE - [Date]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Meetings: 3 completed, 2 transcripts processed
✅ Actions: 2 completed, 3 new added, 1 carried forward
✅ Impact: 2 highlights captured
✅ Clay: 2 notes added, 1 contact created
⚠️ Attention: 1 transcript missing (Blackstone)
✅ Archived: Today's files moved to archive/2026-01-08/
✅ Ready: _today/ prepared for tomorrow

Outstanding items for tomorrow:
- Process Blackstone transcript when available
- 2 files in _inbox/ to process

Good night! 🌙
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Output Structure

After running `/wrap`:

```
_today/
├── tasks/                              # PERSISTS
│   └── master-task-list.md             # Updated with reconciliation
└── archive/
    └── 2026-01-08/                     # Today's archived files
        ├── 00-overview.md
        ├── 03-1000-customer-nielsen-prep.md
        ├── 80-actions-due.md
        ├── 83-email-summary.md
        ├── 90-agenda-needed/
        └── wrap-summary.md             # NEW: Day's wrap summary
```

## Dependencies

**APIs:**
- Google Calendar (read) - verify today's meetings
- Google Sheets (read) - account context
- Clay MCP (read/write) - relationship notes and contact creation

**MCP Tools Used:**
- `mcp__clay__searchContacts` - find contacts by name/company
- `mcp__clay__getContact` - get contact details
- `mcp__clay__createNote` - add meeting notes to contacts
- `mcp__clay__createContact` - create new contacts for unknown attendees

**Data Sources:**
- `_today/*.md` - today's generated files
- `_today/tasks/master-task-list.md` - task tracking
- `Accounts/*/04-Action-Items/` - distributed action files
- `Accounts/*/02-Meetings/` - meeting summaries
- `_inbox/` - unprocessed files

**Skills/Workflows:**
- inbox-processing - if processing transcripts
- daily-csm/ACTION-TRACKING - action item patterns

## Error Handling

**If no meetings today:**
- Skip transcript and summary checks
- Still reconcile actions and archive

**If master task list doesn't exist:**
- Create from template
- Populate with actions scanned from account files

**If user declines all prompts:**
- Still archive files
- Note skipped items in wrap summary

**If run multiple times same day:**
- Detect existing archive for today
- Ask: "Already wrapped today. Re-wrap? [Yes / No]"

## Related Commands

- `/today` - Morning setup (inverse of /wrap)
- `/week` - Monday review (processes archives)
- `/month` - Monthly roll-up
- `/quarter` - Quarterly review
- `/email-scan` - Email triage (can run standalone)
