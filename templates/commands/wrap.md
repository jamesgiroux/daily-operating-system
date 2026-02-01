# /wrap - End of Day Closure

Close out the day with proper reconciliation and cleanup.

## When to Use

Run at end of workday (or when leaving for the day). This command:
- Verifies meeting notes/transcripts were processed
- Reconciles action items (completed, new, updated)
- Updates master task list
- Captures quick wins and impacts from today
- Archives today's files for tomorrow
- Prepares `_today/` for the next `/today` run

## Philosophy

**"Close the loops"** - Every important meeting should result in:
1. Notes/transcript processed (if applicable)
2. Summary in canonical location
3. Action items captured and tracked
4. Impact noted for weekly capture

**Zero guilt design** - If you skip /wrap one day, /today will catch the gaps tomorrow.

## Execution Steps

### Step 1: Identify Today's Meetings

```bash
python3 .config/google/google_api.py calendar list 1
```

Filter to customer and project meetings that already happened (check end time < now).

Build list:
```python
todays_completed_meetings = [
    {
        'account': 'ClientName',
        'time': '10:00 AM',
        'type': 'customer',
        'recording_expected': True,
        'prep_file': '_today/03-1000-customer-client-prep.md'
    },
    # ...
]
```

### Step 2: Verify Transcripts Are in Inbox

**Critical step:** Before closing the day, verify that all customer/project meeting transcripts are in `_inbox/`.

```python
def verify_transcripts_in_inbox(todays_meetings, today_date):
    """
    Check that all meetings that should have transcripts have files in _inbox/
    Returns: list of missing transcripts
    """
    missing = []

    for meeting in todays_meetings:
        # Skip meetings that don't typically have recordings
        if meeting['type'] in ['personal', 'internal-standup']:
            continue

        # Check if transcript exists in inbox
        patterns = [
            f"_inbox/*{today_date}*{meeting['account'].lower()}*transcript*.md",
            f"_inbox/*{today_date}*{meeting['account'].lower()}*call*.md",
            f"_inbox/*{today_date}*{meeting['account'].lower()}*.md"
        ]

        transcript_found = False
        for pattern in patterns:
            if glob.glob(pattern):
                transcript_found = True
                break

        if not transcript_found and meeting.get('recording_expected', True):
            missing.append({
                'meeting': meeting['title'],
                'account': meeting['account'],
                'time': meeting['time'],
                'action': 'Download from recording tool and save to _inbox/'
            })

    return missing
```

**Prompt user if transcripts missing:**
```
"Missing transcripts for today's meetings:

1. Client A call (10:00 AM)
   → Download from Gong/Fireflies/Zoom and save to _inbox/

2. Project X sync (2:00 PM)
   → Check if recording was made, download if available

Add transcripts now, or defer to tomorrow?

[Add now - I'll wait] / [Skip - will catch tomorrow] / [No recording was made]"
```

**Why this matters:**
- /today processes `_inbox/` each morning
- Missing transcripts = missing context for future meetings
- Missing transcripts = missing action items in master list
- Better to capture same-day while memory is fresh

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
            match_columns={'Day': format_day(today_date), 'Account/Meeting': meeting['name']},
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

**Resilience:** If /today wasn't run:
- /wrap still marks completed meetings as "✅ Done" in week overview
- Surfaces "no prep file found" as info (not error)
- System continues gracefully

### Step 3: Check Transcript/Notes Processing Status

For each important meeting:

```python
def check_meeting_status(meeting, today_date):
    """
    Check if meeting notes/transcript was processed
    Returns: 'processed', 'in_inbox', 'missing'
    """
    account = meeting['account']

    # 1. Check canonical location for summary
    summary_pattern = f"Accounts/{account}/meetings/{today_date}-*"
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
## Meeting Notes Status

| Meeting | Time | Status | Action |
|---------|------|--------|--------|
| Client A | 10:00 AM | Processed | None |
| Client B | 2:00 PM | In inbox | Process with /inbox |
| Client C | 4:00 PM | Missing | Check recording, upload transcript |
```

