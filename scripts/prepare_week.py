#!/usr/bin/env python3
"""Phase 1: Week Preparation Script for DailyOS Daybreak.

Gathers deterministic data for the weekly planning workflow and writes
a directive JSON that Claude enriches in Phase 2.

Operations performed:
    1. Calculate week number and date range (Mon-Fri)
    2. Fetch the week's calendar events via Google Calendar API
    3. Classify meetings by type (customer, internal, personal, etc.)
    4. Check for overdue actions from SQLite database
    5. Identify calendar gaps for focus time blocks
    6. Write structured directive to week-directive.json

Usage:
    python3 prepare_week.py [workspace_path]

    workspace_path  Path to the workspace root (defaults to cwd or
                    the WORKSPACE environment variable).

Exit codes:
    0  Success
    1  Fatal error (workspace not found, I/O failure)
"""

from __future__ import annotations

import json
import os
import sqlite3
import sys
from datetime import datetime, date, timedelta, timezone
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

TOKEN_PATH = Path.home() / ".dailyos" / "google" / "token.json"
CREDENTIALS_PATH = Path.home() / ".dailyos" / "google" / "credentials.json"

SCOPES = ["https://www.googleapis.com/auth/calendar.readonly"]

# Work-day boundaries for gap analysis (hours, 24h clock)
WORK_DAY_START_HOUR = 9
WORK_DAY_END_HOUR = 17

# Minimum gap length (minutes) worth reporting
MIN_GAP_MINUTES = 30

# All-Hands attendee threshold (per MEETING-TYPES.md)
ALL_HANDS_THRESHOLD = 50


# ---------------------------------------------------------------------------
# Helpers: workspace resolution
# ---------------------------------------------------------------------------

def resolve_workspace() -> Path:
    """Determine workspace path from argv, env, or cwd.

    Priority order:
        1. sys.argv[1] if provided
        2. WORKSPACE environment variable (set by Rust executor)
        3. Current working directory
    """
    if len(sys.argv) > 1:
        ws = Path(sys.argv[1])
    elif "WORKSPACE" in os.environ:
        ws = Path(os.environ["WORKSPACE"])
    else:
        ws = Path.cwd()

    ws = ws.resolve()
    if not ws.is_dir():
        print(f"ERROR: Workspace directory does not exist: {ws}", file=sys.stderr)
        sys.exit(1)
    return ws


# ---------------------------------------------------------------------------
# Helpers: week calculation
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


DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]


# ---------------------------------------------------------------------------
# Google Calendar integration
# ---------------------------------------------------------------------------

def _build_calendar_service() -> Any | None:
    """Authenticate and return a Google Calendar API service, or None."""
    try:
        from google.oauth2.credentials import Credentials
        from google.auth.transport.requests import Request
        from googleapiclient.discovery import build
    except ImportError:
        print(
            "  WARN: google-api-python-client not installed. "
            "Calendar data will be empty.",
            file=sys.stderr,
        )
        return None

    if not TOKEN_PATH.exists():
        print(
            f"  WARN: Google token not found at {TOKEN_PATH}. "
            "Calendar data will be empty.",
            file=sys.stderr,
        )
        return None

    creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)

    if creds.expired and creds.refresh_token:
        try:
            creds.refresh(Request())
            TOKEN_PATH.write_text(creds.to_json())
        except Exception as exc:
            print(f"  WARN: Token refresh failed: {exc}", file=sys.stderr)
            return None

    if not creds.valid:
        print("  WARN: Google credentials are invalid.", file=sys.stderr)
        return None

    return build("calendar", "v3", credentials=creds, cache_discovery=False)


