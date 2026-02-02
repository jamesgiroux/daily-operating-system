#!/usr/bin/env python3
"""
Phase 3: Today Delivery Script
Handles post-AI delivery operations for /today command.

After Claude has executed AI tasks (Phase 2), this script:
1. Reads enriched directive with AI outputs
2. Writes files to _today/ directory
3. Updates week overview with prep status
4. Optionally creates calendar events for time blocks
5. Generates summary output

Usage:
    python3 _tools/deliver_today.py [--directive FILE] [--skip-calendar]
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

from file_utils import ensure_today_structure, VIP_ROOT, TODAY_DIR
from calendar_utils import create_calendar_event

# Paths
DIRECTIVE_FILE = TODAY_DIR / ".today-directive.json"
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


def write_overview_file(directive: Dict, ai_outputs: Dict) -> Path:
    """
    Write the 00-overview.md file.

    Args:
        directive: The directive dictionary
        ai_outputs: AI-generated outputs to include

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    day_of_week = context.get('day_of_week', 'Today')

    # Parse date for display
    try:
        date_obj = datetime.strptime(date, '%Y-%m-%d')
        date_display = date_obj.strftime('%A, %B %d, %Y')
    except ValueError:
        date_display = date

    # Build schedule table - filter out personal events and solo events
    schedule_rows = []
    events = directive.get('calendar', {}).get('events', [])

    for event in events:
        # Get classification info
        event_id = event.get('id')
        meeting_type = 'Unknown'
        prep_status = '-'

        for mtype, meetings in directive.get('meetings', {}).items():
            for m in meetings:
                if m.get('event_id') == event_id:
                    meeting_type = mtype
                    prep_status = m.get('prep_status', '-')
                    break

        # Skip personal events (Home, Daily Prep, Post-Meeting Catch-Up, etc.)
        if meeting_type == 'personal':
            continue

        # Format time
        start = event.get('start', '')
        if 'T' in start:
            try:
                dt = datetime.fromisoformat(start.replace('Z', '+00:00'))
                time_display = dt.strftime('%-I:%M %p')
            except ValueError:
                time_display = start[:5] if len(start) >= 5 else start
        else:
            time_display = 'All day'

        # Escape pipe characters in title
        title = event.get('summary', 'No title').replace('|', '/')

        schedule_rows.append(f"| {time_display} | {title} | {meeting_type.title()} | {prep_status} |")

    schedule_table = "| Time | Event | Type | Prep Status |\n|------|-------|------|-------------|\n"
    schedule_table += "\n".join(schedule_rows) if schedule_rows else "| - | No meetings today | - | - |"

    # Build customer meetings section
    customer_sections = []
    for meeting in directive.get('meetings', {}).get('customer', []):
        account = meeting.get('account', 'Unknown')
        start = meeting.get('start_display', '')

        # Find context for this meeting
        meeting_context = None
        for ctx in directive.get('meeting_contexts', []):
            if ctx.get('account') == account:
                meeting_context = ctx
                break

        account_data = meeting_context.get('account_data', {}) if meeting_context else {}

        section = f"""### {account} ({start})
- **Ring**: {account_data.get('ring', 'Unknown')}
- **ARR**: {account_data.get('arr', 'Unknown')}
- **Renewal**: {account_data.get('renewal', 'Unknown')}
- **Prep**: See prep file below"""

        customer_sections.append(section)

    customer_section = "\n\n".join(customer_sections) if customer_sections else "No customer meetings today."

    # Build email section
    emails = directive.get('emails', {})
    high_priority = emails.get('high_priority', [])
    medium_count = emails.get('medium_count', 0)
    low_count = emails.get('low_count', 0)

    email_rows = []
    for email in high_priority[:5]:  # Limit to 5
        email_rows.append(f"| {email.get('from', '')[:30]} | {email.get('subject', '')[:40]} | Review needed |")

    email_table = "| From | Subject | Notes |\n|------|---------|-------|\n"
    email_table += "\n".join(email_rows) if email_rows else "| - | No high priority emails | - |"

    # Build action items section
    actions = directive.get('actions', {})
    overdue = actions.get('overdue', [])
    due_today = actions.get('due_today', [])
    waiting_on = actions.get('waiting_on', [])

    overdue_items = []
    for task in overdue[:5]:
        overdue_items.append(f"- [ ] {task.get('title', 'Unknown')} - {task.get('account', '')} - Due: {task.get('due', '')} ({task.get('days_overdue', 0)} days overdue)")

    due_today_items = []
    for task in due_today[:5]:
        due_today_items.append(f"- [ ] {task.get('title', 'Unknown')} - {task.get('account', '')}")

    # Build Waiting On table for overview
    waiting_on_table = ""
    if waiting_on:
        waiting_on_table = "| Who | What | Days |\n|-----|------|------|\n"
        for item in waiting_on[:5]:
            waiting_on_table += f"| {item.get('who', '')} | {item.get('what', '')} | {item.get('days', '')} |\n"

    # Build agenda status section
    agendas = directive.get('agendas_needed', [])
    agenda_rows = []
    for agenda in agendas[:5]:
        agenda_rows.append(f"| {agenda.get('account', '')} | {agenda.get('date', '')} | ⚠️ Needs agenda | Draft in 90-agenda-needed/ |")

    agenda_table = "| Meeting | Date | Status | Action |\n|---------|------|--------|--------|\n"
    agenda_table += "\n".join(agenda_rows) if agenda_rows else "| - | - | ✅ All set | - |"

    # Build warnings section
    warnings = directive.get('warnings', [])
    warnings_section = ""
    if warnings:
        warnings_section = "## ⚠️ Attention Needed\n\n"
        for w in warnings:
            warnings_section += f"- {w.get('level', 'warning').upper()}: {w.get('message', '')}\n"
            if w.get('action'):
                warnings_section += f"  - **Suggested:** {w['action']}\n"
        warnings_section += "\n"

    # Compose overview
    content = f"""# Today: {date_display}

{warnings_section}## Schedule

{schedule_table}

## Customer Meetings Today

{customer_section}

## Email - Needs Attention

### HIGH Priority ({len(high_priority)})

{email_table}

### Summary
- **High Priority**: {len(high_priority)} (review in 83-email-summary.md)
- **Medium**: {medium_count} (labeled for later)
- **Low**: {low_count} (consider archiving)

## Action Items - Quick View

### Overdue ({len(overdue)})

{chr(10).join(overdue_items) if overdue_items else "✅ No overdue items"}

### Due Today ({len(due_today)})

{chr(10).join(due_today_items) if due_today_items else "✅ No items due today"}

## Waiting On ({len(waiting_on)})

{waiting_on_table if waiting_on_table else "✅ Nothing pending"}

## Agenda Status (Next 3-4 Business Days)

{agenda_table}

## Today's Files

See the numbered files below for meeting prep:
- `00-overview.md` - This file
- `01-HHMM-type-name.md` - Meeting prep files
- `80-actions-due.md` - Full action item details
- `83-email-summary.md` - Email summaries

---
*Generated by /today at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "00-overview.md"
    output_path.write_text(content)

    return output_path


def write_actions_file(directive: Dict) -> Path:
    """
    Write the 80-actions-due.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    actions = directive.get('actions', {})

    # Build sections
    overdue_section = ""
    for task in actions.get('overdue', []):
        overdue_section += f"""- [ ] **{task.get('title', 'Unknown')}** - {task.get('account', '')} - Due: {task.get('due', '')} ({task.get('days_overdue', 0)} days overdue)
  - **Context**: {task.get('context', 'No context available')}
  - **Source**: {task.get('source', 'Unknown')}

"""

    due_today_section = ""
    for task in actions.get('due_today', []):
        due_today_section += f"""- [ ] **{task.get('title', 'Unknown')}** - {task.get('account', '')}
  - **Context**: {task.get('context', 'No context available')}
  - **Source**: {task.get('source', 'Unknown')}

"""

    related_section = ""
    for task in actions.get('related_to_meetings', []):
        related_section += f"""- [ ] **{task.get('title', 'Unknown')}** - Due: {task.get('due', 'No date')}
  - **Context**: {task.get('context', 'No context available')}
  - **Status update to share**: [Complete before meeting]

"""

    # Build Waiting On section for actions file
    waiting_on = actions.get('waiting_on', [])
    if waiting_on:
        waiting_table = "| Who | What | Asked | Days | Context |\n|-----|------|-------|------|---------|"
        for item in waiting_on:
            waiting_table += f"\n| {item.get('who', '')} | {item.get('what', '')} | {item.get('asked', '')} | {item.get('days', '')} | {item.get('context', '')} |"
        waiting_section = f"""## Waiting On (Delegated)

*Outbound asks where others owe you a response*

{waiting_table}

**Tip:** If stale (>7 days), consider follow-up or escalation.
"""
    else:
        waiting_section = """## Waiting On (Delegated)

*Outbound asks where others owe you a response*

✅ No pending delegated items
"""

    content = f"""# Action Items - {date}

## Overdue

{overdue_section if overdue_section else "✅ No overdue items"}

## Due Today

{due_today_section if due_today_section else "✅ No items due today"}

## Related to Today's Meetings

{related_section if related_section else "No related action items for today's meetings."}

{waiting_section}
## Due This Week

*See master task list for full weekly view: `_today/tasks/master-task-list.md`*

---
*Generated by /today at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "80-actions-due.md"
    output_path.write_text(content)

    return output_path


def write_email_summary_file(directive: Dict) -> Path:
    """
    Write the 83-email-summary.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    emails = directive.get('emails', {})
    high_priority = emails.get('high_priority', [])

    email_details = ""
    for i, email in enumerate(high_priority, 1):
        email_details += f"""### {i}. {email.get('subject', 'No subject')}

**From**: {email.get('from', 'Unknown')}
**Date**: {email.get('date', 'Unknown')}

**Snippet**:
> {email.get('snippet', 'No preview available')}

**Classification**: [AI to classify: 🟢 OPPORTUNITY / 🟡 INFO / 🔴 RISK / 🔵 ACTION NEEDED]

**For James**:
- [ ] Specific ask: [AI to extract]
- [ ] Recommended action: [AI to suggest]
- [ ] Owner: [James / Other]

---

"""

    content = f"""# Email Summary - {date}

## HIGH Priority Emails ({len(high_priority)})

{email_details if email_details else "✅ No high priority emails today"}

## Summary Statistics

| Category | Count |
|----------|-------|
| High Priority | {len(high_priority)} |
| Medium (Internal/P2) | {emails.get('medium_count', 0)} |
| Low (Newsletters/Auto) | {emails.get('low_count', 0)} |

## Recommended Actions

*Based on email analysis:*

1. [AI to generate prioritized action list]

---
*Generated by /today at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
*Run /email-scan for deeper analysis*
"""

    output_path = TODAY_DIR / "83-email-summary.md"
    output_path.write_text(content)

    return output_path


