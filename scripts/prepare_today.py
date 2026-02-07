#!/usr/bin/env python3
"""Phase 1: Today Preparation Script for DailyOS Daybreak.

Gathers deterministic data (calendar, email, actions, workspace state) and
writes a JSON directive for Claude Code to enrich in Phase 2.

Thin orchestrator per ADR-0030: all data-gathering logic lives in ops/.

Output file:
    {workspace}/_today/data/today-directive.json

Calling convention (matches pty.rs):
    - cwd is set to the workspace root by the Rust executor
    - WORKSPACE env var is also set
    - No positional arguments in production

For manual testing:
    python3 scripts/prepare_today.py /path/to/workspace
"""

from __future__ import annotations

import sys
from datetime import datetime, date, timezone
from pathlib import Path
from typing import Any

from ops.config import (
    resolve_workspace,
    load_config,
    get_profile,
    get_user_domain,
    build_account_domain_hints,
    write_json,
    _info,
    _warn,
)
from ops.calendar_fetch import fetch_and_classify
from ops.email_fetch import fetch_and_classify_emails
from ops.action_parse import parse_workspace_actions
from ops.meeting_prep import gather_all_meeting_contexts
from ops.gap_analysis import compute_gaps


# ---------------------------------------------------------------------------
# Workspace file inventory (orchestrator-specific)
# ---------------------------------------------------------------------------

def inventory_today_files(workspace: Path) -> list[str]:
    """List existing files in {workspace}/_today/."""
    today_dir = workspace / "_today"
    if not today_dir.is_dir():
        return []

    try:
        return sorted(
            f.name
            for f in today_dir.iterdir()
            if f.is_file() and not f.name.startswith(".")
        )
    except OSError:
        return []


def count_inbox_pending(workspace: Path) -> int:
    """Count files pending in {workspace}/_inbox/."""
    inbox_dir = workspace / "_inbox"
    if not inbox_dir.is_dir():
        return 0

    try:
        return sum(
            1 for f in inbox_dir.iterdir()
            if f.is_file() and not f.name.startswith(".")
        )
    except OSError:
        return 0


# ---------------------------------------------------------------------------
# AI task generation (orchestrator-specific)
# ---------------------------------------------------------------------------

