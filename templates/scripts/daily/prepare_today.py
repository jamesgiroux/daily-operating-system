#!/usr/bin/env python3
"""
Phase 1: Today Preparation Script
Handles all deterministic operations for /today command.

Outputs a JSON directive file that Claude uses for AI-required operations.

Operations performed:
1. Date/week calculations
2. Archive yesterday's files (if needed)
3. Fetch calendar events
4. Classify meetings by type/domain
5. Fetch account data from Google Sheet
6. Aggregate action items from master list
7. Scan for existing files in _today/
8. Identify agenda needs for look-ahead
9. Fetch unread emails
10. Output structured directive for Claude

Usage:
    python3 _tools/prepare_today.py [--skip-archive] [--skip-email] [--skip-dashboard] [--output FILE]
"""

import argparse
import json
import subprocess
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Any, Optional

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent / 'lib'))

from calendar_utils import (
    fetch_calendar_events, filter_events_by_status, get_week_dates,
    get_business_days_ahead, format_time_for_display, format_time_for_filename,
    calculate_meeting_gaps, check_google_api_available
)
from task_utils import (
    load_master_task_list, get_tasks_due_on, get_overdue_tasks,
    get_tasks_for_accounts, filter_tasks_by_owner, format_task_for_directive,
    extract_waiting_on
)
from file_utils import (
    ensure_today_structure, archive_daily_files, check_yesterday_archive,
    list_today_files, list_inbox_files, count_inbox_pending,
    find_account_dashboard, find_recent_meeting_summaries,
    find_account_action_file, get_file_age_days, check_yesterday_transcripts
)
from meeting_utils import (
    classify_meeting, load_domain_mapping, load_bu_cache,
    fetch_account_data, build_account_lookup, format_classification_for_directive,
    get_internal_domains
)
from dashboard_utils import (
    is_dashboard_autostart_enabled, start_dashboard_background
)


def extract_json_from_output(output: str) -> str:
    """
    Extract JSON from output that may contain warning messages.

    The Google API script may print Python warnings before the JSON output.
    This function finds the first JSON array or object and returns it.

    Args:
        output: Raw stdout that may contain warnings + JSON

    Returns:
        The JSON portion of the output, or empty string if not found
    """
    if not output:
        return ""

    # Find the first [ or { which starts JSON
    for i, char in enumerate(output):
        if char == '[' or char == '{':
            return output[i:]

    return output


# Paths
VIP_ROOT = Path(__file__).parent.parent
TODAY_DIR = VIP_ROOT / "_today"
GOOGLE_API_PATH = VIP_ROOT / ".config/google/google_api.py"
DIRECTIVE_FILE = TODAY_DIR / ".today-directive.json"


def run_resilience_checks(yesterday: datetime) -> List[Dict[str, str]]:
    """
    Check for issues from yesterday that need attention.

    Returns:
        List of warning dictionaries with 'level', 'message', 'action'
    """
    warnings = []

    # 1. Check if /wrap ran yesterday (archive should exist)
    if not check_yesterday_archive(yesterday):
        # Only warn on weekdays
        if yesterday.weekday() < 5:
            warnings.append({
                'level': 'warning',
                'message': f"Yesterday's files not archived - /wrap may not have run",
                'action': 'Run /wrap to reconcile, or archive manually'
            })

    # 2. Check for unprocessed transcripts from yesterday
    unprocessed = check_yesterday_transcripts(yesterday)
    if unprocessed:
        warnings.append({
            'level': 'warning',
            'message': f"{len(unprocessed)} transcripts from yesterday not processed",
            'action': 'Process with /inbox-processing'
        })

    # 3. Check master task list exists
    master_list = TODAY_DIR / "tasks/master-task-list.md"
    if not master_list.exists():
        warnings.append({
            'level': 'info',
            'message': 'Master task list not found',
            'action': 'Will create from template'
        })

    return warnings


def fetch_emails(max_results: int = 30) -> List[Dict[str, Any]]:
    """
    Fetch unread emails from Gmail.

    Returns:
        List of email dictionaries
    """
    try:
        result = subprocess.run(
            ["python3", str(GOOGLE_API_PATH), "gmail", "search", "is:unread in:inbox", str(max_results)],
            capture_output=True,
            text=True,
            timeout=30
        )

        if result.returncode != 0 or not result.stdout.strip():
            return []

        if result.stdout.strip() == "No messages found.":
            return []

        # Extract JSON from output (handles warnings printed before JSON)
        json_str = extract_json_from_output(result.stdout)
        if not json_str:
            return []

        return json.loads(json_str)

    except Exception as e:
        print(f"Warning: Email fetch failed: {e}", file=sys.stderr)
        return []


