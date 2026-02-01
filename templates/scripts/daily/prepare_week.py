#!/usr/bin/env python3
"""
Phase 1: Week Preparation Script
Handles all deterministic operations for /week command.

Outputs a JSON directive file that Claude uses for AI-required operations.

Operations performed:
1. Calculate week number and date range (Mon-Fri)
2. Archive previous week's files
3. Fetch week's calendar events
4. Classify all meetings and determine prep type
5. Fetch account data and build hygiene checks
6. Aggregate action items for the week
7. Check previous week's impact file
8. Identify calendar gaps for time blocking
9. Output structured directive for Claude

Usage:
    python3 _tools/prepare_week.py [--skip-archive] [--output FILE]
"""

import argparse
import json
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent / 'lib'))

from calendar_utils import (
    fetch_calendar_events, get_week_dates, format_time_for_display,
    format_time_for_filename, calculate_meeting_gaps, filter_events_by_date,
    check_google_api_available
)
from task_utils import (
    load_master_task_list, get_overdue_tasks, get_tasks_for_week,
    filter_tasks_by_owner, format_task_for_directive, scan_account_action_files
)
from file_utils import (
    ensure_today_structure, archive_week_files, list_today_files,
    find_account_dashboard, get_file_age_days, TODAY_DIR, ARCHIVE_DIR,
    LEADERSHIP_DIR, VIP_ROOT
)
from meeting_utils import (
    classify_meeting, load_domain_mapping, load_bu_cache,
    fetch_account_data, build_account_lookup, format_classification_for_directive
)

# Paths
DIRECTIVE_FILE = TODAY_DIR / ".week-directive.json"
IMPACT_DIR = LEADERSHIP_DIR / "02-Performance/Weekly-Impact"


def check_previous_week_impact(week_number: int, year: int) -> Dict[str, Any]:
    """
    Check if previous week's impact file exists and its status.

    Args:
        week_number: Current week number
        year: Current year

    Returns:
        Status dictionary
    """
    prev_week = week_number - 1
    prev_year = year

    # Handle year boundary
    if prev_week < 1:
        prev_week = 52
        prev_year = year - 1

    impact_file = IMPACT_DIR / f"{prev_year}-W{prev_week:02d}-impact-capture.md"

    result = {
        'exists': False,
        'path': str(impact_file),
        'status': 'missing',
        'carry_forward_items': [],
    }

    if impact_file.exists():
        result['exists'] = True

        # Read and check status
        content = impact_file.read_text()

        if 'status: draft' in content.lower():
            result['status'] = 'draft'
        elif 'status: complete' in content.lower():
            result['status'] = 'complete'
        else:
            result['status'] = 'exists'

        # Look for "Action Items for Next Week" section
        if 'next week' in content.lower():
            # Parse carry-forward items (simplified)
            lines = content.split('\n')
            in_next_week = False
            for line in lines:
                if 'next week' in line.lower() and '#' in line:
                    in_next_week = True
                    continue
                if in_next_week:
                    if line.startswith('#'):
                        break
                    if line.strip().startswith('- '):
                        result['carry_forward_items'].append(line.strip())

    return result


