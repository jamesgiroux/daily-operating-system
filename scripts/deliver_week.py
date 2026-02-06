#!/usr/bin/env python3
"""Phase 3: Week Delivery Script for Daybreak.

Reads the week directive (produced by Phase 1) and any Claude-enriched
week files (produced by Phase 2), then writes the final JSON output that
the Tauri frontend consumes.

Output file:
    {workspace}/_today/data/week-overview.json

The Rust backend deserializes this file via ``json_loader::load_week_json``
into the ``WeekOverview`` struct defined in ``types.rs``.  All keys use
camelCase to match the ``serde(rename_all = "camelCase")`` annotation.

Calling convention (matches pty.rs):
    - cwd is set to the workspace root by the Rust executor
    - WORKSPACE env var is also set
    - No positional arguments are passed in production

For manual testing you may pass the workspace path as argv[1]:
    python3 scripts/deliver_week.py /path/to/workspace
"""

from __future__ import annotations

import json
import os
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional


# ---------------------------------------------------------------------------
# Valid enum values (must match Rust types.rs enums exactly)
# ---------------------------------------------------------------------------

VALID_MEETING_TYPES = frozenset({
    "customer", "qbr", "training", "internal", "team_sync",
    "one_on_one", "partnership", "all_hands", "external", "personal",
})

VALID_PREP_STATUSES = frozenset({
    "prep_needed", "agenda_needed", "bring_updates", "context_needed",
    "prep_ready", "draft_ready", "done",
})

VALID_SEVERITIES = frozenset({"critical", "warning", "info"})

# Mapping from emoji prep-status labels used by the old prepare_week.py
# to the snake_case enum values expected by the Rust frontend.
_PREP_STATUS_ALIASES: Dict[str, str] = {
    "prep needed": "prep_needed",
    "agenda needed": "agenda_needed",
    "bring updates": "bring_updates",
    "context needed": "context_needed",
    "prep ready": "prep_ready",
    "draft ready": "draft_ready",
}

# Day names for iterating Mon-Fri
_WEEKDAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]


# ---------------------------------------------------------------------------
# Workspace resolution
# ---------------------------------------------------------------------------

def resolve_workspace() -> Path:
    """Determine the workspace root directory.

    Resolution order:
        1. sys.argv[1] (manual testing)
        2. WORKSPACE environment variable (Rust executor)
        3. Current working directory (fallback)

    Returns:
        Absolute path to the workspace root.
    """
    if len(sys.argv) > 1 and sys.argv[1]:
        return Path(sys.argv[1]).resolve()
    env_ws = os.environ.get("WORKSPACE")
    if env_ws:
        return Path(env_ws).resolve()
    return Path.cwd().resolve()


# ---------------------------------------------------------------------------
# File I/O helpers
# ---------------------------------------------------------------------------

def load_json(path: Path) -> Optional[Dict[str, Any]]:
    """Load a JSON file, returning None on any failure."""
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as exc:
        print(f"Warning: Failed to load {path}: {exc}", file=sys.stderr)
        return None