def classify_email_priority(email: Dict, account_domains: set) -> str:
    """
    Classify email priority based on sender and content.

    Args:
        email: Email dictionary
        account_domains: Set of known customer domains

    Returns:
        Priority level: 'high', 'medium', 'low'
    """
    from_addr = email.get('from', '').lower()
    subject = email.get('subject', '').lower()

    # Extract domain from sender
    if '@' in from_addr:
        # Handle "Name <email@domain.com>" format
        if '<' in from_addr:
            from_addr = from_addr.split('<')[1].split('>')[0]
        domain = from_addr.split('@')[1] if '@' in from_addr else ''
    else:
        domain = ''

    # HIGH: Customer emails, action words
    if domain in account_domains:
        return 'high'

    action_words = ['urgent', 'asap', 'action required', 'please respond', 'deadline']
    if any(word in subject for word in action_words):
        return 'high'

    # MEDIUM: Internal colleagues, meeting-related
    if domain in get_internal_domains():
        return 'medium'

    if any(word in subject for word in ['meeting', 'calendar', 'invite']):
        return 'medium'

    # LOW: Newsletters, automated, GitHub without @mention
    low_signals = ['newsletter', 'digest', 'notification', 'automated', 'noreply', 'no-reply']
    if any(signal in from_addr or signal in subject for signal in low_signals):
        return 'low'

    if 'github.com' in domain:
        return 'low'

    return 'medium'


def identify_agendas_needed(events: List[Dict], look_ahead_days: int = 5) -> List[Dict]:
    """
    Identify customer meetings in the look-ahead window that need agendas.

    Args:
        events: List of calendar events
        look_ahead_days: Number of days to look ahead

    Returns:
        List of meetings needing agendas
    """
    needs_agenda = []
    today = datetime.now().date()
    look_ahead = [today + timedelta(days=i) for i in range(1, look_ahead_days + 1)]
    # Filter to business days only
    look_ahead = [d for d in look_ahead if d.weekday() < 5][:4]

    for event in events:
        start_str = event.get('start', '')
        if not start_str:
            continue

        # Parse event date
        try:
            if 'T' in start_str:
                event_date = datetime.fromisoformat(start_str.replace('Z', '+00:00')).date()
            else:
                event_date = datetime.strptime(start_str, '%Y-%m-%d').date()
        except ValueError:
            continue

        # Skip if not in look-ahead window
        if event_date not in look_ahead:
            continue

        # Check if it's classified as customer meeting needing agenda
        classification = event.get('classification', {})
        if classification.get('type') == 'customer' and classification.get('agenda_owner') == 'you':
            # Check if agenda exists (would need to check calendar description)
            description = event.get('description', '')
            has_agenda = 'docs.google.com' in description or len(description) > 100

            if not has_agenda:
                needs_agenda.append({
                    'event_id': event.get('id'),
                    'account': classification.get('account'),
                    'title': event.get('summary'),
                    'date': event_date.isoformat(),
                    'start': start_str
                })

    return needs_agenda


def gather_meeting_context(classifications: List[Dict], account_lookup: Dict) -> List[Dict]:
    """
    Gather context needed for each customer meeting.

    Args:
        classifications: List of meeting classifications
        account_lookup: Account data lookup dictionary

    Returns:
        List of context dictionaries for each meeting
    """
    meeting_contexts = []

    for meeting in classifications:
        if meeting.get('type') != 'customer':
            continue

        account = meeting.get('account')
        if not account:
            continue

        context = {
            'event_id': meeting.get('event_id'),
            'account': account,
            'title': meeting.get('title'),
            'start': meeting.get('start'),
            'type': meeting.get('type'),
            'prep_status': meeting.get('prep_status'),
            'agenda_owner': meeting.get('agenda_owner'),
        }

        # Add account data
        account_data = account_lookup.get(account, {})
        context['account_data'] = {
            'ring': account_data.get('ring'),
            'arr': account_data.get('arr'),
            'renewal': account_data.get('renewal'),
            'last_engagement': account_data.get('last_engagement'),
        }

        # Check for dashboard
        dashboard = find_account_dashboard(account)
        if dashboard:
            context['dashboard_path'] = str(dashboard)
            context['dashboard_age_days'] = get_file_age_days(dashboard)
        else:
            context['dashboard_path'] = None

        # Check for recent meetings
        recent = find_recent_meeting_summaries(account, limit=2)
        context['recent_meetings'] = [str(p) for p in recent]

        # Check for action file
        actions = find_account_action_file(account)
        if actions:
            context['action_file'] = str(actions)
            context['action_file_age_days'] = get_file_age_days(actions)
        else:
            context['action_file'] = None

        meeting_contexts.append(context)

    return meeting_contexts


