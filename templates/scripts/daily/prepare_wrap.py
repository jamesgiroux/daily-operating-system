#!/usr/bin/env python3
"""
Phase 1: Wrap Preparation Script
Handles all deterministic operations for /wrap command.

Outputs a JSON directive file that Claude uses for AI-required operations.

Operations performed:
1. Fetch today's calendar (completed meetings)
2. Check transcript existence in _inbox/
3. Parse week overview for prep status reconciliation
4. Parse master task list for due-today items
5. List inbox files for processing
6. Identify account files to update
7. Output structured directive for Claude

Usage:
    python3 _tools/prepare_wrap.py [--output FILE]
"""

import argparse
import json
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Any, Optional

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent / 'lib'))

from calendar_utils import fetch_calendar_events, filter_events_by_status, parse_event_time, check_google_api_available
from task_utils import (
    load_master_task_list, get_tasks_due_on, filter_tasks_by_owner,
    format_task_for_directive
)
from file_utils import (
    list_today_files, list_inbox_files, count_inbox_pending,
    check_yesterday_archive, TODAY_DIR, INBOX_DIR, ACCOUNTS_DIR
)
from meeting_utils import classify_meeting, load_domain_mapping, load_bu_cache

# Paths
VIP_ROOT = Path(__file__).parent.parent
DIRECTIVE_FILE = TODAY_DIR / ".wrap-directive.json"


def filter_completed_meetings(events: List[Dict], current_time: datetime) -> List[Dict]:
    """
    Filter to only completed meetings (end time < now).

    Args:
        events: List of calendar events
        current_time: Current datetime

    Returns:
        List of completed meeting events
    """
    completed = []

    for event in events:
        end_str = event.get('end', '')
        end_time = parse_event_time(end_str)

        if end_time and end_time < current_time:
            completed.append(event)

    return completed


def check_transcript_status(meeting: Dict, today_date: str, domain_mapping: Dict) -> Dict[str, Any]:
    """
    Check if a meeting's transcript has been processed.

    Args:
        meeting: Meeting classification dictionary
        today_date: Today's date string (YYYY-MM-DD)
        domain_mapping: Domain to account mapping

    Returns:
        Status dictionary with 'status', 'files', 'summary_exists', etc.
    """
    account = meeting.get('account', '')
    meeting_type = meeting.get('type', 'unknown')

    result = {
        'event_id': meeting.get('event_id'),
        'account': account,
        'type': meeting_type,
        'status': 'unknown',
        'transcript_in_inbox': None,
        'summary_exists': False,
        'summary_path': None,
        'actions_exists': False,
        'actions_path': None,
        'recording_expected': meeting_type == 'customer',  # Assume customer meetings have recordings
    }

    # Only check for customer meetings
    if meeting_type != 'customer' or not account:
        result['status'] = 'not_applicable'
        return result

    # Handle multi-BU account format
    if ' / ' in account:
        parent, bu = account.split(' / ', 1)
        account_path = ACCOUNTS_DIR / parent / bu
    else:
        account_path = ACCOUNTS_DIR / account

    # Check for summary in canonical location
    meetings_dir = account_path / '02-Meetings'
    if meetings_dir.exists():
        # Look for summary file with today's date
        summaries = list(meetings_dir.glob(f'{today_date}*.md'))
        if summaries:
            result['summary_exists'] = True
            result['summary_path'] = str(summaries[0])
            result['status'] = 'processed'

    # Check for actions file
    actions_dir = account_path / '04-Action-Items'
    if actions_dir.exists():
        actions = list(actions_dir.glob(f'{today_date}*.md'))
        if actions:
            result['actions_exists'] = True
            result['actions_path'] = str(actions[0])

    # Check inbox for unprocessed transcript
    if not result['summary_exists']:
        inbox_files = list_inbox_files()
        account_lower = account.lower().replace(' / ', '-').replace(' ', '-')

        for inbox_file in inbox_files:
            filename_lower = inbox_file.name.lower()
            if today_date in filename_lower and (
                account_lower in filename_lower or
                'transcript' in filename_lower
            ):
                result['transcript_in_inbox'] = str(inbox_file)
                result['status'] = 'in_inbox'
                break

    # If neither found
    if result['status'] == 'unknown':
        result['status'] = 'missing'

    return result


def parse_week_overview_prep_status() -> Dict[str, Dict]:
    """
    Parse week overview to get current prep status for meetings.

    Returns:
        Dictionary mapping account names to their prep info
    """
    week_overview = TODAY_DIR / "week-00-overview.md"

    if not week_overview.exists():
        return {}

    content = week_overview.read_text()
    prep_status = {}

    # Find the meetings table and parse it
    lines = content.split('\n')
    in_table = False
    headers = []

    for line in lines:
        if '| Day | Time |' in line or '|-----|------|' in line:
            in_table = True
            if '| Day |' in line:
                headers = [h.strip() for h in line.split('|')[1:-1]]
            continue

        if in_table and line.startswith('|'):
            cells = [c.strip() for c in line.split('|')[1:-1]]

            if len(cells) >= 4:
                # Extract account/meeting name and prep status
                account_col = None
                status_col = None

                for i, header in enumerate(headers):
                    if 'account' in header.lower() or 'meeting' in header.lower():
                        account_col = i
                    elif 'prep' in header.lower() and 'status' in header.lower():
                        status_col = i

                if account_col is not None and status_col is not None:
                    account = cells[account_col] if account_col < len(cells) else ''
                    status = cells[status_col] if status_col < len(cells) else ''

                    if account:
                        prep_status[account] = {
                            'status': status,
                            'row_data': cells
                        }

        elif in_table and not line.startswith('|') and line.strip():
            # End of table
            in_table = False

    return prep_status


