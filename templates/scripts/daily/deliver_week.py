#!/usr/bin/env python3
"""
Phase 3: Week Delivery Script
Handles post-AI delivery operations for /week command.

After Claude has executed AI tasks (Phase 2), this script:
1. Reads enriched directive with AI outputs
2. Writes week-00 through week-04 files
3. Creates weekly impact template
4. Optionally creates calendar events for time blocks
5. Moves archive to inbox for canonical processing
6. Generates completion output

Usage:
    python3 _tools/deliver_week.py [--directive FILE] [--skip-calendar] [--skip-inbox]
"""

import argparse
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent / 'lib'))

from file_utils import (
    ensure_today_structure, TODAY_DIR, ARCHIVE_DIR, INBOX_DIR,
    LEADERSHIP_DIR, VIP_ROOT
)
from calendar_utils import create_calendar_event

# Paths
DIRECTIVE_FILE = TODAY_DIR / ".week-directive.json"
GOOGLE_API_PATH = VIP_ROOT / ".config/google/google_api.py"


def load_directive(path: Path) -> Optional[Dict[str, Any]]:
    """
    Load the directive file.

    Args:
        path: Path to directive JSON file

    Returns:
        Directive dictionary or None if failed
    """
    if not path.exists():
        print(f"Error: Directive file not found: {path}", file=sys.stderr)
        return None

    try:
        with open(path) as f:
            return json.load(f)
    except Exception as e:
        print(f"Error: Failed to load directive: {e}", file=sys.stderr)
        return None


def write_week_overview(directive: Dict, ai_outputs: Dict) -> Path:
    """
    Write the week-00-overview.md file.

    Args:
        directive: The directive dictionary
        ai_outputs: AI-generated outputs

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)
    date_range = context.get('date_range_display', '')

    # Build meetings table
    meetings_by_day = directive.get('meetings', {}).get('by_day', {})
    meeting_rows = []

    for day_name in ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday']:
        day_meetings = meetings_by_day.get(day_name, [])

        for meeting in day_meetings:
            account = meeting.get('account', meeting.get('title', 'Unknown'))
            time = meeting.get('start_display', '-')
            ring = meeting.get('account_data', {}).get('ring', '-') if meeting.get('type') == 'customer' else '-'
            prep_status = meeting.get('prep_status', '-')
            meeting_type = meeting.get('type', 'Unknown').title()

            meeting_rows.append(
                f"| {day_name[:3]} | {time} | {account} | {ring} | {prep_status} | {meeting_type} |"
            )

    meetings_table = "| Day | Time | Account/Meeting | Ring | Prep Status | Meeting Type |\n"
    meetings_table += "|-----|------|-----------------|------|-------------|---------------|\n"
    meetings_table += "\n".join(meeting_rows) if meeting_rows else "| - | - | No meetings | - | - | - |"

    # Build action summary
    actions = directive.get('actions', {})
    overdue = actions.get('overdue', [])
    this_week = actions.get('this_week', [])

    overdue_items = []
    for task in overdue[:5]:
        overdue_items.append(f"- [ ] **{task.get('title', 'Unknown')}** - {task.get('account', '')} - Due: {task.get('due', '')} ({task.get('days_overdue', 0)} days overdue)")

    this_week_items = []
    for task in this_week[:5]:
        this_week_items.append(f"- [ ] **{task.get('title', 'Unknown')}** - {task.get('account', '')} - Due: {task.get('due', '')}")

    # Build hygiene alerts
    hygiene = directive.get('hygiene_alerts', [])

    critical_alerts = []
    high_alerts = []
    for account_alerts in hygiene:
        account = account_alerts.get('account', 'Unknown')
        for alert in account_alerts.get('alerts', []):
            entry = f"- **{account}** - {alert.get('message', '')}"
            if alert.get('level') == 'critical':
                critical_alerts.append(entry)
            elif alert.get('level') == 'high':
                high_alerts.append(entry)

    # Build time block suggestions
    suggestions = directive.get('time_blocks', {}).get('suggestions', [])
    block_rows = []
    for s in suggestions[:5]:
        block_rows.append(f"| {s.get('block_type', 'Focus')}: {s.get('task', '')[:30]} | {s.get('day', '')} | {s.get('duration', 30)}m |")

    blocks_table = "| Block | Day | Duration |\n|-------|-----|----------|\n"
    blocks_table += "\n".join(block_rows) if block_rows else "| No suggestions | - | - |"

    healthy_count = len([a for a in hygiene if not any(alert['level'] in ['critical', 'high'] for alert in a.get('alerts', []))])

    content = f"""# Week Overview: W{week_number:02d} - {date_range}

## This Week's Meetings

{meetings_table}

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

### Overdue ({len(overdue)})