def fetch_week_events(
    monday: date,
    friday: date,
) -> list[dict[str, Any]]:
    """Fetch calendar events for Mon-Fri of the given week.

    Returns a list of dicts with keys: id, summary, start, end, attendees,
    description, location.
    """
    service = _build_calendar_service()
    if service is None:
        return []

    time_min = datetime.combine(monday, datetime.min.time()).isoformat() + "Z"
    time_max = datetime.combine(
        friday + timedelta(days=1), datetime.min.time()
    ).isoformat() + "Z"

    events: list[dict[str, Any]] = []
    page_token: str | None = None

    try:
        while True:
            result = (
                service.events()
                .list(
                    calendarId="primary",
                    timeMin=time_min,
                    timeMax=time_max,
                    singleEvents=True,
                    orderBy="startTime",
                    maxResults=250,
                    pageToken=page_token,
                )
                .execute()
            )

            for item in result.get("items", []):
                start_raw = item.get("start", {})
                end_raw = item.get("end", {})

                events.append(
                    {
                        "id": item.get("id", ""),
                        "summary": item.get("summary", "(No title)"),
                        "start": start_raw.get("dateTime", start_raw.get("date", "")),
                        "end": end_raw.get("dateTime", end_raw.get("date", "")),
                        "attendees": [
                            a.get("email", "")
                            for a in item.get("attendees", [])
                        ],
                        "description": item.get("description", ""),
                        "location": item.get("location", ""),
                    }
                )

            page_token = result.get("nextPageToken")
            if not page_token:
                break

    except Exception as exc:
        print(f"  WARN: Calendar API error: {exc}", file=sys.stderr)

    return events


# ---------------------------------------------------------------------------
# Meeting classification
# ---------------------------------------------------------------------------

def _parse_event_dt(time_str: str) -> datetime | None:
    """Parse an ISO datetime string to a naive datetime."""
    if not time_str:
        return None
    try:
        if "T" in time_str:
            dt = datetime.fromisoformat(time_str.replace("Z", "+00:00"))
            return dt.replace(tzinfo=None)
        return datetime.strptime(time_str, "%Y-%m-%d")
    except (ValueError, TypeError):
        return None


def _load_config() -> dict[str, Any]:
    """Load ~/.dailyos/config.json if available."""
    config_path = Path.home() / ".dailyos" / "config.json"
    if config_path.exists():
        try:
            return json.loads(config_path.read_text())
        except (json.JSONDecodeError, OSError):
            pass
    return {}


def _get_user_domain(config: dict[str, Any]) -> str:
    """Derive the user's email domain from the Google token.

    Falls back to empty string if unavailable.
    """
    # Try reading from token.json (contains email field after OAuth)
    if TOKEN_PATH.exists():
        try:
            token_data = json.loads(TOKEN_PATH.read_text())
            # Google tokens may contain an "account" or we can infer from
            # the client_id, but the simplest is to look at the token's
            # granted scopes or just rely on any embedded email.
            # Most tokens don't contain the email directly, so this is
            # best-effort.
        except (json.JSONDecodeError, OSError):
            pass

    return ""