def write_suggested_focus_file(directive: Dict) -> Path:
    """
    Write the 81-suggested-focus.md file.

    Args:
        directive: The directive dictionary

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    actions = directive.get('actions', {})
    agendas = directive.get('agendas_needed', [])
    gaps = directive.get('calendar', {}).get('gaps', [])

    # Build pre-meeting prep items - include BOTH today's customer meetings AND upcoming agendas
    prep_items = []

    # Today's customer meetings
    customer_meetings = directive.get('meetings', {}).get('customer', [])
    for m in customer_meetings:
        if m.get('event_id') not in directive.get('calendar', {}).get('past', []):
            prep_items.append(f"- [ ] Review {m.get('account', 'Unknown')} prep before {m.get('start_display', '')} call")

    # Upcoming meetings that need agendas (look-ahead)
    for agenda in agendas:
        prep_items.append(f"- [ ] Prep for {agenda.get('account', 'Unknown')} on {agenda.get('date', '')} (agenda needed)")

    # Also check week overview for upcoming customer meetings
    week_overview_path = TODAY_DIR / "week-00-overview.md"
    if week_overview_path.exists() and not prep_items:
        # Parse week overview to find meetings this week
        try:
            week_content = week_overview_path.read_text()
            # Simple check: look for customer meetings in the table
            if '📋 Prep needed' in week_content or '📅 Agenda needed' in week_content:
                prep_items.append(f"- [ ] Review `week-00-overview.md` for this week's customer meetings needing prep")
        except:
            pass

    # Build overdue items
    overdue_items = []
    for task in actions.get('overdue', [])[:3]:
        overdue_items.append(f"- [ ] Address: {task.get('title', '')} ({task.get('days_overdue', 0)} days overdue)")

    # Build agenda items
    agenda_items = []
    for agenda in agendas[:3]:
        agenda_items.append(f"- [ ] Review and send agenda for {agenda.get('account', '')} ({agenda.get('date', '')} meeting)")

    # Build available time blocks
    time_blocks = []
    for gap in gaps[:3]:
        duration = gap.get('duration_minutes', 0)
        if duration >= 30:
            # Handle both ISO datetime (2026-02-02T09:00:00) and time-only (09:00) formats
            start_str = gap.get('start', '') or gap.get('start_time', '')
            end_str = gap.get('end', '') or gap.get('end_time', '')
            # Extract time portion if it's a full datetime
            if 'T' in start_str:
                start_time = start_str.split('T')[1][:5]  # Get HH:MM
            elif start_str and len(start_str) >= 5:
                start_time = start_str[:5] if ':' in start_str[:5] else start_str
            else:
                start_time = start_str
            if 'T' in end_str:
                end_time = end_str.split('T')[1][:5]  # Get HH:MM
            elif end_str and len(end_str) >= 5:
                end_time = end_str[:5] if ':' in end_str[:5] else end_str
            else:
                end_time = end_str
            if start_time and end_time:
                time_blocks.append(f"- {start_time} - {end_time} ({duration} min available)")

    content = f"""# Suggested Focus Areas - {date}