{chr(10).join(overdue_items) if overdue_items else "✅ No overdue items"}

### Due This Week ({len(this_week)})

{chr(10).join(this_week_items) if this_week_items else "✅ No items due this week"}

## Account Hygiene Alerts

### 🔴 Critical

{chr(10).join(critical_alerts) if critical_alerts else "✅ No critical alerts"}

### 🟡 Needs Attention

{chr(10).join(high_alerts[:5]) if high_alerts else "✅ No high-priority alerts"}

### 🟢 Healthy

- {healthy_count} accounts with no alerts

## Suggested Calendar Blocks

{blocks_table}

*Confirm before creating calendar events*

## Weekly Impact Template

Pre-populated template created:
`Leadership/02-Performance/Weekly-Impact/{context.get('year', 2026)}-W{week_number:02d}-impact-capture.md`

**Reminder**: Capture impacts throughout the week, not Friday afternoon.

---
*Generated by /week at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "week-00-overview.md"
    output_path.write_text(content)

    return output_path


def write_customer_meetings(directive: Dict) -> Path:
    """
    Write the week-01-customer-meetings.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)

    customer_meetings = directive.get('meetings', {}).get('customer', [])

    sections = []
    for meeting in customer_meetings:
        account = meeting.get('account', 'Unknown')
        account_data = meeting.get('account_data', {})

        section = f"""## {account}

**Time**: {meeting.get('start_display', 'TBD')}
**Ring**: {account_data.get('ring', 'Unknown')}
**ARR**: {account_data.get('arr', 'Unknown')}
**Renewal**: {account_data.get('renewal', 'Unknown')}

**Meeting Type**: {meeting.get('title', 'Sync')}
**Prep Status**: {meeting.get('prep_status', 'Unknown')}

**Context**:
- Last engagement: {account_data.get('last_engagement', 'Unknown')}
- Agenda owner: {meeting.get('agenda_owner', 'Unknown')}

---
"""
        sections.append(section)

    content = f"""# Customer Meetings - W{week_number:02d}

{chr(10).join(sections) if sections else "No customer meetings this week."}

---
*Generated by /week at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "week-01-customer-meetings.md"
    output_path.write_text(content)

    return output_path


def write_actions_file(directive: Dict) -> Path:
    """
    Write the week-02-actions.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)
    actions = directive.get('actions', {})

    overdue = actions.get('overdue', [])
    this_week = actions.get('this_week', [])

    overdue_section = ""
    for task in overdue:
        overdue_section += f"""- [ ] **{task.get('title', 'Unknown')}** - {task.get('account', '')}
  - Due: {task.get('due', '')} ({task.get('days_overdue', 0)} days overdue)
  - Source: {task.get('source', 'Unknown')}

"""

    this_week_section = ""
    for task in this_week:
        this_week_section += f"""- [ ] **{task.get('title', 'Unknown')}** - {task.get('account', '')}
  - Due: {task.get('due', '')}
  - Priority: {task.get('priority', 'P2')}

"""

    content = f"""# Action Items - W{week_number:02d}

## Overdue ({len(overdue)})

{overdue_section if overdue_section else "✅ No overdue items"}

## Due This Week ({len(this_week)})

{this_week_section if this_week_section else "✅ No items due this week"}

## No Due Date (Review Needed)

*Check master task list for items without due dates:*
`_today/tasks/master-task-list.md`

---
*Generated by /week at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "week-02-actions.md"
    output_path.write_text(content)

    return output_path


def write_hygiene_alerts(directive: Dict) -> Path:
    """
    Write the week-03-hygiene-alerts.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)
    hygiene = directive.get('hygiene_alerts', [])

    critical_section = ""
    high_section = ""
    medium_section = ""

    for account_alerts in hygiene:
        account = account_alerts.get('account', 'Unknown')
        ring = account_alerts.get('ring', 'Unknown')
        arr = account_alerts.get('arr', 'Unknown')

        for alert in account_alerts.get('alerts', []):
            entry = f"""### {account}
- **Issue**: {alert.get('message', '')}
- **Ring**: {ring}
- **ARR**: {arr}
- **Action**: {get_suggested_action(alert.get('type', ''))}

"""
            level = alert.get('level', 'low')
            if level == 'critical':
                critical_section += entry
            elif level == 'high':
                high_section += entry
            else:
                medium_section += entry

    healthy_count = len([a for a in hygiene if not any(alert['level'] in ['critical', 'high'] for alert in a.get('alerts', []))])

    content = f"""# Account Hygiene Alerts - W{week_number:02d}

## Critical (Act This Week)

{critical_section if critical_section else "✅ No critical alerts"}

## High Priority (Needs Attention)

{high_section if high_section else "✅ No high-priority alerts"}

