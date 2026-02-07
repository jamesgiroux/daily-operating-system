"""Calendar fetch + meeting classification for any date range.

Unifies prepare_today.py and prepare_week.py calendar logic per ADR-0030.
Uses the richer 10-rule classification algorithm from prepare_today.py
for all date ranges (today and week).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, date, timedelta, timezone
from typing import Any

from .config import (
    ALL_HANDS_THRESHOLD,
    PERSONAL_EMAIL_DOMAINS,
    build_calendar_service,
    _warn,
)


# ---------------------------------------------------------------------------
# Result types
# ---------------------------------------------------------------------------

@dataclass
class CalendarResult:
    """Result of fetching and classifying calendar events."""
    events: list[dict[str, Any]] = field(default_factory=list)
    classified: list[dict[str, Any]] = field(default_factory=list)
    meetings_by_type: dict[str, list[dict[str, Any]]] = field(default_factory=dict)
    time_status: dict[str, list[str]] = field(default_factory=dict)
    events_by_day: dict[str, list[dict[str, Any]]] = field(default_factory=dict)


# Day names for weekly bucketing
DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]

# All known meeting types
MEETING_TYPES = (
    "customer", "internal", "team_sync", "one_on_one", "partnership",
    "qbr", "training", "external", "all_hands", "personal",
)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def fetch_and_classify(
    start_date: date,
    end_date: date,
    user_domain: str,
    account_hints: set[str] | None = None,
) -> CalendarResult:
    """Fetch events from Google Calendar and classify each meeting.

    Works for any date range: single day (/today) or full week (/week).

    Args:
        start_date: First date to fetch (inclusive).
        end_date: Last date to fetch (inclusive).
        user_domain: User's email domain for internal/external classification.
        account_hints: Lowercased slugs of known customer accounts.

    Returns:
        CalendarResult with events, classified meetings, type buckets,
        time status, and events bucketed by day.
    """
    result = CalendarResult()

    # Fetch raw events
    raw_events = _fetch_events(start_date, end_date)
    result.events = raw_events

    # Classify each event using the full multi-signal algorithm
    classified: list[dict[str, Any]] = []
    for ev in raw_events:
        classified.append(classify_meeting(ev, user_domain))
    result.classified = classified

    # Bucket by type
    meetings_by_type: dict[str, list[dict[str, Any]]] = {t: [] for t in MEETING_TYPES}
    for ev in classified:
        mt = ev.get("type", "unknown")
        if mt in meetings_by_type:
            meetings_by_type[mt].append(ev)
        else:
            meetings_by_type["external"].append(ev)
    result.meetings_by_type = meetings_by_type

    # Time classification (past / in_progress / upcoming)
    now = datetime.now(tz=timezone.utc)
    result.time_status = classify_event_times(raw_events, now)

    # Bucket by day (for week view)
    if (end_date - start_date).days > 0:
        result.events_by_day = _organize_by_day(classified, start_date)

    return result


# ---------------------------------------------------------------------------
# Google Calendar API
# ---------------------------------------------------------------------------

def _fetch_events(
    start_date: date,
    end_date: date,
) -> list[dict[str, Any]]:
    """Fetch calendar events for a date range from Google Calendar.

    Returns a list of normalized event dicts with keys:
        id, summary, start, end, attendees, organizer, description,
        location, is_recurring.
    """
    service = build_calendar_service()
    if service is None:
        return []

    time_min = datetime.combine(start_date, datetime.min.time()).isoformat() + "Z"
    time_max = datetime.combine(
        end_date + timedelta(days=1), datetime.min.time()
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

                events.append({
                    "id": item.get("id", ""),
                    "summary": item.get("summary", "(No title)"),
                    "start": start_raw.get("dateTime", start_raw.get("date", "")),
                    "end": end_raw.get("dateTime", end_raw.get("date", "")),
                    "attendees": [
                        a.get("email", "")
                        for a in item.get("attendees", [])
                    ],
                    "organizer": item.get("organizer", {}).get("email", ""),
                    "description": item.get("description", ""),
                    "location": item.get("location", ""),
                    "is_recurring": bool(item.get("recurringEventId")),
                })

            page_token = result.get("nextPageToken")
            if not page_token:
                break

    except Exception as exc:
        _warn(f"Calendar API error: {exc}")

    return events


# ---------------------------------------------------------------------------
# Meeting classification (per MEETING-TYPES.md)
# ---------------------------------------------------------------------------

def classify_meeting(
    event: dict[str, Any],
    user_domain: str,
) -> dict[str, Any]:
    """Classify a calendar event using the multi-signal algorithm.

    Classification order (first match wins):
        1. personal: no attendees or only organizer
        2. all_hands: 50+ attendees or title keywords
        3. Title overrides: qbr, training, one_on_one
        4. All-internal: one_on_one (2 people), team_sync, internal
        5. External: customer (matched to domain), external (unknown)

    Returns a dict merging classification metadata onto the event.
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
        "organizer": event.get("organizer", ""),
        "is_recurring": event.get("is_recurring", False),
    }

    # ---- Step 1: Personal (no attendees or only organizer) ----
    if attendee_count <= 1:
        result["type"] = "personal"
        return result

    # ---- Step 2: Scale-based override (50+ attendees) ----
    if attendee_count >= ALL_HANDS_THRESHOLD:
        result["type"] = "all_hands"
        return result

    # ---- Step 3: Title keyword overrides ----
    if any(kw in title_lower for kw in ("all hands", "all-hands", "town hall")):
        result["type"] = "all_hands"
        return result

    # Track title overrides that still need domain matching for account
    title_override_type: str | None = None

    if any(kw in title_lower for kw in ("qbr", "business review", "quarterly review")):
        title_override_type = "qbr"
    elif any(kw in title_lower for kw in ("training", "enablement", "workshop")):
        title_override_type = "training"
    elif any(kw in title_lower for kw in ("1:1", "1-1", "one on one", "1-on-1")):
        title_override_type = "one_on_one"

    # ---- Step 4: Domain classification ----
    if user_domain:
        external = [a for a in attendees if "@" in a and not a.lower().endswith(f"@{user_domain}")]
        internal = [a for a in attendees if "@" in a and a.lower().endswith(f"@{user_domain}")]
    else:
        # Without a known domain, treat all as potentially external
        external = attendees
        internal = []

    external_domains = {
        a.split("@")[-1].lower() for a in external if "@" in a
    }

    has_external = len(external) > 0

    # ---- Step 5: All-internal path ----
    if not has_external:
        if title_override_type == "one_on_one" or attendee_count == 2:
            result["type"] = title_override_type or "one_on_one"
            return result

        if title_override_type:
            result["type"] = title_override_type
            return result

        # Team sync signals
        sync_signals = ("sync", "standup", "stand-up", "scrum", "daily", "weekly")
        if any(signal in title_lower for signal in sync_signals) and event.get("is_recurring", False):
            result["type"] = "team_sync"
            return result

        result["type"] = "internal"
        return result

    # ---- Step 6: External path ----
    # Personal email domains only -> personal event
    if external_domains and external_domains.issubset(PERSONAL_EMAIL_DOMAINS):
        result["type"] = "personal"
        return result

    # External attendees present
    result["external_domains"] = sorted(external_domains)

    # Apply title override if set (e.g., QBR with external attendees)
    if title_override_type:
        result["type"] = title_override_type
    elif attendee_count == 2:
        result["type"] = "one_on_one"
    else:
        result["type"] = "customer"

    return result