**If transcripts in inbox:**
```
"Found 2 unprocessed transcripts in _inbox/. Would you like me to process them now?"

Options:
- Process all
- Process individually
- Skip (will be flagged tomorrow)
```

### Step 4: Reconcile Action Items

**Sources to check:**
1. Today's meeting prep files (what was supposed to be discussed)
2. Today's meeting summaries (if processed, what was decided)
3. Master task list (what was due today)

**Reconciliation flow:**

```python
def reconcile_actions(today_date):
    """
    1. Load master task list
    2. For items due today: prompt for status update
    3. Extract new actions from today's meeting summaries
    4. Update master list
    """
    master_tasks = load_master_task_list()
    updates = []
    new_tasks = []

    # Items due today
    due_today = [t for t in master_tasks if t['due'] == today_date]
    for task in due_today:
        status = check_task_status(task)
        if status != task['status']:
            updates.append({'task': task, 'new_status': status})

    # New items from today's meetings
    for meeting in todays_completed_meetings:
        if meeting.get('summary_exists'):
            summary = read_meeting_summary(meeting)
            extracted_actions = extract_actions_from_summary(summary)
            new_tasks.extend(extracted_actions)

    return updates, new_tasks
```

**Prompt for status updates:**
```
"Action items due today:

1. [ ] Send updated documentation to Client A
   Account: Client A | Due: Today | Source: Dec 15 meeting

   Status? [Completed / In Progress / Blocked / Deferred]

2. [ ] Schedule follow-up with Client B team
   Account: Client B | Due: Today | Source: Dec 18 call

   Status? [Completed / In Progress / Blocked / Deferred]
"
```

**New actions extracted:**
```
"New action items from today's meetings:

From Client A call (10:00 AM):
1. [ ] Review Q1 roadmap proposal - Due: [suggest date]
2. [ ] Send case study examples - Due: [suggest date]

Add these to master task list? [Yes / Edit first / Skip]"
```

### Step 5: Update Master Task List

**Location:** `_today/tasks/master-task-list.md`

**Update operations:**
1. Mark completed items with completion date
2. Update status of in-progress items
3. Add new items with proper metadata

```python
def update_master_task_list(updates, new_tasks, today_date):
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
        master['tasks'].append(task)

    save_master_task_list(master)
```

### Step 6: Capture Daily Impact

Impact has two dimensions that should be captured separately:

1. **Customer/Client Outcomes** - Value delivered TO others
2. **Personal Impact** - What YOU moved forward

**Prompt for Customer Outcomes:**
```
"CUSTOMER OUTCOMES - What value did your customers/clients receive today?

Meetings completed: 3
- Client A (10:00 AM)
- Client B (2:00 PM)
- Client C (4:00 PM)

What did they gain, achieve, or avoid because of your work?

Examples:
- 'Client A now has visibility into X feature'
- 'Client B avoided delay through proactive coordination'
- 'Client C reduced risk through planning session'

[Enter customer outcomes or 'skip']"
```

**Prompt for Personal Impact:**
```
"PERSONAL IMPACT - What did you move forward today?

What did you personally accomplish, deliver, or influence?

Examples:
- 'Delivered roadmap presentation to Client A'
- 'Facilitated cross-team alignment on Project X'
- 'Completed 3 account dashboard refreshes'

[Enter personal accomplishments or 'skip']"
```

**If highlights provided:**
- Append to weekly impact capture file
- Tag for monthly roll-up

### Step 7: Update Dashboards (Optional)

If significant events occurred, prompt for dashboard updates:

```
"Significant events detected:

1. Client A: New risk identified (timeline concern)
   → Update dashboard risks section?

2. Client B: Win - demo successful
   → Add to Recent Wins?

Update dashboards now? [Yes for all / Select individually / Skip]"
```

### Step 8: Archive Today's Files