## Medium Priority (Review When Possible)

{medium_section if medium_section else "✅ No medium-priority alerts"}

## Healthy Accounts

{healthy_count} accounts with no alerts.

---
*Generated by /week at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "week-03-hygiene-alerts.md"
    output_path.write_text(content)

    return output_path


def get_suggested_action(alert_type: str) -> str:
    """Get suggested action for an alert type."""
    actions = {
        'renewal_critical': 'Schedule RM alignment meeting immediately',
        'renewal_warning': 'Begin EBR planning, reach out to customer',
        'renewal_inform': 'Schedule 6-month assessment',
        'stale_contact': 'Schedule touchpoint this week',
        'stale_dashboard': 'Refresh dashboard before next customer interaction',
        'missing_dashboard': 'Create account dashboard',
        'missing_success_plan': 'Create success plan',
        'stale_success_plan': 'Review and update success plan',
    }
    return actions.get(alert_type, 'Review and address as needed')


def write_focus_file(directive: Dict) -> Path:
    """
    Write the week-04-focus.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)
    actions = directive.get('actions', {})
    hygiene = directive.get('hygiene_alerts', [])
    customer_meetings = directive.get('meetings', {}).get('customer', [])

    overdue_count = len(actions.get('overdue', []))
    critical_count = sum(1 for a in hygiene if any(alert['level'] == 'critical' for alert in a.get('alerts', [])))
    stale_dashboards = sum(1 for a in hygiene if any(alert['type'] == 'stale_dashboard' for alert in a.get('alerts', [])))

    content = f"""# Weekly Focus Priorities - W{week_number:02d}

## Must Do This Week

1. [ ] Send agendas for all customer meetings ({len(customer_meetings)} meetings)
2. [ ] Address {overdue_count} overdue action items
{'3. [ ] Handle ' + str(critical_count) + ' critical hygiene alerts' if critical_count > 0 else ''}

## Should Do This Week