def classify_meeting(
    event: dict[str, Any],
    user_domain: str,
) -> dict[str, Any]:
    """Classify a calendar event according to MEETING-TYPES.md algorithm.

    Returns a dict with classification metadata merged onto the event.
    """
    title = event.get("summary", "")
    title_lower = title.lower()
    attendees: list[str] = event.get("attendees", [])
    attendee_count = len(attendees)

    result: dict[str, Any] = {
        "id": event.get("id", ""),
        "title": title,
        "start": event.get("start", ""),
        "end": event.get("end", ""),
        "attendees": attendees,
    }

    # ------- Step 1: Scale-based override -------
    if attendee_count >= ALL_HANDS_THRESHOLD:
        result["type"] = "all_hands"
        result["prep_depth"] = "none"
        return result

    # ------- Step 2: Title keyword overrides -------
    if any(kw in title_lower for kw in ("qbr", "business review", "quarterly review")):
        result["type"] = "qbr"
        result["prep_depth"] = "comprehensive"
        return result

    if any(kw in title_lower for kw in ("training", "enablement", "workshop")):
        result["type"] = "training"
        result["prep_depth"] = "moderate"
        return result

    if any(kw in title_lower for kw in ("all hands", "town hall")):
        result["type"] = "all_hands"
        result["prep_depth"] = "none"
        return result

    # ------- Step 3: Internal vs External -------
    if user_domain:
        external = [a for a in attendees if not a.endswith(f"@{user_domain}")]
        internal = [a for a in attendees if a.endswith(f"@{user_domain}")]
    else:
        # Without a known domain, treat all attendees as potentially external
        external = attendees
        internal = []

    has_external = len(external) > 0

    if has_external:
        # Basic heuristic: personal calendars (gmail, outlook, etc.)
        personal_domains = {
            "gmail.com",
            "googlemail.com",
            "outlook.com",
            "hotmail.com",
            "yahoo.com",
            "icloud.com",
            "me.com",
            "live.com",
        }
        external_domains = {
            e.split("@")[-1].lower() for e in external if "@" in e
        }
        if external_domains and external_domains.issubset(personal_domains):
            result["type"] = "personal"
            result["prep_depth"] = "none"
            return result

        # Default external -> customer (can be refined with domain mapping)
        result["type"] = "customer"
        result["prep_depth"] = "full"
        result["external_domains"] = sorted(external_domains)
        return result

    # ------- Step 4: Internal classification -------
    if attendee_count == 2:
        if any(kw in title_lower for kw in ("1:1", "1-1", "one on one")):
            result["type"] = "one_on_one"
            result["prep_depth"] = "personal"
            return result
        # 2-person internal: default to 1:1
        result["type"] = "one_on_one"
        result["prep_depth"] = "personal"
        return result

    if any(kw in title_lower for kw in ("standup", "sync", "scrum", "daily")):
        result["type"] = "team_sync"
        result["prep_depth"] = "light"
        return result

    # Catch-all: personal events (no attendees or only organizer)
    if attendee_count <= 1:
        result["type"] = "personal"
        result["prep_depth"] = "none"
        return result

    result["type"] = "internal"
    result["prep_depth"] = "light"
    return result


# ---------------------------------------------------------------------------
# Organize events by day
# ---------------------------------------------------------------------------

def organize_by_day(
    events: list[dict[str, Any]],
    monday: date,
) -> dict[str, list[dict[str, Any]]]:
    """Bucket classified events into Monday-Friday lists."""
    by_day: dict[str, list[dict[str, Any]]] = {d: [] for d in DAY_NAMES}

    for ev in events:
        dt = _parse_event_dt(ev.get("start", ""))
        if dt is None:
            continue
        weekday = dt.weekday()
        if 0 <= weekday <= 4:
            by_day[DAY_NAMES[weekday]].append(ev)

    return by_day


# ---------------------------------------------------------------------------
# Calendar gap analysis
# ---------------------------------------------------------------------------

def compute_gaps_for_day(
    day_events: list[dict[str, Any]],
    day_date: date,
) -> list[dict[str, Any]]:
    """Find free time blocks between meetings on a given day.

    Only returns gaps >= MIN_GAP_MINUTES within work hours.
    """
    day_start = datetime.combine(day_date, datetime.min.time()).replace(
        hour=WORK_DAY_START_HOUR
    )
    day_end = datetime.combine(day_date, datetime.min.time()).replace(
        hour=WORK_DAY_END_HOUR
    )

    # Sort events by start time
    parsed_intervals: list[tuple[datetime, datetime]] = []
    for ev in day_events:
        s = _parse_event_dt(ev.get("start", ""))
        e = _parse_event_dt(ev.get("end", ""))
        if s and e:
            parsed_intervals.append((s, e))
    parsed_intervals.sort(key=lambda x: x[0])

    gaps: list[dict[str, Any]] = []
    cursor = day_start

    for start, end in parsed_intervals:
        # Clamp to work hours
        start = max(start, day_start)
        end = min(end, day_end)

        if start > cursor:
            duration = int((start - cursor).total_seconds() / 60)
            if duration >= MIN_GAP_MINUTES:
                gaps.append(
                    {
                        "start": cursor.isoformat(),
                        "end": start.isoformat(),
                        "duration_minutes": duration,
                    }
                )
        cursor = max(cursor, end)

    # Gap after last meeting
    if cursor < day_end:
        duration = int((day_end - cursor).total_seconds() / 60)
        if duration >= MIN_GAP_MINUTES:
            gaps.append(
                {
                    "start": cursor.isoformat(),
                    "end": day_end.isoformat(),
                    "duration_minutes": duration,
                }
            )

    return gaps


