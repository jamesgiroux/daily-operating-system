#!/usr/bin/env python3
"""
Phase 3: Today Delivery Script
Handles post-AI delivery operations for /today command.

After Claude has executed AI tasks (Phase 2), this script:
1. Reads enriched directive with AI outputs
2. Writes markdown files to _today/ directory
3. Writes JSON data files to _today/data/ (for Tauri frontend)
4. Updates week overview with prep status
5. Optionally creates calendar events for time blocks
6. Generates summary output

JSON files generated (in _today/data/):
    - schedule.json   -- meetings with classifications
    - actions.json    -- all action items (flat list with priorities)
    - emails.json     -- email summaries with stats
    - preps/*.json    -- one per meeting with full context
    - manifest.json   -- generation metadata, file index, statistics

Usage:
    python3 _tools/deliver_today.py [--directive FILE] [--skip-calendar]
    python3 _tools/deliver_today.py --json-only     # JSON only, no markdown
    python3 _tools/deliver_today.py --no-json        # Markdown only, no JSON
"""

import argparse
import hashlib
import json
import re
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
DATA_DIR = TODAY_DIR / "data"
PREPS_DIR = DATA_DIR / "preps"

# Valid meeting types (used for classification normalization)
VALID_MEETING_TYPES = {
    "customer", "qbr", "training", "internal", "team_sync",
    "one_on_one", "partnership", "all_hands", "external", "personal",
}


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

**For You**:
- [ ] Specific ask: [AI to extract]
- [ ] Recommended action: [AI to suggest]
- [ ] Owner: [You / Other]

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

    # Check if file already exists with AI-enriched content (from Phase 2)
    # If it was modified after the directive was created, preserve it
    if output_path.exists():
        directive_path = TODAY_DIR / ".today-directive.json"
        if directive_path.exists():
            directive_mtime = directive_path.stat().st_mtime
            file_mtime = output_path.stat().st_mtime
            if file_mtime > directive_mtime:
                # File was modified after directive was created (AI enriched it)
                # Don't overwrite
                return output_path

        # Also check for markers indicating AI enrichment
        existing_content = output_path.read_text()
        if "[AI to classify" not in existing_content and "[AI to extract" not in existing_content:
            # File has actual classifications, not placeholders - preserve it
            return output_path

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


def _make_meeting_id(event: Dict, meeting_type: str) -> str:
    """
    Generate a stable meeting ID from an event.

    Format: HHMM-type-slug (e.g., "0900-customer-acme-sync").
    Falls back to a hash-based ID if time parsing fails.

    Args:
        event: Calendar event dictionary
        meeting_type: Classified meeting type string

    Returns:
        Stable meeting ID string
    """
    title = event.get('summary', 'untitled')
    slug = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')[:40]

    start = event.get('start', '')
    time_prefix = ''
    if 'T' in start:
        try:
            dt = datetime.fromisoformat(start.replace('Z', '+00:00'))
            time_prefix = dt.strftime('%H%M')
        except ValueError:
            pass

    if not time_prefix:
        # Fall back to hash for all-day or unparseable events
        raw = f"{start}-{title}"
        time_prefix = hashlib.md5(raw.encode()).hexdigest()[:6]

    return f"{time_prefix}-{meeting_type}-{slug}"


def _format_time_display(iso_string: str) -> str:
    """
    Convert an ISO datetime string to a human-readable time.

    Args:
        iso_string: ISO 8601 datetime string

    Returns:
        Formatted time string (e.g., "9:00 AM") or "All day"
    """
    if 'T' not in iso_string:
        return 'All day'
    try:
        dt = datetime.fromisoformat(iso_string.replace('Z', '+00:00'))
        return dt.strftime('%-I:%M %p')
    except ValueError:
        return iso_string[:5] if len(iso_string) >= 5 else iso_string


def _normalize_meeting_type(raw_type: str) -> str:
    """
    Normalize a meeting type string to a valid enum value.

    Args:
        raw_type: Raw meeting type from the directive

    Returns:
        Normalized meeting type string
    """
    normalized = raw_type.lower().replace(' ', '_').replace('-', '_')
    if normalized in VALID_MEETING_TYPES:
        return normalized
    return "internal"