# ---------------------------------------------------------------------------
# Event time classification (past / in_progress / upcoming)
# ---------------------------------------------------------------------------

def _parse_event_dt(time_str: str) -> datetime | None:
    """Parse an ISO datetime string to a timezone-aware datetime."""
    if not time_str:
        return None
    try:
        if "T" in time_str:
            return datetime.fromisoformat(time_str.replace("Z", "+00:00"))
        # All-day event: treat as midnight UTC
        return datetime.strptime(time_str, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    except (ValueError, TypeError):
        return None


def classify_event_times(
    events: list[dict[str, Any]],
    now: datetime,
) -> dict[str, list[str]]:
    """Bucket event IDs into past, in_progress, and upcoming."""
    result: dict[str, list[str]] = {
        "past": [],
        "in_progress": [],
        "upcoming": [],
    }

    if now.tzinfo is None:
        now = now.replace(tzinfo=timezone.utc)

    for ev in events:
        event_id = ev.get("id", "")
        start_dt = _parse_event_dt(ev.get("start", ""))
        end_dt = _parse_event_dt(ev.get("end", ""))

        if start_dt is None:
            result["upcoming"].append(event_id)
            continue

        if end_dt and now >= end_dt:
            result["past"].append(event_id)
        elif start_dt <= now and (end_dt is None or now < end_dt):
            result["in_progress"].append(event_id)
        else:
            result["upcoming"].append(event_id)

    return result


# ---------------------------------------------------------------------------
# Day bucketing (for week view)
# ---------------------------------------------------------------------------

def _organize_by_day(
    classified: list[dict[str, Any]],
    monday: date,
) -> dict[str, list[dict[str, Any]]]:
    """Bucket classified events into Monday-Friday lists."""
    by_day: dict[str, list[dict[str, Any]]] = {d: [] for d in DAY_NAMES}

    for ev in classified:
        dt = _parse_event_dt(ev.get("start", ""))
        if dt is None:
            continue
        weekday = dt.weekday()
        if 0 <= weekday <= 4:
            by_day[DAY_NAMES[weekday]].append(ev)

    return by_day