## Priority 1: Pre-Meeting Prep

{chr(10).join(prep_items) if prep_items else "✅ No upcoming meetings need prep review"}

## Priority 2: Overdue Items

{chr(10).join(overdue_items) if overdue_items else "✅ No overdue items"}

## Priority 3: Agenda Sending

{chr(10).join(agenda_items) if agenda_items else "✅ All upcoming agendas handled"}

## Priority 4: Available Time Blocks

{chr(10).join(time_blocks) if time_blocks else "No significant gaps today"}

## Energy-Aware Notes

- **Morning (high energy)**: Strategic prep, customer calls, complex writing
- **Afternoon (lower energy)**: Admin capture, follow-ups, email triage

## Quick Wins for Downtime

- [ ] Update a stale dashboard (check 83-hygiene-alerts.md)
- [ ] Process files in _inbox/ ({directive.get('files', {}).get('inbox_pending', 0)} pending)
- [ ] Review and archive low priority emails

---
*Generated by /today at {datetime.now().strftime('%Y-%m-%d %H:%M')}*
"""

    output_path = TODAY_DIR / "81-suggested-focus.md"
    output_path.write_text(content)

    return output_path


def update_week_overview(directive: Dict) -> bool:
    """
    Update week overview with prep status for today's meetings.

    Args:
        directive: The directive dictionary

    Returns:
        True if updated, False if no week overview exists
    """
    week_overview = TODAY_DIR / "week-00-overview.md"

    if not week_overview.exists():
        return False

    content = week_overview.read_text()

    # Update prep status for today's customer meetings
    for meeting in directive.get('meetings', {}).get('customer', []):
        account = meeting.get('account', '')
        if not account:
            continue

        # Find and update the row
        # This is a simplified update - production code should be more robust
        old_status = '📋 Prep needed'
        new_status = '✅ Prep ready'

        if meeting.get('prep_status') == '📅 Agenda needed':
            old_status = '📅 Agenda needed'
            new_status = '✏️ Draft ready'

        # Simple string replacement for the account row
        # This assumes the account name appears in the table
        if account in content and old_status in content:
            # Find the line with this account and update
            lines = content.split('\n')
            for i, line in enumerate(lines):
                if account in line and old_status in line:
                    lines[i] = line.replace(old_status, new_status)
                    break
            content = '\n'.join(lines)

    week_overview.write_text(content)
    return True


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
    parser = argparse.ArgumentParser(description='Deliver today files')
    parser.add_argument('--directive', type=str, default=str(DIRECTIVE_FILE), help='Directive file path')
    parser.add_argument('--skip-calendar', action='store_true', help='Skip calendar event creation')
    parser.add_argument('--keep-directive', action='store_true', help='Keep directive file after delivery')
    args = parser.parse_args()

    print("=" * 60)
    print("PHASE 3: TODAY DELIVERY")
    print("=" * 60)

    # Load directive
    directive_path = Path(args.directive)
    directive = load_directive(directive_path)

    if not directive:
        print("\nError: Could not load directive. Run prepare_today.py first.")
        return 1

    # Ensure structure
    ensure_today_structure()

    files_written = []

    # Write overview
    print("\nWriting 00-overview.md...")
    overview = write_overview_file(directive, {})
    files_written.append(overview)
    print(f"  ✅ {overview.name}")

    # Write actions file
    print("\nWriting 80-actions-due.md...")
    actions = write_actions_file(directive)
    files_written.append(actions)
    print(f"  ✅ {actions.name}")

    # Write email summary
    print("\nWriting 83-email-summary.md...")
    emails = write_email_summary_file(directive)
    files_written.append(emails)
    print(f"  ✅ {emails.name}")

    # Write suggested focus
    print("\nWriting 81-suggested-focus.md...")
    focus = write_suggested_focus_file(directive)
    files_written.append(focus)
    print(f"  ✅ {focus.name}")

    # Update week overview
    print("\nUpdating week overview...")
    if update_week_overview(directive):
        print("  ✅ Week overview updated with prep status")
    else:
        print("  ⚠️  No week overview found (run /week first)")

    # Note about meeting prep files
    print("\n📋 Meeting Prep Files:")
    print("  Note: Meeting prep files are generated by Claude during Phase 2.")
    print("  If prep files are missing, Claude should generate them from meeting contexts.")

    # Cleanup
    if not args.keep_directive:
        print("\nCleaning up directive file...")
        cleanup_directive(directive_path)
        print("  ✅ Directive removed")

    # Summary
    print("\n" + "=" * 60)
    print("✅ PHASE 3 COMPLETE")
    print("=" * 60)
    print(f"\nFiles written: {len(files_written)}")
    for f in files_written:
        print(f"  - {f.name}")

    print(f"\nOutput directory: {TODAY_DIR}")
    print("\n/today workflow complete!")

    return 0


if __name__ == "__main__":
    sys.exit(main())