def check_agenda_draft_status(account: str) -> Dict[str, Any]:
    """
    Check if an agenda draft was created and potentially sent.

    Args:
        account: Account name

    Returns:
        Status dictionary
    """
    agenda_dir = TODAY_DIR / "90-agenda-needed"

    result = {
        'draft_exists': False,
        'draft_path': None,
        'possibly_sent': False,
    }

    if not agenda_dir.exists():
        return result

    # Look for draft file
    account_slug = account.lower().replace(' / ', '-').replace(' ', '-')
    drafts = list(agenda_dir.glob(f'{account_slug}*.md'))

    if drafts:
        result['draft_exists'] = True
        result['draft_path'] = str(drafts[0])

    return result


def get_tasks_needing_update(today: datetime) -> List[Dict]:
    """
    Get tasks that might need status updates (due today or overdue).

    Args:
        today: Today's datetime

    Returns:
        List of tasks needing status prompts
    """
    task_data = load_master_task_list()
    all_tasks = task_data.get('tasks', [])

    # Filter to James's incomplete tasks
    james_tasks = filter_tasks_by_owner(all_tasks, 'james')
    incomplete_tasks = [t for t in james_tasks if not t.get('completed')]

    # Get due today and overdue
    due_today = get_tasks_due_on(incomplete_tasks, today)

    tasks_needing_update = []
    for task in due_today:
        tasks_needing_update.append({
            **format_task_for_directive(task),
            'prompt_type': 'status_update',
            'options': ['Completed', 'In Progress', 'Blocked', 'Deferred']
        })

    return tasks_needing_update


def identify_new_actions_from_meetings(completed_meetings: List[Dict]) -> List[Dict]:
    """
    Identify meetings that may have generated new action items.

    Args:
        completed_meetings: List of completed meeting classifications

    Returns:
        List of meetings that need action extraction
    """
    meetings_with_potential_actions = []

    for meeting in completed_meetings:
        if meeting.get('type') == 'customer':
            meetings_with_potential_actions.append({
                'event_id': meeting.get('event_id'),
                'account': meeting.get('account'),
                'title': meeting.get('title'),
                'needs_action_extraction': True,
            })

    return meetings_with_potential_actions