```bash
TODAY=$(date +%Y-%m-%d)

# Create archive directory
mkdir -p _today/archive/$TODAY

# Move daily files EXCEPT week-* files (NOT tasks/, archive/, or week-* files)
for f in _today/*.md; do
    filename=$(basename "$f")
    if [[ ! "$filename" == week-* ]]; then
        mv "$f" _today/archive/$TODAY/ 2>/dev/null
    fi
done

# Move agenda-needed contents
if [ -d "_today/90-agenda-needed" ] && [ "$(ls -A _today/90-agenda-needed/ 2>/dev/null)" ]; then
    mkdir -p _today/archive/$TODAY/90-agenda-needed
    mv _today/90-agenda-needed/*.md _today/archive/$TODAY/90-agenda-needed/ 2>/dev/null
fi
```

**IMPORTANT:**
- `tasks/` directory is NEVER archived - it persists.
- `week-*` files are NEVER archived by /wrap - they persist until /week archives them.

### Step 9: Check Inbox for New Files

```bash
ls -la _inbox/
```

**If files found:**
```
"New files detected in _inbox/:

1. 2026-01-08-client-transcript.md (transcript)
2. 2026-01-08-strategy-doc.pdf (document)

Process now or defer to tomorrow's /today?

[Process now / Defer]"
```

### Step 10: Generate Wrap Summary

Create `_today/archive/[TODAY]/wrap-summary.md`:

```markdown
# Day Wrap Summary - [Date]

## Meetings Completed
| Account | Time | Prep Status | Notes | Actions |
|---------|------|-------------|-------|---------|
| Client A | 10:00 AM | ✅ Done | Processed | 2 new |
| Client B | 2:00 PM | ✅ Done | Processed | 1 new |
| Client C | 4:00 PM | ✅ Done | Missing | - |
| Manager 1:1 | 1:00 PM | ✅ Done | N/A | - |

## Action Items Reconciled

### Completed Today
- [x] Send docs to Client B *(was due today)*
- [x] Follow up with Client A on timeline *(was overdue)*

### New Items Added
- [ ] Review Q1 roadmap proposal - Client A - Due: Jan 15
- [ ] Coordinate Engineering on project - Client B - Due: Jan 12

### Still Open (Carried Forward)
- [ ] Schedule team follow-up - Client A - Due: Jan 10

## Impacts Captured
- **Value Delivered**: Client B demo successful
- **Risk Identified**: Client A timeline may slip

## Agenda Tasks
| Meeting | Status | Notes |
|---------|--------|-------|
| Client B renewal (tomorrow) | ✅ Sent | Added to calendar description |
| Client C check-in (Wed) | ✏️ Draft ready | Review and send tomorrow |

## Inbox Status
- Processed: 0
- Deferred: 2 files

## Dashboard Updates
- Client A: Updated risks section
- Client B: Added recent win

---
*Wrapped at: [timestamp]*
*Ready for tomorrow's /today*
```

### Step 11: Display Completion Summary

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
DAY WRAP COMPLETE - [Date]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Meetings: 4 completed → all marked ✅ Done in week overview
  - 3 customer/project meetings
  - 1 internal meeting
Actions: 2 completed, 3 new added, 1 carried forward
Impact: 2 highlights captured
Agendas: 1 sent, 1 draft ready for tomorrow
Attention: 1 transcript missing (Client C)
Archived: Today's files moved to archive/2026-01-08/
Ready: _today/ prepared for tomorrow

Outstanding items for tomorrow:
- Send agenda for Client C check-in (✏️ Draft ready)
- Process Client C transcript when available
- 2 files in _inbox/ to process

Good night!
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
        ├── 03-1000-customer-client-prep.md
        ├── 80-actions-due.md
        ├── 83-email-summary.md
        ├── 90-agenda-needed/
        └── wrap-summary.md             # NEW: Day's wrap summary
```

## Dependencies

**APIs:**
- Google Calendar (read) - verify today's meetings

**Data Sources:**
- `_today/*.md` - today's generated files
- `_today/tasks/master-task-list.md` - task tracking
- `_inbox/` - unprocessed files

## Error Handling

**If no meetings today:**
- Skip transcript and summary checks
- Still reconcile actions and archive

**If master task list doesn't exist:**
- Create from template

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
- `/email-scan` - Email triage (can run standalone)