def check_account_hygiene(account_lookup: Dict, domain_mapping: Dict) -> List[Dict]:
    """
    Check account hygiene and generate alerts.

    Args:
        account_lookup: Account data lookup dictionary
        domain_mapping: Domain to account mapping

    Returns:
        List of hygiene alert dictionaries
    """
    alerts = []
    today = datetime.now().date()

    # Ring-based thresholds (days)
    contact_thresholds = {
        'Foundation': 90,
        'Evolution': 45,
        'Influence': 30,
        'Summit': 14,
    }

    for account_name, data in account_lookup.items():
        account_alerts = []
        ring = data.get('ring', 'Foundation')

        # Check last engagement
        last_engagement = data.get('last_engagement')
        if last_engagement:
            try:
                last_date = datetime.strptime(last_engagement, '%Y-%m-%d').date()
                days_since = (today - last_date).days
                threshold = contact_thresholds.get(ring, 90)

                if days_since > threshold:
                    account_alerts.append({
                        'type': 'stale_contact',
                        'level': 'high' if days_since > threshold * 1.5 else 'medium',
                        'message': f"No contact in {days_since} days (threshold: {threshold} for {ring})",
                        'days': days_since,
                    })
            except ValueError:
                pass

        # Check renewal date
        renewal = data.get('renewal')
        if renewal:
            try:
                renewal_date = datetime.strptime(renewal, '%Y-%m-%d').date()
                months_out = (renewal_date - today).days / 30

                if months_out <= 3:
                    account_alerts.append({
                        'type': 'renewal_critical',
                        'level': 'critical',
                        'message': f"Renewal in {int(months_out)} months - RM alignment needed",
                        'months_out': months_out,
                    })
                elif months_out <= 4:
                    account_alerts.append({
                        'type': 'renewal_warning',
                        'level': 'high',
                        'message': f"Renewal in {int(months_out)} months - EBR planning required",
                        'months_out': months_out,
                    })
                elif months_out <= 6:
                    account_alerts.append({
                        'type': 'renewal_inform',
                        'level': 'medium',
                        'message': f"Renewal in {int(months_out)} months - assessment due",
                        'months_out': months_out,
                    })
            except ValueError:
                pass

        # Check dashboard staleness
        dashboard = find_account_dashboard(account_name)
        if dashboard:
            age = get_file_age_days(dashboard)
            if age > 60:
                account_alerts.append({
                    'type': 'stale_dashboard',
                    'level': 'medium',
                    'message': f"Dashboard not updated in {age} days",
                    'days': age,
                })
        else:
            account_alerts.append({
                'type': 'missing_dashboard',
                'level': 'low',
                'message': "No account dashboard found",
            })

        # Check success plan
        success_plan = data.get('success_plan')
        success_plan_updated = data.get('success_plan_updated')

        if ring in ['Evolution', 'Influence', 'Summit']:
            if not success_plan or success_plan.lower() == 'no':
                account_alerts.append({
                    'type': 'missing_success_plan',
                    'level': 'high',
                    'message': f"No success plan for {ring} account",
                })
            elif success_plan_updated:
                try:
                    sp_date = datetime.strptime(success_plan_updated, '%Y-%m-%d').date()
                    sp_age = (today - sp_date).days
                    if sp_age > 90:
                        account_alerts.append({
                            'type': 'stale_success_plan',
                            'level': 'medium',
                            'message': f"Success plan not updated in {sp_age} days",
                            'days': sp_age,
                        })
                except ValueError:
                    pass

        if account_alerts:
            alerts.append({
                'account': account_name,
                'ring': ring,
                'arr': data.get('arr'),
                'alerts': account_alerts,
            })

    # Sort by severity
    level_order = {'critical': 0, 'high': 1, 'medium': 2, 'low': 3}
    alerts.sort(key=lambda a: min(level_order.get(alert['level'], 3) for alert in a['alerts']))

    return alerts


def organize_meetings_by_day(classifications: List[Dict], monday: datetime) -> Dict[str, List[Dict]]:
    """
    Organize meeting classifications by day of week.

    Args:
        classifications: List of meeting classifications
        monday: Monday of the week

    Returns:
        Dictionary mapping day names to meetings
    """
    days = {
        'Monday': [],
        'Tuesday': [],
        'Wednesday': [],
        'Thursday': [],
        'Friday': [],
    }

    day_offsets = {
        0: 'Monday',
        1: 'Tuesday',
        2: 'Wednesday',
        3: 'Thursday',
        4: 'Friday',
    }

    for classification in classifications:
        start_str = classification.get('start', '')
        if not start_str:
            continue

        try:
            if 'T' in start_str:
                dt = datetime.fromisoformat(start_str.replace('Z', '+00:00'))
            else:
                dt = datetime.strptime(start_str, '%Y-%m-%d')

            # Make timezone-naive for comparison
            if dt.tzinfo:
                dt = dt.replace(tzinfo=None)

            weekday = dt.weekday()
            if weekday in day_offsets:
                day_name = day_offsets[weekday]
                days[day_name].append(classification)
        except ValueError:
            continue

    return days