def generate_ai_tasks(
    classified: list[dict[str, Any]],
    time_status: dict[str, list[str]],
    emails_high: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Generate the list of tasks for Claude to execute in Phase 2.

    Each task has: type, event_id (optional), priority.
    """
    tasks: list[dict[str, Any]] = []
    past_ids = set(time_status.get("past", []))

    for meeting in classified:
        event_id = meeting.get("id", "")
        meeting_type = meeting.get("type", "")

        # Skip past and personal meetings
        if event_id in past_ids:
            continue
        if meeting_type in ("personal", "all_hands"):
            continue

        if meeting_type in ("customer", "qbr"):
            tasks.append({
                "type": "generate_meeting_prep",
                "event_id": event_id,
                "meeting_type": meeting_type,
                "priority": "high",
            })
        elif meeting_type == "training":
            tasks.append({
                "type": "generate_meeting_prep",
                "event_id": event_id,
                "meeting_type": meeting_type,
                "priority": "medium",
            })
        elif meeting_type == "external":
            has_unknown = bool(meeting.get("external_domains"))
            tasks.append({
                "type": "research_unknown_meeting" if has_unknown else "generate_meeting_prep",
                "event_id": event_id,
                "meeting_type": meeting_type,
                "priority": "medium" if has_unknown else "low",
            })
        elif meeting_type in ("internal", "team_sync", "one_on_one"):
            tasks.append({
                "type": "generate_meeting_prep",
                "event_id": event_id,
                "meeting_type": meeting_type,
                "priority": "low",
            })

    # Email summaries for high-priority emails
    for email in emails_high:
        tasks.append({
            "type": "summarize_email",
            "email_id": email.get("id"),
            "thread_id": email.get("thread_id"),
            "priority": "medium",
        })

    # Generate daily briefing narrative
    tasks.append({
        "type": "generate_briefing_narrative",
        "priority": "high",
    })

    return tasks


# ---------------------------------------------------------------------------
# Main orchestrator
# ---------------------------------------------------------------------------

def main() -> int:
    """Orchestrate Phase 1 today preparation."""
    workspace = resolve_workspace()

    print("=" * 60, file=sys.stderr)
    print("PHASE 1: TODAY PREPARATION", file=sys.stderr)
    print("=" * 60, file=sys.stderr)

    now = datetime.now(tz=timezone.utc)
    today = date.today()

    # ---- Load config ----
    config = load_config()
    profile = get_profile(config)
    user_domain = get_user_domain(config)

    _info(f"Workspace: {workspace}")
    _info(f"Profile:   {profile}")
    _info(f"Domain:    {user_domain or '(unknown)'}")

    # ---- Step 1: Context metadata ----
    _info("")
    _info("Step 1: Building context metadata...")

    iso_year, iso_week, _ = today.isocalendar()
    context: dict[str, Any] = {
        "date": today.isoformat(),
        "day_of_week": today.strftime("%A"),
        "week_number": iso_week,
        "year": iso_year,
        "profile": profile,
    }

    _info(f"  Date: {today.isoformat()} ({today.strftime('%A')}), W{iso_week:02d}")

    # ---- Step 2-4: Fetch calendar events + classify + time status ----
    _info("")
    _info("Step 2: Fetching and classifying calendar events...")

    account_hints = build_account_domain_hints(workspace)
    cal = fetch_and_classify(today, today, user_domain, account_hints)

    _info(f"  Found {len(cal.events)} events")
    type_counts = {k: len(v) for k, v in cal.meetings_by_type.items() if v}
    for mt, count in sorted(type_counts.items()):
        _info(f"    {mt}: {count}")

    _info(f"  Past: {len(cal.time_status.get('past', []))}, "
          f"In progress: {len(cal.time_status.get('in_progress', []))}, "
          f"Upcoming: {len(cal.time_status.get('upcoming', []))}")

    # ---- Step 5: Calendar gaps ----
    _info("")
    _info("Step 3: Analyzing calendar gaps...")

    gaps = compute_gaps(cal.events, today)
    total_gap_minutes = sum(g["duration_minutes"] for g in gaps)
    _info(f"  {len(gaps)} gaps totaling {total_gap_minutes} min of focus time")

    # ---- Step 6: Fetch emails ----
    _info("")
    _info("Step 4: Fetching emails...")

    # Build customer domain set from external meeting attendees
    customer_domains: set[str] = set()
    for ev in cal.meetings_by_type.get("customer", []):
        for domain in ev.get("external_domains", []):
            customer_domains.add(domain)

    _info(f"  Account domain hints: {len(account_hints)}")

    email_result = fetch_and_classify_emails(
        customer_domains, user_domain, account_hints,
    )

    _info(f"  Found {len(email_result.all_emails)} unread emails")
    _info(f"  High: {len(email_result.high)}, "
          f"Medium: {email_result.medium_count}, "
          f"Low: {email_result.low_count}")

    # ---- Step 7: Parse actions ----
    _info("")
    _info("Step 5: Parsing action items...")

    actions = parse_workspace_actions(workspace)
    actions_dict = actions.to_dict()

    _info(f"  Overdue: {len(actions_dict['overdue'])}, "
          f"Due today: {len(actions_dict['due_today'])}, "
          f"Due this week: {len(actions_dict['due_this_week'])}, "
          f"Waiting on: {len(actions_dict['waiting_on'])}")

    # ---- Step 8: Meeting contexts (reference approach -- DEC19) ----
    _info("")
    _info("Step 6: Gathering meeting contexts...")

    meeting_contexts = gather_all_meeting_contexts(cal.classified, workspace)
    refs_count = sum(len(ctx.get("refs", {})) for ctx in meeting_contexts)
    _info(f"  {len(meeting_contexts)} meetings with context, {refs_count} file references")

    # ---- Step 9: File inventory ----
    _info("")
    _info("Step 7: Inventorying workspace files...")

    existing_today = inventory_today_files(workspace)
    inbox_pending = count_inbox_pending(workspace)
    _info(f"  Existing _today/ files: {len(existing_today)}")
    _info(f"  Inbox pending: {inbox_pending}")

    # ---- Step 10: Generate AI tasks ----
    _info("")
    _info("Step 8: Generating AI task list...")

    ai_tasks = generate_ai_tasks(cal.classified, cal.time_status, email_result.high)
    _info(f"  {len(ai_tasks)} AI tasks generated")

    # ---- Build directive ----
    # Strip attendee lists from the classified output to keep directive lean.
    lean_events = [
        {
            "id": ev.get("id"),
            "summary": ev.get("summary"),
            "start": ev.get("start"),
            "end": ev.get("end"),
        }
        for ev in cal.events
    ]

    def _lean_meeting(m: dict[str, Any]) -> dict[str, Any]:
        return {k: v for k, v in m.items() if k != "attendees"}

    lean_meetings = {
        mt: [_lean_meeting(m) for m in meetings]
        for mt, meetings in cal.meetings_by_type.items()
    }

    directive: dict[str, Any] = {
        "command": "today",
        "generated_at": now.isoformat(),
        "context": context,
        "calendar": {
            "events": lean_events,
            "past": cal.time_status["past"],
            "in_progress": cal.time_status["in_progress"],
            "upcoming": cal.time_status["upcoming"],
            "gaps": gaps,
        },
        "meetings": lean_meetings,
        "meeting_contexts": meeting_contexts,
        "actions": actions_dict,
        "emails": {
            "high_priority": email_result.high,
            "classified": email_result.all_emails,
            "medium_count": email_result.medium_count,
            "low_count": email_result.low_count,
        },
        "files": {
            "existing_today": existing_today,
            "inbox_pending": inbox_pending,
        },
        "ai_tasks": ai_tasks,
    }

    # ---- Write output ----
    output_path = workspace / "_today" / "data" / "today-directive.json"
    write_json(output_path, directive)

    # ---- Summary ----
    print("", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    print("PHASE 1 COMPLETE", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    print(f"  Directive: {output_path}", file=sys.stderr)
    print(f"  Events:    {len(cal.events)}", file=sys.stderr)
    print(f"  Customer:  {len(cal.meetings_by_type.get('customer', []))}", file=sys.stderr)
    print(f"  Actions:   {len(actions_dict['overdue'])} overdue, {len(actions_dict['due_today'])} due today", file=sys.stderr)
    print(f"  Emails:    {len(email_result.high)} high priority", file=sys.stderr)
    print(f"  AI tasks:  {len(ai_tasks)}", file=sys.stderr)
    print(f"  Focus:     {total_gap_minutes} min available", file=sys.stderr)
    print("", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