def write_json(path: Path, data: Dict[str, Any]) -> None:
    """Write *data* as pretty-printed JSON to *path*."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(data, indent=2, default=str, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


# ---------------------------------------------------------------------------
# Normalisation helpers
# ---------------------------------------------------------------------------

def normalise_meeting_type(raw: str) -> str:
    """Normalise a meeting type string to a valid enum value."""
    normalised = raw.lower().replace(" ", "_").replace("-", "_")
    if normalised in VALID_MEETING_TYPES:
        return normalised
    return "internal"


def normalise_prep_status(raw: str) -> str:
    """Normalise a prep-status string to a valid enum value.

    Handles emoji-prefixed labels from the old prepare_week.py as well as
    already-normalised snake_case values.
    """
    # Strip common emoji prefixes
    stripped = raw.strip()
    for char in stripped:
        if char.isalpha():
            break
        stripped = stripped[1:]
    stripped = stripped.strip().lower()

    if stripped in VALID_PREP_STATUSES:
        return stripped

    alias = _PREP_STATUS_ALIASES.get(stripped)
    if alias:
        return alias

    return "prep_needed"


def normalise_severity(raw: str) -> str:
    """Normalise a severity string to a valid enum value.

    Maps the four-level scale used by prepare_week.py (critical, high,
    medium, low) to the three-level AlertSeverity Rust enum (critical,
    warning, info).
    """
    normalised = raw.lower().strip()
    if normalised in VALID_SEVERITIES:
        return normalised
    # Map legacy four-level names to three-level enum
    if normalised == "high":
        return "warning"
    if normalised in ("medium", "low"):
        return "info"
    return "warning"


# ---------------------------------------------------------------------------
# Time display helpers
# ---------------------------------------------------------------------------

def format_time_display(iso_string: str) -> str:
    """Convert an ISO datetime string to a human-readable time like '9:00 AM'."""
    if not iso_string or "T" not in iso_string:
        return ""
    try:
        dt = datetime.fromisoformat(iso_string.replace("Z", "+00:00"))
        # %-I is non-portable (Linux only); use lstrip('0') instead
        return dt.strftime("%I:%M %p").lstrip("0")
    except (ValueError, TypeError):
        return iso_string[:5] if len(iso_string) >= 5 else iso_string


def monday_of_week(dt: datetime) -> datetime:
    """Return the Monday of the ISO week containing *dt*."""
    return dt - timedelta(days=dt.weekday())


# ---------------------------------------------------------------------------
# Core builders
# ---------------------------------------------------------------------------

def build_week_day(
    date: str,
    day_name: str,
    meetings_raw: List[Dict[str, Any]],
) -> Dict[str, Any]:
    """Build a single ``WeekDay`` object for the output JSON.

    Args:
        date: ISO date string (YYYY-MM-DD).
        day_name: e.g. "Monday".
        meetings_raw: Raw meeting dicts from the directive's by_day bucket.

    Returns:
        A dict matching the Rust ``WeekDay`` struct.
    """
    meetings: List[Dict[str, Any]] = []
    for m in meetings_raw:
        meeting_type = normalise_meeting_type(m.get("type", "internal"))

        # Skip personal events (consistent with deliver_today.py)
        if meeting_type == "personal":
            continue

        time_display = m.get("start_display", "")
        if not time_display:
            time_display = format_time_display(m.get("start", ""))

        prep_status = normalise_prep_status(m.get("prep_status", "prep_needed"))

        meetings.append({
            "time": time_display or "TBD",
            "title": m.get("title", m.get("summary", "Meeting")),
            "account": m.get("account") or None,
            "type": meeting_type,
            "prepStatus": prep_status,
        })

    return {
        "date": date,
        "dayName": day_name,
        "meetings": meetings,
    }


def build_action_summary(
    directive: Dict[str, Any],
    data_dir: Optional[Path] = None,
) -> Dict[str, Any]:
    """Build the ``WeekActionSummary`` from directive actions data.

    Falls back to today's ``actions.json`` if the directive has no
    action data (e.g. SQLite was empty when prepare_week ran).
    """
    actions = directive.get("actions", {})
    overdue = actions.get("overdue", [])
    this_week = actions.get("thisWeek", actions.get("this_week", []))

    # Fallback: read from today's actions.json if directive has nothing
    if not overdue and not this_week and data_dir is not None:
        actions_path = data_dir / "actions.json"
        if actions_path.exists():
            try:
                with open(actions_path, encoding="utf-8") as f:
                    today_actions = json.load(f)
                summary = today_actions.get("summary", {})
                all_actions = today_actions.get("actions", [])
                overdue = [a for a in all_actions if a.get("isOverdue")]
                this_week = [a for a in all_actions if not a.get("isOverdue")]
            except (json.JSONDecodeError, OSError):
                pass

    critical_items: List[str] = []
    for task in overdue:
        title = task.get("title", "")
        account = task.get("account", "")
        label = f"{title} - {account}" if account else title
        if label:
            critical_items.append(label)

    return {
        "overdueCount": len(overdue),
        "dueThisWeek": len(this_week),
        "criticalItems": critical_items[:10],
    }


def build_hygiene_alerts(directive: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Build the ``Vec<HygieneAlert>`` from directive hygiene data.

    The old prepare_week.py nests alerts inside per-account objects.
    We flatten them into the simple per-alert structure the frontend expects.
    """
    raw_hygiene = directive.get("hygiene_alerts", [])
    alerts: List[Dict[str, Any]] = []

    for account_block in raw_hygiene:
        account_name = account_block.get("account", "Unknown")
        ring = account_block.get("tier") or account_block.get("ring")
        arr = account_block.get("arr")

        for alert in account_block.get("alerts", []):
            severity = normalise_severity(alert.get("level", "warning"))
            issue = alert.get("message", "Unknown issue")

            alerts.append({
                "account": account_name,
                "ring": ring,
                "arr": str(arr) if arr else None,
                "issue": issue,
                "severity": severity,
            })

    return alerts


