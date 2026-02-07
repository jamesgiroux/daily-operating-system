#!/usr/bin/env python3
"""Phase 1: Week Preparation Script for DailyOS Daybreak.

Gathers deterministic data for the weekly planning workflow and writes
a directive JSON that Claude enriches in Phase 2.

Thin orchestrator per ADR-0030: all data-gathering logic lives in ops/.

Operations performed:
    1. Calculate week number and date range (Mon-Fri)
    2. Fetch the week's calendar events via Google Calendar API
    3. Classify meetings by type (customer, internal, personal, etc.)
    4. Check for overdue actions from SQLite database
    5. Gather meeting contexts for prep
    6. Identify calendar gaps for focus time blocks
    7. Write structured directive to week-directive.json

Usage:
    python3 prepare_week.py [workspace_path]

    workspace_path  Path to the workspace root (defaults to cwd or
                    the WORKSPACE environment variable).

Exit codes:
    0  Success
    1  Fatal error (workspace not found, I/O failure)
"""

from __future__ import annotations

import sys
from datetime import datetime, date, timedelta, timezone
from pathlib import Path
from typing import Any

from ops.config import (
    resolve_workspace,
    load_config,
    get_profile,
    get_user_domain,
    write_json,
    _info,
)
from ops.calendar_fetch import fetch_and_classify
from ops.action_parse import fetch_actions_from_db
from ops.meeting_prep import gather_all_meeting_contexts
from ops.gap_analysis import compute_all_gaps, suggest_focus_blocks


# ---------------------------------------------------------------------------
# Week-specific helpers
# ---------------------------------------------------------------------------

def get_week_bounds(ref_date: date | None = None) -> tuple[date, date, int, int]:
    """Return (monday, friday, iso_week_number, year) for *ref_date*.

    If *ref_date* falls on a weekend, the following Monday's week is used.
    """
    if ref_date is None:
        ref_date = date.today()

    # Monday of the current ISO week
    monday = ref_date - timedelta(days=ref_date.weekday())
    friday = monday + timedelta(days=4)
    iso_year, iso_week, _ = monday.isocalendar()
    return monday, friday, iso_week, iso_year


def format_date_range(monday: date, friday: date) -> str:
    """Human-friendly date range string, e.g. 'February 2-6, 2026'."""
    if monday.month == friday.month:
        return f"{monday.strftime('%B')} {monday.day}-{friday.day}, {friday.year}"
    return f"{monday.strftime('%B')} {monday.day} - {friday.strftime('%B')} {friday.day}, {friday.year}"


# ---------------------------------------------------------------------------
# Main orchestrator
# ---------------------------------------------------------------------------

def main() -> int:
    """Orchestrate Phase 1 week preparation."""
    workspace = resolve_workspace()

    print("=" * 60, file=sys.stderr)
    print("PHASE 1: WEEK PREPARATION", file=sys.stderr)
    print("=" * 60, file=sys.stderr)

    # ---- Step 1: Week context ----
    now = datetime.now(tz=timezone.utc)
    monday, friday, week_num, year = get_week_bounds(now.date())
    week_label = f"W{week_num:02d}"
    date_range = format_date_range(monday, friday)

    _info(f"\n  Week {week_label}: {date_range}")

    config = load_config()
    profile = get_profile(config)
    user_domain = get_user_domain(config)

    context: dict[str, Any] = {
        "weekNumber": week_label,
        "year": year,
        "monday": monday.isoformat(),
        "friday": friday.isoformat(),
        "dateRange": date_range,
        "profile": profile,
    }

    # ---- Step 2: Fetch and classify calendar events ----
    _info("")
    _info("Step 2: Fetching and classifying calendar events...")

    cal = fetch_and_classify(monday, friday, user_domain)

    _info(f"  Found {len(cal.events)} events")
    type_counts: dict[str, int] = {}
    for ev in cal.classified:
        mt = ev.get("type", "unknown")
        type_counts[mt] = type_counts.get(mt, 0) + 1
    for mt, count in sorted(type_counts.items()):
        _info(f"    {mt}: {count}")

    # ---- Step 3: Actions from SQLite ----
    _info("")
    _info("Step 3: Checking actions...")

    actions = fetch_actions_from_db(workspace, monday, friday)
    _info(f"    Overdue: {len(actions['overdue'])}")
    _info(f"    Due this week: {len(actions['thisWeek'])}")

    # ---- Step 4: Meeting contexts ----
    _info("")
    _info("Step 4: Gathering meeting contexts...")

    meeting_contexts = gather_all_meeting_contexts(cal.classified, workspace)
    refs_count = sum(len(ctx.get("refs", {})) for ctx in meeting_contexts)
    _info(f"  {len(meeting_contexts)} meetings with context, {refs_count} file references")

    # ---- Step 5: Gap analysis ----
    _info("")
    _info("Step 5: Analyzing calendar gaps...")

    gaps_by_day = compute_all_gaps(cal.events_by_day, monday)
    suggestions = suggest_focus_blocks(gaps_by_day)

    total_gap_minutes = sum(
        g["duration_minutes"] for day_gaps in gaps_by_day.values() for g in day_gaps
    )
    _info(f"    Total available focus time: {total_gap_minutes} min")
    _info(f"    Focus block suggestions: {len(suggestions)}")

    # ---- Step 6: Build directive ----
    # Strip attendee lists from the by_day output to keep the directive lean.
    serializable_by_day: dict[str, list[dict[str, Any]]] = {}
    for day_name, day_events in cal.events_by_day.items():
        serializable_by_day[day_name] = [
            {
                "id": ev.get("id"),
                "title": ev.get("title"),
                "start": ev.get("start"),
                "end": ev.get("end"),
                "type": ev.get("type"),
                "external_domains": ev.get("external_domains"),
            }
            for ev in day_events
        ]

    directive: dict[str, Any] = {
        "command": "week",
        "generatedAt": now.isoformat(),
        "context": context,
        "meetingsByDay": serializable_by_day,
        "meetingContexts": meeting_contexts,
        "actions": actions,
        "timeBlocks": {
            "gapsByDay": gaps_by_day,
            "suggestions": suggestions,
        },
    }

    # ---- Write output ----
    output_path = workspace / "_today" / "data" / "week-directive.json"
    write_json(output_path, directive)

    # ---- Summary ----
    print("", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    print("PHASE 1 COMPLETE", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    print(f"  Directive: {output_path}", file=sys.stderr)
    print(f"  Events:    {len(cal.events)}", file=sys.stderr)
    print(f"  Overdue:   {len(actions['overdue'])}", file=sys.stderr)
    print(f"  This week: {len(actions['thisWeek'])}", file=sys.stderr)
    print(f"  Contexts:  {len(meeting_contexts)}", file=sys.stderr)
    print(f"  Focus:     {total_gap_minutes} min available", file=sys.stderr)
    print("", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