def _classify_event(event: Dict, meetings: Dict) -> str:
    """
    Look up the meeting type for a calendar event by matching its event_id
    against the classified meetings dict from the directive.

    Args:
        event: Calendar event with an 'id' field
        meetings: The directive's 'meetings' dict keyed by type

    Returns:
        Meeting type string
    """
    event_id = event.get('id')
    for mtype, meeting_list in meetings.items():
        for m in meeting_list:
            if m.get('event_id') == event_id:
                return _normalize_meeting_type(mtype)
    return "internal"


def _find_meeting_context(
    account: str,
    meeting_contexts: List[Dict],
) -> Optional[Dict]:
    """
    Find the meeting context block for a given account.

    Args:
        account: Account name to match
        meeting_contexts: List of context dicts from directive

    Returns:
        Matching context dict or None
    """
    for ctx in meeting_contexts:
        if ctx.get('account') == account:
            return ctx
    return None


def _build_prep_summary(meeting: Dict, meeting_context: Optional[Dict]) -> Optional[Dict]:
    """
    Build a condensed prep summary for embedding in schedule.json.

    Args:
        meeting: Meeting dict from directive's meetings section
        meeting_context: Optional context block for the meeting

    Returns:
        Prep summary dict with atAGlance, discuss, watch, wins keys,
        or None if no meaningful data
    """
    if not meeting_context:
        return None

    account_data = meeting_context.get('account_data', {})
    at_a_glance = []

    if account_data.get('ring'):
        at_a_glance.append(f"Ring: {account_data['ring']}")
    if account_data.get('arr'):
        at_a_glance.append(f"ARR: {account_data['arr']}")
    if account_data.get('renewal'):
        at_a_glance.append(f"Renewal: {account_data['renewal']}")
    if account_data.get('health'):
        at_a_glance.append(f"Health: {account_data['health']}")

    # If there's nothing to show, skip the prep summary
    if not at_a_glance:
        return None

    return {
        "atAGlance": at_a_glance[:4],
        "discuss": [],
        "watch": [],
        "wins": [],
    }