def build_time_blocks(directive: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Build the ``Vec<TimeBlock>`` from directive gap/suggestion data."""
    blocks: List[Dict[str, Any]] = []

    # Support both camelCase and snake_case directive formats
    time_blocks_raw = directive.get("timeBlocks", directive.get("time_blocks", {}))

    # Prefer explicit suggestions from prepare_week
    suggestions = time_blocks_raw.get("suggestions", [])
    for s in suggestions:
        day = s.get("day", "")
        start = s.get("start", "")
        end = s.get("end", "")
        duration = s.get("duration_minutes", s.get("duration", 30))

        # Extract HH:MM from ISO datetime if needed
        if "T" in start:
            start = start.split("T")[1][:5]
        if "T" in end:
            end = end.split("T")[1][:5]

        suggested_use = s.get("suggested_use", s.get("block_type", "Focus"))
        task = s.get("task", "")
        if task:
            suggested_use = f"{suggested_use}: {task}"

        if day and start and end:
            blocks.append({
                "day": day,
                "start": start,
                "end": end,
                "durationMinutes": int(duration),
                "suggestedUse": suggested_use,
            })

    # If no suggestions, try to surface raw gaps as "Deep work" blocks
    if not blocks:
        gaps_by_day = time_blocks_raw.get("gapsByDay", time_blocks_raw.get("gaps_by_day", {}))
        for day_name in _WEEKDAY_NAMES:
            for gap in gaps_by_day.get(day_name, []):
                duration = gap.get("duration_minutes", 0)
                if duration < 30:
                    continue
                start = gap.get("start", "")
                end = gap.get("end", "")
                if "T" in start:
                    start = start.split("T")[1][:5]
                if "T" in end:
                    end = end.split("T")[1][:5]
                if start and end:
                    blocks.append({
                        "day": day_name,
                        "start": start,
                        "end": end,
                        "durationMinutes": int(duration),
                        "suggestedUse": "Deep work",
                    })

    return blocks[:10]  # Cap at 10


def build_focus_areas(directive: Dict[str, Any]) -> List[str]:
    """Derive focus area labels from the directive contents."""
    areas: List[str] = []

    # Count customer meetings across all days
    meetings_by_day = directive.get("meetingsByDay",
                        directive.get("meetings", {}).get("by_day", {}))
    customer_count = sum(
        1 for day_meetings in meetings_by_day.values()
        for m in day_meetings
        if m.get("type") == "customer"
    )
    if customer_count:
        areas.append(f"Customer meetings ({customer_count})")

    actions = directive.get("actions", {})
    overdue = actions.get("overdue", [])
    if overdue:
        areas.append(f"Overdue items ({len(overdue)})")

    hygiene = directive.get("hygiene_alerts", directive.get("hygieneAlerts", []))
    critical_count = sum(
        1 for a in hygiene
        if any(
            alert.get("level") in ("critical", "high")
            for alert in a.get("alerts", [])
        )
    )
    if critical_count:
        areas.append(f"Hygiene alerts ({critical_count} critical)")

    this_week = actions.get("thisWeek", actions.get("this_week", []))
    if this_week:
        areas.append(f"Due this week ({len(this_week)})")

    prev_week = directive.get("previous_week", {})
    if prev_week.get("status") == "draft":
        areas.append("Finalize last week's impact")

    return areas if areas else ["Review weekly overview"]


# ---------------------------------------------------------------------------
# Main builder
# ---------------------------------------------------------------------------

def build_week_overview(
    directive: Dict[str, Any],
    data_dir: Optional[Path] = None,
) -> Dict[str, Any]:
    """Build the complete ``WeekOverview`` JSON document.

    Args:
        directive: The raw week-directive.json contents.
        data_dir: Path to ``_today/data/`` for fallback reads.

    Returns:
        A dict matching the Rust ``WeekOverview`` struct (camelCase keys).
    """
    context = directive.get("context", {})

    # Support both camelCase (new prepare_week.py) and snake_case (legacy)
    week_number_raw = context.get("weekNumber", context.get("week_number", 0))
    if isinstance(week_number_raw, str) and week_number_raw.startswith("W"):
        week_number = week_number_raw
    else:
        week_number = f"W{int(week_number_raw):02d}"

    date_range = context.get("dateRange", context.get("date_range_display", ""))
    monday_str = context.get("monday", "")

    # Build days array — support both key formats
    days: List[Dict[str, Any]] = []
    meetings_by_day = directive.get("meetingsByDay",
                        directive.get("meetings", {}).get("by_day", {}))

    for i, day_name in enumerate(_WEEKDAY_NAMES):
        if monday_str:
            try:
                monday_dt = datetime.strptime(monday_str, "%Y-%m-%d")
                day_dt = monday_dt + timedelta(days=i)
                day_date = day_dt.strftime("%Y-%m-%d")
            except ValueError:
                day_date = ""
        else:
            day_date = ""

        day_meetings = meetings_by_day.get(day_name, [])
        days.append(build_week_day(day_date, day_name, day_meetings))

    overview: Dict[str, Any] = {
        "weekNumber": week_number,
        "dateRange": date_range,
        "days": days,
        "actionSummary": build_action_summary(directive, data_dir=data_dir),
        "hygieneAlerts": build_hygiene_alerts(directive),
        "focusAreas": build_focus_areas(directive),
        "availableTimeBlocks": build_time_blocks(directive),
    }

    return overview


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> int:
    """Read the week directive and write week-overview.json."""
    workspace = resolve_workspace()
    today_dir = workspace / "_today"
    data_dir = today_dir / "data"

    # Load the week directive
    directive_path = data_dir / "week-directive.json"
    directive = load_json(directive_path)

    if directive is None:
        # Fall back to the legacy location used by the old prepare_week.py
        legacy_path = today_dir / ".week-directive.json"
        directive = load_json(legacy_path)

    if directive is None:
        print(
            "Error: week directive not found. "
            "Checked:\n"
            f"  {data_dir / 'week-directive.json'}\n"
            f"  {today_dir / '.week-directive.json'}",
            file=sys.stderr,
        )
        return 1

    # Build the output
    overview = build_week_overview(directive, data_dir=data_dir)

    # Write the output
    output_path = data_dir / "week-overview.json"
    write_json(output_path, overview)

    print(f"Wrote {output_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