def main():
    """Main preparation orchestrator."""
    parser = argparse.ArgumentParser(description='Prepare today directive')
    parser.add_argument('--skip-archive', action='store_true', help='Skip archiving yesterday')
    parser.add_argument('--skip-email', action='store_true', help='Skip email fetch')
    parser.add_argument('--skip-dashboard', action='store_true', help='Skip dashboard auto-start')
    parser.add_argument('--output', type=str, default=str(DIRECTIVE_FILE), help='Output file path')
    args = parser.parse_args()

    print("=" * 60)
    print("PHASE 1: TODAY PREPARATION")
    print("=" * 60)

    # Auto-start dashboard (non-blocking, before other steps)
    if not args.skip_dashboard and is_dashboard_autostart_enabled():
        success, msg = start_dashboard_background()
        if success:
            print(f"  Dashboard: {msg}")

    # Initialize directive structure
    now = datetime.now()
    today = now.date()
    yesterday = now - timedelta(days=1)
    monday, friday, week_number = get_week_dates(now)

    # Check Google API availability early
    api_available, api_reason = check_google_api_available()

    directive = {
        'command': 'today',
        'generated_at': now.isoformat(),
        'context': {
            'date': today.isoformat(),
            'day_of_week': today.strftime('%A'),
            'week_number': week_number,
            'year': today.year,
        },
        'api_status': {
            'available': api_available,
            'reason': api_reason if not api_available else None,
        },
        'warnings': [],
        'calendar': {
            'events': [],
            'past': [],
            'in_progress': [],
            'upcoming': [],
            'gaps': [],
        },
        'meetings': {
            'customer': [],
            'internal': [],
            'project': [],
            'personal': [],
            'external': [],
        },
        'meeting_contexts': [],
        'actions': {
            'overdue': [],
            'due_today': [],
            'due_this_week': [],
            'related_to_meetings': [],
            'waiting_on': [],
        },
        'emails': {
            'high_priority': [],
            'medium_count': 0,
            'low_count': 0,
        },
        'agendas_needed': [],
        'files': {
            'existing_today': [],
            'inbox_pending': 0,
        },
        'ai_tasks': [],
    }

    # Step 0: Resilience checks
    print("\nStep 0: Running resilience checks...")
    warnings = run_resilience_checks(yesterday)
    directive['warnings'] = warnings
    for w in warnings:
        print(f"  {w['level'].upper()}: {w['message']}")

    # Step 1: Ensure structure and archive yesterday
    print("\nStep 1: Ensuring directory structure...")
    ensure_today_structure()

    if not args.skip_archive:
        # Check if we should archive (weekday and no existing archive)
        if yesterday.weekday() < 5 and not check_yesterday_archive(yesterday):
            existing = list_today_files()
            if existing['daily']:
                print(f"  Archiving {len(existing['daily'])} files from yesterday...")
                archive_daily_files(yesterday)

    # Step 2: Fetch account data
    print("\nStep 2: Fetching account data...")
    account_lookup = {}
    domain_mapping = load_domain_mapping()
    bu_cache = load_bu_cache()

    if api_available:
        sheet_data = fetch_account_data()
        if sheet_data:
            account_lookup = build_account_lookup(sheet_data)
            print(f"  Loaded {len(account_lookup)} accounts")

            # Build domain set for email classification
            account_domains = set()
            for data in account_lookup.values():
                if data.get('email_domain'):
                    account_domains.add(data['email_domain'].lower())
        else:
            print("  Warning: Could not load account data from Google Sheets")
            account_domains = set(domain_mapping.keys())
    else:
        print(f"  Skipped (Google API unavailable: {api_reason})")
        account_domains = set(domain_mapping.keys())

    # Step 3: Fetch calendar events
    print("\nStep 3: Fetching calendar events...")
    events = []
    if api_available:
        events = fetch_calendar_events(days=1)
        print(f"  Found {len(events)} events for today")
    else:
        print(f"  Skipped (Google API unavailable: {api_reason})")
        print("  NOTE: Add meetings manually or complete Google API setup")

    # Step 4: Classify meetings
    print("\nStep 4: Classifying meetings...")
    classifications = []
    for event in events:
        classification = classify_meeting(event, domain_mapping, bu_cache)
        classification['start_display'] = format_time_for_display(event.get('start', ''))
        classification['start_filename'] = format_time_for_filename(event.get('start', ''))
        classifications.append(classification)

        # Attach classification to event for later use
        event['classification'] = classification

    # Categorize by type
    for c in classifications:
        meeting_type = c.get('type', 'unknown')
        formatted = format_classification_for_directive(c)
        formatted['start_display'] = c.get('start_display')
        formatted['start_filename'] = c.get('start_filename')

        if meeting_type in directive['meetings']:
            directive['meetings'][meeting_type].append(formatted)
        else:
            directive['meetings']['external'].append(formatted)

    # Categorize by time status
    time_status = filter_events_by_status(events, now)
    directive['calendar']['past'] = [e.get('id') for e in time_status['past']]
    directive['calendar']['in_progress'] = [e.get('id') for e in time_status['in_progress']]
    directive['calendar']['upcoming'] = [e.get('id') for e in time_status['upcoming']]
    directive['calendar']['events'] = [
        {
            'id': e.get('id'),
            'summary': e.get('summary'),
            'start': e.get('start'),
            'end': e.get('end'),
        }
        for e in events
    ]

    # Calculate meeting gaps
    gaps = calculate_meeting_gaps(events)
    directive['calendar']['gaps'] = gaps

    print(f"  Customer: {len(directive['meetings']['customer'])}")
    print(f"  Internal: {len(directive['meetings']['internal'])}")
    print(f"  Project: {len(directive['meetings']['project'])}")

    # Step 5: Gather meeting context
    print("\nStep 5: Gathering meeting context...")
    meeting_contexts = gather_meeting_context(classifications, account_lookup)
    directive['meeting_contexts'] = meeting_contexts

    # Step 6: Aggregate action items
    print("\nStep 6: Aggregating action items...")
    task_data = load_master_task_list()
    all_tasks = task_data.get('tasks', [])

    # Filter to James's tasks only
    james_tasks = filter_tasks_by_owner(all_tasks, 'james')
    incomplete_tasks = [t for t in james_tasks if not t.get('completed')]

    # Get overdue
    overdue = get_overdue_tasks(incomplete_tasks, now)
    directive['actions']['overdue'] = [format_task_for_directive(t) for t in overdue]

    # Get due today
    due_today = get_tasks_due_on(incomplete_tasks, now)
    directive['actions']['due_today'] = [format_task_for_directive(t) for t in due_today]

    # Get related to today's meetings
    meeting_accounts = [m.get('account') for m in directive['meetings']['customer'] if m.get('account')]
    related = get_tasks_for_accounts(incomplete_tasks, meeting_accounts)
    directive['actions']['related_to_meetings'] = [format_task_for_directive(t) for t in related]

    # Get Waiting On (Delegated) items
    waiting_on = extract_waiting_on()
    directive['actions']['waiting_on'] = waiting_on

    print(f"  Overdue: {len(overdue)}")
    print(f"  Due today: {len(due_today)}")
    print(f"  Related to meetings: {len(related)}")
    print(f"  Waiting on: {len(waiting_on)}")

    # Step 7: Fetch emails
    if not args.skip_email and api_available:
        print("\nStep 7: Fetching emails...")
        emails = fetch_emails(max_results=30)
        print(f"  Found {len(emails)} unread emails")

        # Classify by priority
        high = []
        medium_count = 0
        low_count = 0

        for email in emails:
            priority = classify_email_priority(email, account_domains)
            if priority == 'high':
                high.append({
                    'id': email.get('id'),
                    'thread_id': email.get('threadId'),
                    'from': email.get('from'),
                    'subject': email.get('subject'),
                    'snippet': email.get('snippet'),
                    'date': email.get('date'),
                })
            elif priority == 'medium':
                medium_count += 1
            else:
                low_count += 1

        directive['emails']['high_priority'] = high
        directive['emails']['medium_count'] = medium_count
        directive['emails']['low_count'] = low_count

        print(f"  High priority: {len(high)}")
        print(f"  Medium: {medium_count}")
        print(f"  Low: {low_count}")
    elif not api_available:
        print("\nStep 7: Skipping email fetch (Google API unavailable)")
    else:
        print("\nStep 7: Skipping email fetch (--skip-email)")

    # Step 8: Look-ahead for agendas
    print("\nStep 8: Checking agenda needs for look-ahead...")
    agendas_needed = []
    if api_available:
        # Fetch 5 days of events for look-ahead
        look_ahead_events = fetch_calendar_events(days=5)
        # Classify them
        for event in look_ahead_events:
            event['classification'] = classify_meeting(event, domain_mapping, bu_cache)

        agendas_needed = identify_agendas_needed(look_ahead_events)
        print(f"  Agendas needed: {len(agendas_needed)}")
    else:
        print("  Skipped (Google API unavailable)")
    directive['agendas_needed'] = agendas_needed

    # Step 9: Check existing files
    print("\nStep 9: Checking existing files...")
    existing = list_today_files()
    directive['files']['existing_today'] = [f.name for f in existing['daily'] + existing['week']]
    directive['files']['inbox_pending'] = count_inbox_pending()
    print(f"  Existing today files: {len(directive['files']['existing_today'])}")
    print(f"  Inbox pending: {directive['files']['inbox_pending']}")

    # Step 10: Generate AI task list
    print("\nStep 10: Generating AI task list...")

    # Customer meeting preps
    for meeting in directive['meetings']['customer']:
        if meeting.get('event_id') not in directive['calendar']['past']:
            directive['ai_tasks'].append({
                'type': 'generate_customer_prep',
                'event_id': meeting.get('event_id'),
                'account': meeting.get('account'),
                'priority': 'high' if meeting.get('prep_status') == '📅 Agenda needed' else 'medium',
            })

    # Internal meeting preps
    for meeting in directive['meetings']['internal']:
        if meeting.get('event_id') not in directive['calendar']['past']:
            directive['ai_tasks'].append({
                'type': 'generate_internal_prep',
                'event_id': meeting.get('event_id'),
                'priority': 'low',
            })

    # Project meeting preps
    for meeting in directive['meetings']['project']:
        if meeting.get('event_id') not in directive['calendar']['past']:
            directive['ai_tasks'].append({
                'type': 'generate_project_prep',
                'event_id': meeting.get('event_id'),
                'project': meeting.get('project'),
                'priority': 'medium',
            })

    # High priority email summaries
    for email in directive['emails']['high_priority']:
        directive['ai_tasks'].append({
            'type': 'summarize_email',
            'email_id': email.get('id'),
            'thread_id': email.get('thread_id'),
            'priority': 'medium',
        })

    # Agenda drafts for look-ahead
    for agenda in directive['agendas_needed']:
        directive['ai_tasks'].append({
            'type': 'generate_agenda_draft',
            'event_id': agenda.get('event_id'),
            'account': agenda.get('account'),
            'date': agenda.get('date'),
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
        print(f"   Calendar, email, and Sheets data unavailable")
        print(f"   Task list and local files still processed")

    print(f"\nSummary:")
    print(f"  - Google API: {'✅ Available' if api_available else '❌ Unavailable'}")
    print(f"  - Meetings today: {len(events)}")
    print(f"  - Customer meetings: {len(directive['meetings']['customer'])}")
    print(f"  - Overdue actions: {len(directive['actions']['overdue'])}")
    print(f"  - High priority emails: {len(directive['emails']['high_priority'])}")
    print(f"  - AI tasks: {len(directive['ai_tasks'])}")

    if directive['warnings']:
        print(f"\n⚠️  Warnings: {len(directive['warnings'])}")
        for w in directive['warnings']:
            print(f"    - {w['message']}")

    print("\nNext: Claude executes AI tasks and generates prep files")
    print("Then: Run python3 _tools/deliver_today.py")

    return 0


if __name__ == "__main__":
    sys.exit(main())