def _build_schedule_json(directive: Dict) -> Dict[str, Any]:
    """
    Build the schedule.json payload from directive data.

    Conforms to templates/schemas/schedule.schema.json.
    Uses camelCase keys to match the Rust json_loader.rs consumer.

    Args:
        directive: The full directive dictionary

    Returns:
        Dictionary ready to serialize as schedule.json
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    events = directive.get('calendar', {}).get('events', [])
    meetings_by_type = directive.get('meetings', {})
    meeting_contexts = directive.get('meeting_contexts', [])

    meetings_json: List[Dict[str, Any]] = []

    for event in events:
        meeting_type = _classify_event(event, meetings_by_type)

        # Skip personal events — the markdown overview does the same
        if meeting_type == 'personal':
            continue

        meeting_id = _make_meeting_id(event, meeting_type)
        start = event.get('start', '')
        end = event.get('end', '')

        # Look up the meeting entry to get account info
        account: Optional[str] = None
        meeting_entry: Optional[Dict] = None
        for mtype, mlist in meetings_by_type.items():
            for m in mlist:
                if m.get('event_id') == event.get('id'):
                    account = m.get('account')
                    meeting_entry = m
                    break

        # Build prep summary for customer meetings
        prep_summary = None
        has_prep = False
        prep_file = None

        if meeting_entry and account:
            mc = _find_meeting_context(account, meeting_contexts)
            prep_summary = _build_prep_summary(meeting_entry, mc)
            if mc:
                has_prep = True
                prep_file = f"preps/{meeting_id}.json"

        meeting_obj: Dict[str, Any] = {
            "id": meeting_id,
            "time": _format_time_display(start),
            "title": event.get('summary', 'No title'),
            "type": meeting_type,
            "hasPrep": has_prep,
            "isCurrent": False,
        }

        if end:
            meeting_obj["endTime"] = _format_time_display(end)
        if account:
            meeting_obj["account"] = account
        if prep_file:
            meeting_obj["prepFile"] = prep_file
        if prep_summary:
            meeting_obj["prepSummary"] = prep_summary

        meetings_json.append(meeting_obj)

    schedule: Dict[str, Any] = {
        "date": date,
        "meetings": meetings_json,
    }

    # Add optional overview fields if available from AI enrichment
    if context.get('greeting'):
        schedule["greeting"] = context['greeting']
    if context.get('summary'):
        schedule["summary"] = context['summary']
    if context.get('focus'):
        schedule["focus"] = context['focus']

    return schedule


def _build_actions_json(directive: Dict) -> Dict[str, Any]:
    """
    Build the actions.json payload from directive data.

    Flattens the overdue / due_today / due_this_week / waiting_on
    groups into a single actions array with status and priority fields
    matching the actions.schema.json contract.

    Args:
        directive: The full directive dictionary

    Returns:
        Dictionary ready to serialize as actions.json
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    raw_actions = directive.get('actions', {})

    overdue = raw_actions.get('overdue', [])
    due_today = raw_actions.get('due_today', [])
    due_this_week = raw_actions.get('due_this_week', [])
    waiting_on = raw_actions.get('waiting_on', [])

    actions_list: List[Dict[str, Any]] = []
    seen_ids: set = set()

    def _make_action_id(prefix: str, index: int, title: str) -> str:
        slug = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')[:30]
        return f"{prefix}-{index:03d}-{slug}"

    # Overdue items -> P1, pending, is_overdue=True
    for i, task in enumerate(overdue):
        aid = _make_action_id("overdue", i, task.get('title', ''))
        if aid in seen_ids:
            aid = f"{aid}-dup{i}"
        seen_ids.add(aid)

        actions_list.append({
            "id": aid,
            "title": task.get('title', 'Unknown'),
            "account": task.get('account'),
            "priority": "P1",
            "status": "pending",
            "dueDate": task.get('due'),
            "isOverdue": True,
            "daysOverdue": task.get('days_overdue', 0),
            "context": task.get('context'),
            "source": task.get('source'),
        })

    # Due today -> P1, pending
    for i, task in enumerate(due_today):
        aid = _make_action_id("today", i, task.get('title', ''))
        if aid in seen_ids:
            aid = f"{aid}-dup{i}"
        seen_ids.add(aid)

        actions_list.append({
            "id": aid,
            "title": task.get('title', 'Unknown'),
            "account": task.get('account'),
            "priority": "P1",
            "status": "pending",
            "dueDate": task.get('due'),
            "isOverdue": False,
            "context": task.get('context'),
            "source": task.get('source'),
        })

    # Due this week -> P2, pending
    for i, task in enumerate(due_this_week):
        aid = _make_action_id("week", i, task.get('title', ''))
        if aid in seen_ids:
            aid = f"{aid}-dup{i}"
        seen_ids.add(aid)

        actions_list.append({
            "id": aid,
            "title": task.get('title', 'Unknown'),
            "account": task.get('account'),
            "priority": "P2",
            "status": "pending",
            "dueDate": task.get('due'),
            "isOverdue": False,
            "context": task.get('context'),
            "source": task.get('source'),
        })

    # Waiting on -> P2, waiting
    for i, item in enumerate(waiting_on):
        aid = _make_action_id("waiting", i, item.get('what', ''))
        if aid in seen_ids:
            aid = f"{aid}-dup{i}"
        seen_ids.add(aid)

        actions_list.append({
            "id": aid,
            "title": f"Waiting: {item.get('what', 'Unknown')}",
            "account": item.get('who'),
            "priority": "P2",
            "status": "waiting",
            "context": item.get('context'),
        })

    return {
        "date": date,
        "summary": {
            "overdue": len(overdue),
            "dueToday": len(due_today),
            "dueThisWeek": len(due_this_week),
            "waitingOn": len(waiting_on),
        },
        "actions": actions_list,
    }


