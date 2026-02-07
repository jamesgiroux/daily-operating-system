"""Calendar gap computation and focus block suggestions.

Extracted from prepare_today.py and prepare_week.py per ADR-0030.
"""

from __future__ import annotations

from datetime import datetime, date, timedelta
from typing import Any

from .config import WORK_DAY_START_HOUR, WORK_DAY_END_HOUR, MIN_GAP_MINUTES
from .calendar_fetch import _parse_event_dt, DAY_NAMES


def compute_gaps(
    events: list[dict[str, Any]],
    day_date: date,
) -> list[dict[str, Any]]:
    """Find free time blocks >= MIN_GAP_MINUTES between meetings on a day.

    Operates within work hours (WORK_DAY_START_HOUR to WORK_DAY_END_HOUR).
    """
    day_start = datetime.combine(day_date, datetime.min.time()).replace(
        hour=WORK_DAY_START_HOUR
    )
    day_end = datetime.combine(day_date, datetime.min.time()).replace(
        hour=WORK_DAY_END_HOUR
    )

    # Parse and sort event intervals
    intervals: list[tuple[datetime, datetime]] = []
    for ev in events:
        s = _parse_event_dt(ev.get("start", ""))
        e = _parse_event_dt(ev.get("end", ""))
        if s and e:
            # Strip timezone info for local comparison
            s = s.replace(tzinfo=None)
            e = e.replace(tzinfo=None)
            intervals.append((s, e))
    intervals.sort(key=lambda x: x[0])

    gaps: list[dict[str, Any]] = []
    cursor = day_start

    for start, end in intervals:
        start = max(start, day_start)
        end = min(end, day_end)

        if start > cursor:
            duration = int((start - cursor).total_seconds() / 60)
            if duration >= MIN_GAP_MINUTES:
                gaps.append({
                    "start": cursor.isoformat(),
                    "end": start.isoformat(),
                    "duration_minutes": duration,
                })
        cursor = max(cursor, end)

    # Gap after last meeting
    if cursor < day_end:
        duration = int((day_end - cursor).total_seconds() / 60)
        if duration >= MIN_GAP_MINUTES:
            gaps.append({
                "start": cursor.isoformat(),
                "end": day_end.isoformat(),
                "duration_minutes": duration,
            })

    return gaps


def compute_all_gaps(
    events_by_day: dict[str, list[dict[str, Any]]],
    monday: date,
) -> dict[str, list[dict[str, Any]]]:
    """Compute gaps for each weekday."""
    result: dict[str, list[dict[str, Any]]] = {}
    for i, day_name in enumerate(DAY_NAMES):
        day_date = monday + timedelta(days=i)
        result[day_name] = compute_gaps(events_by_day.get(day_name, []), day_date)
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
            suggestions.append({
                "day": day_name,
                "start": gap["start"],
                "end": gap["end"],
                "duration_minutes": gap["duration_minutes"],
                "suggested_use": block_type,
            })

    return suggestions