def identify_time_blocks_for_tasks(gaps: Dict[str, List], tasks: List[Dict]) -> List[Dict]:
    """
    Match tasks to available time blocks.

    Args:
        gaps: Dictionary of day -> list of gaps
        tasks: List of tasks needing scheduling

    Returns:
        List of suggested time block dictionaries
    """
    suggestions = []

    # Task sizing
    task_sizes = {
        'P1': 60,  # Large - 60 min
        'P2': 30,  # Medium - 30 min
        'P3': 15,  # Small - 15 min
    }

    # Time preferences
    morning_types = ['strategic', 'writing', 'prep', 'analysis']
    afternoon_types = ['email', 'admin', 'followup', 'dashboard']

    for task in tasks[:10]:  # Limit to 10 tasks
        priority = task.get('priority', 'P2')
        duration = task_sizes.get(priority, 30)
        title = task.get('title', '').lower()

        # Determine time preference
        is_morning = any(t in title for t in morning_types)

        # Find suitable gap
        for day_name, day_gaps in gaps.items():
            for gap in day_gaps:
                gap_duration = gap.get('duration_minutes', 0)

                if gap_duration >= duration:
                    # Check time preference
                    gap_start = gap.get('start', '')
                    if gap_start:
                        try:
                            dt = datetime.fromisoformat(gap_start)
                            hour = dt.hour

                            # Morning preference: 9-12, Afternoon: 13-17
                            if is_morning and hour >= 13:
                                continue  # Skip afternoon slots for morning tasks
                            if not is_morning and hour < 12:
                                continue  # Skip morning slots for afternoon tasks

                            suggestions.append({
                                'task': task.get('title'),
                                'day': day_name,
                                'start': gap_start,
                                'end': gap.get('end'),
                                'duration': duration,
                                'priority': priority,
                                'block_type': 'Focus' if is_morning else 'Admin',
                            })
                            break
                        except ValueError:
                            continue

            if len(suggestions) >= 10:
                break

    return suggestions