def _build_emails_json(directive: Dict) -> Dict[str, Any]:
    """
    Build the emails.json payload from directive data.

    Conforms to templates/schemas/emails.schema.json.

    Args:
        directive: The full directive dictionary

    Returns:
        Dictionary ready to serialize as emails.json
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    raw_emails = directive.get('emails', {})

    high_priority = raw_emails.get('high_priority', [])
    medium_count = raw_emails.get('medium_count', 0)
    low_count = raw_emails.get('low_count', 0)

    emails_list: List[Dict[str, Any]] = []

    for i, email in enumerate(high_priority):
        eid = email.get('id', f"email-{i:03d}")
        emails_list.append({
            "id": eid,
            "sender": email.get('from', 'Unknown'),
            "senderEmail": email.get('from_email', ''),
            "subject": email.get('subject', 'No subject'),
            "snippet": email.get('snippet'),
            "priority": "high",
            "received": email.get('date'),
            "emailType": email.get('type'),
            "recommendedAction": email.get('recommended_action'),
            "actionOwner": email.get('action_owner'),
        })

    return {
        "date": date,
        "stats": {
            "highPriority": len(high_priority),
            "normalPriority": medium_count + low_count,
            "needsAction": len([
                e for e in high_priority
                if e.get('action_owner', '').lower() in ('you', 'me', '')
            ]),
        },
        "emails": emails_list,
    }


def _build_prep_json(
    meeting: Dict,
    meeting_type: str,
    meeting_id: str,
    meeting_context: Optional[Dict],
) -> Dict[str, Any]:
    """
    Build an individual meeting prep JSON document.

    Conforms to templates/schemas/prep.schema.json.

    Args:
        meeting: Meeting dict from directive's meetings section
        meeting_type: Normalized meeting type string
        meeting_id: The stable meeting ID
        meeting_context: Optional context block from directive

    Returns:
        Dictionary ready to serialize as preps/{meeting_id}.json
    """
    account = meeting.get('account')
    account_data = {}
    attendees_raw: List[Dict] = []

    if meeting_context:
        account_data = meeting_context.get('account_data', {})
        attendees_raw = meeting_context.get('attendees', [])

    # Quick context from account data
    quick_context: Dict[str, str] = {}
    for key in ('ring', 'arr', 'renewal', 'health', 'tier', 'csm', 'stage'):
        val = account_data.get(key)
        if val:
            quick_context[key.title()] = str(val)

    # Attendees
    attendees = []
    for att in attendees_raw:
        attendees.append({
            "name": att.get('name', att.get('email', 'Unknown')),
            "role": att.get('role'),
            "focus": att.get('focus'),
        })

    # Build start/end time display
    start_display = meeting.get('start_display', '')
    end_display = meeting.get('end_display', '')
    time_range = f"{start_display} - {end_display}" if start_display and end_display else start_display

    prep: Dict[str, Any] = {
        "meetingId": meeting_id,
        "title": meeting.get('title', meeting.get('summary', 'Meeting')),
        "type": meeting_type,
    }

    if time_range:
        prep["timeRange"] = time_range
    if account:
        prep["account"] = account
    if meeting_context and meeting_context.get('narrative'):
        prep["meetingContext"] = meeting_context['narrative']
    if quick_context:
        prep["quickContext"] = quick_context
    if attendees:
        prep["attendees"] = attendees

    # Merge in any extra context fields if the directive provides them
    if meeting_context:
        for field in ('since_last', 'current_state', 'risks',
                      'talking_points', 'questions', 'key_principles'):
            camel = re.sub(r'_([a-z])', lambda m: m.group(1).upper(), field)
            val = meeting_context.get(field)
            if val:
                prep[camel] = val

        # Strategic programs -> array of {name, status} objects
        programs = meeting_context.get('strategic_programs')
        if programs:
            prep["strategicPrograms"] = [
                {"name": p.get('name', str(p)), "status": p.get('status', 'in_progress')}
                if isinstance(p, dict) else {"name": str(p), "status": "in_progress"}
                for p in programs
            ]

        # Open items -> array of {title, dueDate, context, isOverdue}
        open_items = meeting_context.get('open_items')
        if open_items:
            prep["openItems"] = [
                {
                    "title": item.get('title', str(item)) if isinstance(item, dict) else str(item),
                    "dueDate": item.get('due_date') if isinstance(item, dict) else None,
                    "context": item.get('context') if isinstance(item, dict) else None,
                    "isOverdue": item.get('is_overdue', False) if isinstance(item, dict) else False,
                }
                for item in open_items
            ]

        # References -> array of {label, path, lastUpdated}
        references = meeting_context.get('references')
        if references:
            prep["references"] = [
                {
                    "label": ref.get('label', str(ref)) if isinstance(ref, dict) else str(ref),
                    "path": ref.get('path') if isinstance(ref, dict) else None,
                    "lastUpdated": ref.get('last_updated') if isinstance(ref, dict) else None,
                }
                for ref in references
            ]

    return prep


def write_json_data(directive: Dict) -> List[Path]:
    """
    Write JSON data files to _today/data/ for consumption by the Tauri frontend.

    This is the JSON-primary data path (Track B item 2.0a). The Tauri Rust
    backend reads these files via json_loader.rs, falling back to markdown
    parsing when they are absent.

    Generated files:
        - data/schedule.json   — meetings with classifications
        - data/actions.json    — all action items (flat list)
        - data/emails.json     — email summaries
        - data/preps/*.json    — one per meeting with context
        - data/manifest.json   — generation metadata and file index

    All JSON uses camelCase keys to match the serde(rename_all = "camelCase")
    annotations in src-tauri/src/json_loader.rs.

    Note: The schemas under templates/schemas/ currently use snake_case keys.
    This is a known mismatch (RAIDD candidate). The Rust consumer is
    authoritative.

    Args:
        directive: The full directive dictionary

    Returns:
        List of paths to written JSON files
    """
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    PREPS_DIR.mkdir(parents=True, exist_ok=True)

    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    meetings_by_type = directive.get('meetings', {})
    meeting_contexts = directive.get('meeting_contexts', [])

    files_written: List[Path] = []
    prep_manifest_paths: List[str] = []

    # --- schedule.json ---
    schedule_data = _build_schedule_json(directive)
    schedule_path = DATA_DIR / "schedule.json"
    with open(schedule_path, 'w') as f:
        json.dump(schedule_data, f, indent=2, default=str)
    files_written.append(schedule_path)

    # --- actions.json ---
    actions_data = _build_actions_json(directive)
    actions_path = DATA_DIR / "actions.json"
    with open(actions_path, 'w') as f:
        json.dump(actions_data, f, indent=2, default=str)
    files_written.append(actions_path)

    # --- emails.json ---
    emails_data = _build_emails_json(directive)
    emails_path = DATA_DIR / "emails.json"
    with open(emails_path, 'w') as f:
        json.dump(emails_data, f, indent=2, default=str)
    files_written.append(emails_path)

    # --- preps/*.json ---
    # Write a prep file for each meeting that has associated context.
    # Walk the meetings_by_type dict and match against meeting_contexts.
    events = directive.get('calendar', {}).get('events', [])

    for mtype, meeting_list in meetings_by_type.items():
        normalized_type = _normalize_meeting_type(mtype)

        for meeting in meeting_list:
            account = meeting.get('account')
            mc = _find_meeting_context(account, meeting_contexts) if account else None

            # Skip meetings with no useful context to write
            if not mc and not account:
                continue

            # Find the matching calendar event to build a stable ID
            event_id = meeting.get('event_id')
            matched_event = None
            for ev in events:
                if ev.get('id') == event_id:
                    matched_event = ev
                    break

            if matched_event:
                meeting_id = _make_meeting_id(matched_event, normalized_type)
            else:
                # Build ID from meeting fields directly
                title = meeting.get('title', meeting.get('summary', account or 'meeting'))
                slug = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')[:40]
                start = meeting.get('start_display', meeting.get('start', ''))
                time_part = re.sub(r'[^0-9]', '', start)[:4] if start else '0000'
                meeting_id = f"{time_part}-{normalized_type}-{slug}"

            prep_data = _build_prep_json(meeting, normalized_type, meeting_id, mc)
            prep_path = PREPS_DIR / f"{meeting_id}.json"
            with open(prep_path, 'w') as f:
                json.dump(prep_data, f, indent=2, default=str)
            files_written.append(prep_path)
            prep_manifest_paths.append(f"preps/{meeting_id}.json")

    # --- manifest.json ---
    # Gather statistics
    total_meetings = len(schedule_data.get('meetings', []))
    customer_count = sum(
        1 for m in schedule_data.get('meetings', [])
        if m.get('type') in ('customer', 'qbr')
    )
    internal_count = sum(
        1 for m in schedule_data.get('meetings', [])
        if m.get('type') in ('internal', 'team_sync', 'one_on_one', 'all_hands')
    )
    personal_count = sum(
        1 for m in schedule_data.get('meetings', [])
        if m.get('type') == 'personal'
    )

    raw_actions = directive.get('actions', {})
    actions_due = len(raw_actions.get('due_today', []))
    actions_overdue = len(raw_actions.get('overdue', []))
    emails_flagged = len(directive.get('emails', {}).get('high_priority', []))

    manifest: Dict[str, Any] = {
        "schemaVersion": "1.0.0",
        "date": date,
        "generatedAt": datetime.utcnow().isoformat() + "Z",
        "partial": False,
        "files": {
            "schedule": "schedule.json",
            "actions": "actions.json",
            "emails": "emails.json",
            "preps": prep_manifest_paths,
        },
        "stats": {
            "totalMeetings": total_meetings,
            "customerMeetings": customer_count,
            "internalMeetings": internal_count,
            "personalMeetings": personal_count,
            "actionsDue": actions_due,
            "actionsOverdue": actions_overdue,
            "emailsFlagged": emails_flagged,
        },
    }

    # Include profile if available from directive context
    profile = context.get('profile')
    if profile:
        manifest["profile"] = profile

    manifest_path = DATA_DIR / "manifest.json"
    with open(manifest_path, 'w') as f:
        json.dump(manifest, f, indent=2, default=str)
    files_written.append(manifest_path)

    return files_written


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
    parser.add_argument(
        '--json', action='store_true', default=True, dest='write_json',
        help='Write JSON data files to _today/data/ (default: enabled)',
    )
    parser.add_argument(
        '--no-json', action='store_false', dest='write_json',
        help='Skip JSON data file generation',
    )
    parser.add_argument(
        '--json-only', action='store_true', default=False,
        help='Write ONLY JSON data files, skip markdown generation',
    )
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
    skip_markdown = args.json_only

    # ---- Markdown delivery (unless --json-only) ----
    if not skip_markdown:
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
    else:
        print("\n  Skipping markdown (--json-only mode)")

    # ---- JSON delivery (unless --no-json) ----
    if args.write_json:
        print("\nWriting JSON data files to _today/data/...")
        try:
            json_files = write_json_data(directive)
            files_written.extend(json_files)
            for jf in json_files:
                # Show path relative to _today/
                try:
                    rel = jf.relative_to(TODAY_DIR)
                except ValueError:
                    rel = jf.name
                print(f"  ✅ {rel}")
        except Exception as e:
            # JSON generation should not block markdown delivery.
            # Log the error and continue so the briefing is still usable.
            print(f"  ⚠️  JSON generation failed: {e}", file=sys.stderr)
            import traceback
            traceback.print_exc(file=sys.stderr)

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
        # Show relative paths where possible
        try:
            rel = f.relative_to(TODAY_DIR)
            print(f"  - {rel}")
        except ValueError:
            print(f"  - {f.name}")

    print(f"\nOutput directory: {TODAY_DIR}")
    if args.write_json:
        print(f"JSON data directory: {DATA_DIR}")
    print("\n/today workflow complete!")

    return 0


if __name__ == "__main__":
    sys.exit(main())