def compute_all_gaps(
    by_day: dict[str, list[dict[str, Any]]],
    monday: date,
) -> dict[str, list[dict[str, Any]]]:
    """Compute gaps for each weekday."""
    result: dict[str, list[dict[str, Any]]] = {}
    for i, day_name in enumerate(DAY_NAMES):
        day_date = monday + timedelta(days=i)
        result[day_name] = compute_gaps_for_day(by_day.get(day_name, []), day_date)
    return result


def suggest_focus_blocks(
    gaps: dict[str, list[dict[str, Any]]],
) -> list[dict[str, Any]]:
    """Generate focus-time suggestions from large gaps.

    Prioritizes morning slots (deep work) and afternoon slots (admin).
    """
    suggestions: list[dict[str, Any]] = []

    for day_name, day_gaps in gaps.items():
        for gap in day_gaps:
            if gap["duration_minutes"] < MIN_GAP_MINUTES:
                continue

            start_dt = _parse_event_dt(gap["start"])
            if start_dt is None:
                continue

            block_type = "Deep Work" if start_dt.hour < 12 else "Admin / Follow-up"
            suggestions.append(
                {
                    "day": day_name,
                    "start": gap["start"],
                    "end": gap["end"],
                    "duration_minutes": gap["duration_minutes"],
                    "suggested_use": block_type,
                }
            )

    return suggestions


# ---------------------------------------------------------------------------
# SQLite: overdue and this-week actions
# ---------------------------------------------------------------------------

def _open_db(workspace: Path) -> sqlite3.Connection | None:
    """Open the DailyOS SQLite database if it exists."""
    db_path = workspace / "_today" / "data" / "dailyos.db"
    if not db_path.exists():
        return None
    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        return conn
    except sqlite3.Error as exc:
        print(f"  WARN: Could not open database: {exc}", file=sys.stderr)
        return None


def _table_exists(conn: sqlite3.Connection, table_name: str) -> bool:
    """Check whether a table exists in the database."""
    cursor = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
        (table_name,),
    )
    return cursor.fetchone() is not None