def main():
    """Main preparation orchestrator."""
    parser = argparse.ArgumentParser(description='Prepare wrap directive')
    parser.add_argument('--output', type=str, default=str(DIRECTIVE_FILE), help='Output file path')
    args = parser.parse_args()

    print("=" * 60)
    print("PHASE 1: WRAP PREPARATION")
    print("=" * 60)

    # Check Google API availability early
    api_available, api_reason = check_google_api_available()

    # Initialize directive
    now = datetime.now()
    today = now.date()
    today_str = today.isoformat()

    directive = {
        'command': 'wrap',
        'generated_at': now.isoformat(),
        'context': {
            'date': today_str,
            'day_of_week': today.strftime('%A'),
            'time': now.strftime('%H:%M'),
        },
        'api_status': {
            'available': api_available,
            'reason': api_reason if not api_available else None,
        },
        'completed_meetings': [],
        'transcript_status': [],
        'prep_reconciliation': [],
        'tasks_due_today': [],
        'new_actions_potential': [],
        'inbox_files': [],
        'archive_ready': False,
        'ai_tasks': [],
    }

    # Step 1: Fetch today's calendar
    print("\nStep 1: Fetching today's calendar...")
    events = []
    completed = []
    if api_available:
        events = fetch_calendar_events(days=1)
        print(f"  Found {len(events)} events today")

        # Filter to completed meetings
        completed = filter_completed_meetings(events, now)
        print(f"  Completed meetings: {len(completed)}")
    else:
        print(f"  Skipped (Google API unavailable: {api_reason})")
        print("  NOTE: Meeting reconciliation requires Google Calendar")

    # Step 2: Classify completed meetings
    print("\nStep 2: Classifying completed meetings...")
    domain_mapping = load_domain_mapping()
    bu_cache = load_bu_cache()

    classifications = []
    for event in completed:
        classification = classify_meeting(event, domain_mapping, bu_cache)
        classifications.append(classification)

        directive['completed_meetings'].append({
            'event_id': event.get('id'),
            'title': event.get('summary'),
            'start': event.get('start'),
            'end': event.get('end'),
            'type': classification.get('type'),
            'account': classification.get('account'),
        })

    customer_count = len([c for c in classifications if c.get('type') == 'customer'])
    print(f"  Customer meetings: {customer_count}")

    # Step 3: Check transcript status for customer meetings
    print("\nStep 3: Checking transcript status...")
    for classification in classifications:
        if classification.get('type') == 'customer':
            status = check_transcript_status(classification, today_str, domain_mapping)
            directive['transcript_status'].append(status)

            if status['status'] == 'processed':
                print(f"  ✅ {status['account']}: Processed")
            elif status['status'] == 'in_inbox':
                print(f"  ⚠️  {status['account']}: In inbox")
            elif status['status'] == 'missing':
                print(f"  ❌ {status['account']}: Missing")

    # Step 4: Parse week overview for prep reconciliation
    print("\nStep 4: Checking prep status reconciliation...")
    prep_status = parse_week_overview_prep_status()

    for classification in classifications:
        account = classification.get('account')
        if not account:
            continue

        prep_info = prep_status.get(account, {})
        current_status = prep_info.get('status', 'Unknown')

        # Check agenda draft status if applicable
        agenda_status = check_agenda_draft_status(account)

        reconciliation = {
            'account': account,
            'type': classification.get('type'),
            'current_prep_status': current_status,
            'new_status': '✅ Done',
            'agenda_draft_exists': agenda_status.get('draft_exists', False),
            'needs_prompt': False,
        }

        # Check if we need to prompt about agenda
        if '📅 Agenda' in current_status and agenda_status['draft_exists']:
            reconciliation['needs_prompt'] = True
            reconciliation['prompt_type'] = 'agenda_sent'

        directive['prep_reconciliation'].append(reconciliation)

    # Step 5: Get tasks needing status updates
    print("\nStep 5: Checking tasks due today...")
    tasks_due = get_tasks_needing_update(now)
    directive['tasks_due_today'] = tasks_due
    print(f"  Tasks needing status update: {len(tasks_due)}")

    # Step 6: Identify meetings that may have new actions
    print("\nStep 6: Identifying potential new actions...")
    new_actions = identify_new_actions_from_meetings(classifications)
    directive['new_actions_potential'] = new_actions
    print(f"  Meetings with potential actions: {len(new_actions)}")

    # Step 7: List inbox files
    print("\nStep 7: Checking inbox...")
    inbox_files = list_inbox_files()
    directive['inbox_files'] = [
        {
            'path': str(f),
            'name': f.name,
        }
        for f in inbox_files
    ]
    print(f"  Files in inbox: {len(inbox_files)}")

    # Step 8: Check if ready to archive
    print("\nStep 8: Checking archive readiness...")
    today_files = list_today_files()
    directive['archive_ready'] = len(today_files['daily']) > 0
    print(f"  Files to archive: {len(today_files['daily'])}")

    # Step 9: Generate AI task list
    print("\nStep 9: Generating AI task list...")

    # Task status prompts
    for task in directive['tasks_due_today']:
        directive['ai_tasks'].append({
            'type': 'prompt_task_status',
            'task': task,
            'priority': 'high',
        })

    # Transcript processing for in_inbox items
    for status in directive['transcript_status']:
        if status['status'] == 'in_inbox':
            directive['ai_tasks'].append({
                'type': 'process_transcript',
                'account': status['account'],
                'file': status['transcript_in_inbox'],
                'priority': 'high',
            })

    # Impact capture prompt
    if customer_count > 0:
        directive['ai_tasks'].append({
            'type': 'prompt_impact_capture',
            'meetings_count': customer_count,
            'priority': 'medium',
        })

    # Agenda sent prompts
    for recon in directive['prep_reconciliation']:
        if recon.get('needs_prompt'):
            directive['ai_tasks'].append({
                'type': 'prompt_agenda_sent',
                'account': recon['account'],
                'priority': 'low',
            })

    print(f"  AI tasks generated: {len(directive['ai_tasks'])}")

    # Write directive
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w') as f:
        json.dump(directive, f, indent=2, default=str)

    print("\n" + "=" * 60)
    print("✅ PHASE 1 COMPLETE")
    print("=" * 60)
    print(f"\nDirective written to: {output_path}")

    if not api_available:
        print(f"\n⚠️  Running in DEGRADED MODE (no Google API)")
        print(f"   Reason: {api_reason}")
        print(f"   Meeting reconciliation unavailable")
        print(f"   Task list and local files still processed")

    print(f"\nSummary:")
    print(f"  - Google API: {'✅ Available' if api_available else '❌ Unavailable'}")
    print(f"  - Completed meetings: {len(completed)}")
    print(f"  - Customer meetings: {customer_count}")
    print(f"  - Tasks needing status: {len(tasks_due)}")
    print(f"  - Inbox files: {len(inbox_files)}")
    print(f"  - AI tasks: {len(directive['ai_tasks'])}")

    print("\nNext: Claude prompts for task status and impact capture")
    print("Then: Run python3 _tools/deliver_wrap.py")

    return 0


if __name__ == "__main__":
    sys.exit(main())