def main():
    """Main preparation orchestrator."""
    parser = argparse.ArgumentParser(description='Prepare week directive')
    parser.add_argument('--skip-archive', action='store_true', help='Skip archiving previous week')
    parser.add_argument('--output', type=str, default=str(DIRECTIVE_FILE), help='Output file path')
    args = parser.parse_args()

    print("=" * 60)
    print("PHASE 1: WEEK PREPARATION")
    print("=" * 60)

    # Check Google API availability early
    api_available, api_reason = check_google_api_available()

    # Initialize
    now = datetime.now()
    monday, friday, week_number = get_week_dates(now)
    year = now.year

    directive = {
        'command': 'week',
        'generated_at': now.isoformat(),
        'context': {
            'week_number': week_number,
            'year': year,
            'monday': monday.strftime('%Y-%m-%d'),
            'friday': friday.strftime('%Y-%m-%d'),
            'date_range_display': f"{monday.strftime('%B %d')}-{friday.strftime('%d, %Y')}",
        },
        'api_status': {
            'available': api_available,
            'reason': api_reason if not api_available else None,
        },
        'previous_week': {},
        'meetings': {
            'by_day': {},
            'customer': [],
            'internal': [],
            'project': [],
            'personal': [],
            'external': [],
        },
        'actions': {
            'overdue': [],
            'this_week': [],
        },
        'hygiene_alerts': [],
        'time_blocks': {
            'gaps_by_day': {},
            'suggestions': [],
        },
        'impact_template': {
            'path': '',
            'customer_meetings_by_day': {},
        },
        'ai_tasks': [],
    }

    print(f"\nWeek {week_number}: {monday.strftime('%B %d')} - {friday.strftime('%B %d, %Y')}")

    # Step 1: Archive previous week files
    if not args.skip_archive:
        print("\nStep 1: Archiving previous week files...")
        today_files = list_today_files()

        if today_files['week']:
            prev_week = week_number - 1 if week_number > 1 else 52
            archived = archive_week_files(prev_week)
            print(f"  Archived {len(archived)} week files to W{prev_week:02d}/")
        else:
            print("  No previous week files to archive")
    else:
        print("\nStep 1: Skipping archive")

    # Step 2: Check previous week's impact
    print("\nStep 2: Checking previous week's impact...")
    prev_impact = check_previous_week_impact(week_number, year)
    directive['previous_week'] = prev_impact

    if prev_impact['exists']:
        print(f"  Previous week: {prev_impact['status']}")
        if prev_impact['carry_forward_items']:
            print(f"  Carry-forward items: {len(prev_impact['carry_forward_items'])}")
    else:
        print("  ⚠️  Previous week impact not captured")

    # Step 3: Fetch account data
    print("\nStep 3: Fetching account data...")
    account_lookup = {}
    domain_mapping = load_domain_mapping()
    bu_cache = load_bu_cache()

    if api_available:
        sheet_data = fetch_account_data()
        if sheet_data:
            account_lookup = build_account_lookup(sheet_data)
            print(f"  Loaded {len(account_lookup)} accounts")
        else:
            print("  Warning: Could not load account data from Google Sheets")
    else:
        print(f"  Skipped (Google API unavailable: {api_reason})")

    # Step 4: Fetch week's calendar
    print("\nStep 4: Fetching week's calendar...")
    events = []
    week_events = []

    if api_available:
        events = fetch_calendar_events(days=7)

        # Filter to only Mon-Fri of this week
        for event in events:
            start_str = event.get('start', '')
            try:
                if 'T' in start_str:
                    dt = datetime.fromisoformat(start_str.replace('Z', '+00:00'))
                else:
                    dt = datetime.strptime(start_str, '%Y-%m-%d')

                if dt.tzinfo:
                    dt = dt.replace(tzinfo=None)

                if monday <= dt < friday + timedelta(days=1):
                    week_events.append(event)
            except ValueError:
                continue

        print(f"  Found {len(week_events)} events this week")
    else:
        print(f"  Skipped (Google API unavailable: {api_reason})")
        print("  NOTE: Add meetings manually or complete Google API setup")

    # Step 5: Classify meetings
    print("\nStep 5: Classifying meetings...")
    classifications = []

    for event in week_events:
        classification = classify_meeting(event, domain_mapping, bu_cache)
        classification['start_display'] = format_time_for_display(event.get('start', ''))
        classification['start_filename'] = format_time_for_filename(event.get('start', ''))

        # Add account data for customer meetings
        if classification.get('type') == 'customer':
            account = classification.get('account')
            if account and account in account_lookup:
                classification['account_data'] = account_lookup[account]

        classifications.append(classification)

    # Organize by day
    by_day = organize_meetings_by_day(classifications, monday)
    directive['meetings']['by_day'] = {
        day: [format_classification_for_directive(m) for m in meetings]
        for day, meetings in by_day.items()
    }

    # Organize by type
    for c in classifications:
        meeting_type = c.get('type', 'unknown')
        formatted = format_classification_for_directive(c)
        formatted['start_display'] = c.get('start_display')

        if meeting_type in directive['meetings']:
            directive['meetings'][meeting_type].append(formatted)

    customer_count = len(directive['meetings']['customer'])
    print(f"  Customer meetings: {customer_count}")
    print(f"  Internal meetings: {len(directive['meetings']['internal'])}")
    print(f"  Project meetings: {len(directive['meetings']['project'])}")

    # Step 6: Aggregate action items
    print("\nStep 6: Aggregating action items...")
    task_data = load_master_task_list()
    all_tasks = task_data.get('tasks', [])

    # Also scan account action files
    account_tasks = scan_account_action_files()
    all_tasks.extend(account_tasks)

    # Filter to James's tasks
    james_tasks = filter_tasks_by_owner(all_tasks, 'james')
    incomplete_tasks = [t for t in james_tasks if not t.get('completed')]

    # Get overdue
    overdue = get_overdue_tasks(incomplete_tasks, now)
    directive['actions']['overdue'] = [format_task_for_directive(t) for t in overdue]

    # Get this week
    this_week = get_tasks_for_week(incomplete_tasks, monday)
    directive['actions']['this_week'] = [format_task_for_directive(t) for t in this_week]

    print(f"  Overdue: {len(overdue)}")
    print(f"  Due this week: {len(this_week)}")

    # Step 7: Check account hygiene
    print("\nStep 7: Checking account hygiene...")
    hygiene_alerts = check_account_hygiene(account_lookup, domain_mapping)
    directive['hygiene_alerts'] = hygiene_alerts

    critical_count = sum(1 for a in hygiene_alerts if any(alert['level'] == 'critical' for alert in a['alerts']))
    high_count = sum(1 for a in hygiene_alerts if any(alert['level'] == 'high' for alert in a['alerts']))
    print(f"  Critical alerts: {critical_count}")
    print(f"  High alerts: {high_count}")
    print(f"  Total accounts with alerts: {len(hygiene_alerts)}")

    # Step 8: Identify time blocks
    print("\nStep 8: Analyzing time blocks...")

    gaps_by_day = {}
    for day_name, day_meetings in by_day.items():
        day_events = [{'start': m['start'], 'end': m['end']} for m in day_meetings]
        gaps = calculate_meeting_gaps(day_events)
        gaps_by_day[day_name] = gaps

    directive['time_blocks']['gaps_by_day'] = gaps_by_day

    # Suggest time blocks for tasks
    all_pending = overdue + this_week
    suggestions = identify_time_blocks_for_tasks(gaps_by_day, all_pending[:10])
    directive['time_blocks']['suggestions'] = suggestions
    print(f"  Time block suggestions: {len(suggestions)}")

    # Step 9: Prepare impact template data
    print("\nStep 9: Preparing impact template data...")
    impact_path = IMPACT_DIR / f"{year}-W{week_number:02d}-impact-capture.md"
    directive['impact_template']['path'] = str(impact_path)

    # Customer meetings by day for template
    customer_by_day = {}
    for day_name, day_meetings in by_day.items():
        customer_by_day[day_name] = [
            m.get('account', m.get('title'))
            for m in day_meetings
            if m.get('type') == 'customer'
        ]
    directive['impact_template']['customer_meetings_by_day'] = customer_by_day

    # Step 10: Generate AI task list
    print("\nStep 10: Generating AI task list...")

    # Priority setting prompt
    directive['ai_tasks'].append({
        'type': 'prompt_priorities',
        'priority': 'high',
    })

    # Generate customer meeting rows for overview
    for meeting in directive['meetings']['customer']:
        directive['ai_tasks'].append({
            'type': 'generate_meeting_row',
            'meeting': meeting,
            'priority': 'medium',
        })

    # Create agenda tasks for Foundation accounts
    for meeting in directive['meetings']['customer']:
        if meeting.get('prep_status') == '📅 Agenda needed':
            directive['ai_tasks'].append({
                'type': 'create_agenda_task',
                'account': meeting.get('account'),
                'date': meeting.get('start', '')[:10],
                'priority': 'medium',
            })

    # Hygiene alert summaries
    if critical_count > 0:
        directive['ai_tasks'].append({
            'type': 'summarize_critical_alerts',
            'count': critical_count,
            'priority': 'high',
        })

    # Time block approval prompt
    if suggestions:
        directive['ai_tasks'].append({
            'type': 'prompt_time_blocks',
            'suggestions': suggestions,
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
        print(f"   Calendar and Sheets data unavailable")
        print(f"   Task list and local files still processed")

    print(f"\nSummary:")
    print(f"  - Google API: {'✅ Available' if api_available else '❌ Unavailable'}")
    print(f"  - Week: W{week_number} ({monday.strftime('%b %d')} - {friday.strftime('%b %d')})")
    print(f"  - Total meetings: {len(week_events)}")
    print(f"  - Customer meetings: {customer_count}")
    print(f"  - Overdue actions: {len(overdue)}")
    print(f"  - Due this week: {len(this_week)}")
    print(f"  - Hygiene alerts: {len(hygiene_alerts)}")
    print(f"  - AI tasks: {len(directive['ai_tasks'])}")

    print("\nNext: Claude prompts for priorities and generates week files")
    print("Then: Run python3 _tools/deliver_week.py")

    return 0


if __name__ == "__main__":
    sys.exit(main())