def fetch_actions(
    workspace: Path,
    monday: date,
    friday: date,
) -> dict[str, list[dict[str, Any]]]:
    """Read overdue and this-week actions from SQLite.

    Returns {"overdue": [...], "thisWeek": [...]}.
    """
    result: dict[str, list[dict[str, Any]]] = {
        "overdue": [],
        "thisWeek": [],
    }

    conn = _open_db(workspace)
    if conn is None:
        return result

    if not _table_exists(conn, "actions"):
        conn.close()
        return result

    today_str = date.today().isoformat()
    monday_str = monday.isoformat()
    friday_str = friday.isoformat()

    try:
        # Overdue: due before today and not completed
        cursor = conn.execute(
            """
            SELECT id, title, priority, status, due_date, account_id
            FROM actions
            WHERE status != 'completed'
              AND due_date IS NOT NULL
              AND due_date < ?
            ORDER BY due_date ASC
            """,
            (today_str,),
        )
        for row in cursor.fetchall():
            due = row["due_date"]
            days_overdue = (date.today() - date.fromisoformat(due)).days if due else 0
            result["overdue"].append(
                {
                    "id": row["id"],
                    "title": row["title"],
                    "priority": row["priority"],
                    "status": row["status"],
                    "dueDate": due,
                    "accountId": row["account_id"],
                    "daysOverdue": days_overdue,
                }
            )

        # This week: due between monday and friday (inclusive), not completed
        cursor = conn.execute(
            """
            SELECT id, title, priority, status, due_date, account_id
            FROM actions
            WHERE status != 'completed'
              AND due_date IS NOT NULL
              AND due_date >= ?
              AND due_date <= ?
            ORDER BY due_date ASC
            """,
            (monday_str, friday_str),
        )
        for row in cursor.fetchall():
            result["thisWeek"].append(
                {
                    "id": row["id"],
                    "title": row["title"],
                    "priority": row["priority"],
                    "status": row["status"],
                    "dueDate": row["due_date"],
                    "accountId": row["account_id"],
                }
            )
    except sqlite3.Error as exc:
        print(f"  WARN: Action query failed: {exc}", file=sys.stderr)
    finally:
        conn.close()

    return result


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    """Orchestrate Phase 1 week preparation."""
    workspace = resolve_workspace()

    print("=" * 60)
    print("PHASE 1: WEEK PREPARATION")
    print("=" * 60)

    # ---- Step 1: Week context ----
    now = datetime.now()
    monday, friday, week_num, year = get_week_bounds(now.date())
    week_label = f"W{week_num:02d}"
    date_range = format_date_range(monday, friday)

    print(f"\n  Week {week_label}: {date_range}")

    context = {
        "weekNumber": week_label,
        "year": year,
        "monday": monday.isoformat(),
        "friday": friday.isoformat(),
        "dateRange": date_range,
    }

    # ---- Step 2: Fetch calendar events ----
    print("\n  Fetching calendar events...")
    raw_events = fetch_week_events(monday, friday)
    print(f"  Found {len(raw_events)} events")

    # ---- Step 3: Classify meetings ----
    print("  Classifying meetings...")
    config = _load_config()
    user_domain = _get_user_domain(config)

    classified: list[dict[str, Any]] = []
    for ev in raw_events:
        classified.append(classify_meeting(ev, user_domain))

    by_day = organize_by_day(classified, monday)

    # Count by type
    type_counts: dict[str, int] = {}
    for ev in classified:
        mt = ev.get("type", "unknown")
        type_counts[mt] = type_counts.get(mt, 0) + 1

    for mt, count in sorted(type_counts.items()):
        print(f"    {mt}: {count}")

    # ---- Step 4: Overdue actions ----
    print("\n  Checking actions...")
    actions = fetch_actions(workspace, monday, friday)
    print(f"    Overdue: {len(actions['overdue'])}")
    print(f"    Due this week: {len(actions['thisWeek'])}")

    # ---- Step 5: Gap analysis ----
    print("\n  Analyzing calendar gaps...")
    gaps_by_day = compute_all_gaps(by_day, monday)
    suggestions = suggest_focus_blocks(gaps_by_day)

    total_gap_minutes = sum(
        g["duration_minutes"] for day_gaps in gaps_by_day.values() for g in day_gaps
    )
    print(f"    Total available focus time: {total_gap_minutes} min")
    print(f"    Focus block suggestions: {len(suggestions)}")

    # ---- Step 6: Build directive ----
    # Strip attendee lists from the by_day output to keep the directive lean.
    # The full attendee data is in classified[] if Claude needs it.
    serializable_by_day: dict[str, list[dict[str, Any]]] = {}
    for day_name, day_events in by_day.items():
        serializable_by_day[day_name] = [
            {
                "id": ev.get("id"),
                "title": ev.get("title"),
                "start": ev.get("start"),
                "end": ev.get("end"),
                "type": ev.get("type"),
                "prep_depth": ev.get("prep_depth"),
                "external_domains": ev.get("external_domains"),
            }
            for ev in day_events
        ]

    directive: dict[str, Any] = {
        "command": "week",
        "generatedAt": now.astimezone(timezone.utc).isoformat(),
        "context": context,
        "meetingsByDay": serializable_by_day,
        "actions": actions,
        "timeBlocks": {
            "gapsByDay": gaps_by_day,
            "suggestions": suggestions,
        },
    }

    # ---- Write output ----
    output_dir = workspace / "_today" / "data"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "week-directive.json"

    output_path.write_text(json.dumps(directive, indent=2, default=str))

    print("\n" + "=" * 60)
    print("PHASE 1 COMPLETE")
    print("=" * 60)
    print(f"\n  Directive: {output_path}")
    print(f"  Events:    {len(raw_events)}")
    print(f"  Overdue:   {len(actions['overdue'])}")
    print(f"  This week: {len(actions['thisWeek'])}")
    print(f"  Focus:     {total_gap_minutes} min available")
    print()

    return 0


if __name__ == "__main__":
    sys.exit(main())