4. [ ] Refresh {stale_dashboards} stale dashboards
5. [ ] Update weekly impact template (don't wait for Friday)
6. [ ] Review action items without due dates

## Could Do This Week

7. [ ] Review success plan progress for Evolution+ accounts
8. [ ] Clean up master task list
9. [ ] Process inbox files ({directive.get('files', {}).get('inbox_pending', 0)} pending)

## Time Allocation Intent

| Category | Hours | Notes |
|----------|-------|-------|
| Customer Meetings | ~{len(customer_meetings)} | {len(customer_meetings)} meetings scheduled |
| Meeting Prep | ~{len(customer_meetings) * 0.5:.1f} | ~30 min per customer meeting |
| Administrative | ~3 | Actions, hygiene, impact capture |
| Focus Work | ~4 | Deep work on priorities |

---
*Generated by /week at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "week-04-focus.md"
    output_path.write_text(content)

    return output_path


def write_impact_template(directive: Dict) -> Path:
    """
    Write the weekly impact capture template.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)
    year = context.get('year', 2026)
    date_range = context.get('date_range_display', '')
    monday = context.get('monday', '')

    customer_by_day = directive.get('impact_template', {}).get('customer_meetings_by_day', {})

    # Build meetings table
    meeting_rows = []
    for day in ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday']:
        accounts = customer_by_day.get(day, [])
        if accounts:
            for account in accounts:
                meeting_rows.append(f"| {day[:3]} | {account} | | |")
        else:
            meeting_rows.append(f"| {day[:3]} | | | |")

    meetings_table = "| Day | Account | Meeting Type | Outcome |\n"
    meetings_table += "|-----|---------|--------------|----------|\n"
    meetings_table += "\n".join(meeting_rows)

    content = f"""---
area: Leadership
doc_type: impact
status: draft
date: {monday}
week: W{week_number:02d}
tags: [impact, weekly, {year}]
privacy: internal
---

# Weekly Impact Capture - W{week_number:02d} ({date_range})

## Customer Meetings This Week

{meetings_table}

## Customer Outcomes (Value Delivered)

### Customer Wins
-

### Technical Outcomes
-

## Personal Impact (What You Moved Forward)

### Stakeholder Engagement
-

### Strategic Contributions
-

## Expansion Progress

### Opportunities Identified
-

### Pipeline Movement
-

## Risk Management

### Issues Resolved
-

### Risks Mitigated
-

## Cross-Functional Contributions

-

## Key Learnings

-

---
*To be completed throughout the week and finalized by Friday*
"""

    # Ensure directory exists
    impact_dir = LEADERSHIP_DIR / "02-Performance/Weekly-Impact"
    impact_dir.mkdir(parents=True, exist_ok=True)

    output_path = impact_dir / f"{year}-W{week_number:02d}-impact-capture.md"
    output_path.write_text(content)

    return output_path


def create_time_block_events(suggestions: List[Dict], approved: List[str]) -> int:
    """
    Create calendar events for approved time blocks.

    Args:
        suggestions: List of time block suggestions
        approved: List of approved task names

    Returns:
        Number of events created
    """
    created = 0

    for suggestion in suggestions:
        if suggestion.get('task') not in approved:
            continue

        title = f"{suggestion.get('block_type', 'Focus')}: {suggestion.get('task', 'Task')}"
        start = suggestion.get('start', '')
        end = suggestion.get('end', '')
        description = f"Task from master-task-list. Source: /week planning."

        result = create_calendar_event(title, start, end, description)
        if result:
            created += 1

    return created


def cleanup_directive(path: Path) -> None:
    """
    Remove the directive file after successful delivery.

    Args:
        path: Path to directive file
    """
    if path.exists():
        path.unlink()


def main():
    """Main delivery orchestrator."""
    parser = argparse.ArgumentParser(description='Deliver week files')
    parser.add_argument('--directive', type=str, default=str(DIRECTIVE_FILE), help='Directive file path')
    parser.add_argument('--skip-calendar', action='store_true', help='Skip calendar event creation')
    parser.add_argument('--skip-inbox', action='store_true', help='Skip moving archives to inbox')
    parser.add_argument('--keep-directive', action='store_true', help='Keep directive file after delivery')
    parser.add_argument('--ai-outputs', type=str, help='JSON file with AI outputs')
    args = parser.parse_args()

    print("=" * 60)
    print("PHASE 3: WEEK DELIVERY")
    print("=" * 60)

    # Load directive
    directive_path = Path(args.directive)
    directive = load_directive(directive_path)

    if not directive:
        print("\nError: Could not load directive. Run prepare_week.py first.")
        return 1

    # Load AI outputs if provided
    ai_outputs = {}
    if args.ai_outputs:
        ai_path = Path(args.ai_outputs)
        if ai_path.exists():
            with open(ai_path) as f:
                ai_outputs = json.load(f)

    # Ensure structure
    ensure_today_structure()

    files_written = []

    # Write week overview
    print("\nWriting week-00-overview.md...")
    overview = write_week_overview(directive, ai_outputs)
    files_written.append(overview)
    print(f"  ✅ {overview.name}")

    # Write customer meetings
    print("\nWriting week-01-customer-meetings.md...")
    customers = write_customer_meetings(directive)
    files_written.append(customers)
    print(f"  ✅ {customers.name}")

    # Write actions file
    print("\nWriting week-02-actions.md...")
    actions = write_actions_file(directive)
    files_written.append(actions)
    print(f"  ✅ {actions.name}")

    # Write hygiene alerts
    print("\nWriting week-03-hygiene-alerts.md...")
    hygiene = write_hygiene_alerts(directive)
    files_written.append(hygiene)
    print(f"  ✅ {hygiene.name}")

    # Write focus file
    print("\nWriting week-04-focus.md...")
    focus = write_focus_file(directive)
    files_written.append(focus)
    print(f"  ✅ {focus.name}")

    # Write impact template
    print("\nWriting weekly impact template...")
    impact = write_impact_template(directive)
    files_written.append(impact)
    print(f"  ✅ {impact.relative_to(VIP_ROOT)}")

    # Handle calendar events (if not skipped)
    events_created = 0
    if not args.skip_calendar:
        approved = ai_outputs.get('approved_time_blocks', [])
        if approved:
            print("\nCreating calendar events...")
            suggestions = directive.get('time_blocks', {}).get('suggestions', [])
            events_created = create_time_block_events(suggestions, approved)
            print(f"  ✅ Created {events_created} calendar events")
        else:
            print("\n⏭️  Skipping calendar events (none approved)")

    # Cleanup
    if not args.keep_directive:
        print("\nCleaning up directive file...")
        cleanup_directive(directive_path)
        print("  ✅ Directive removed")

    # Summary
    context = directive.get('context', {})
    week_number = context.get('week_number', 0)

    print("\n" + "=" * 60)
    print("✅ PHASE 3 COMPLETE")
    print("=" * 60)
    print(f"\nWeek {week_number} files written: {len(files_written)}")
    for f in files_written:
        try:
            print(f"  - {f.relative_to(VIP_ROOT)}")
        except ValueError:
            print(f"  - {f}")

    if events_created > 0:
        print(f"\nCalendar events created: {events_created}")

    print(f"\nOutput directory: {TODAY_DIR}")
    print("\n/week workflow complete!")
    print("\nNext steps:")
    print("  1. Review week-00-overview.md")
    print("  2. Run /today to generate today's meeting prep")

    return 0


if __name__ == "__main__":
    sys.exit(main())
