#!/usr/bin/env python3
"""Phase 3: Today Delivery Script for Daybreak.

Reads the today directive (produced by Phase 1) and any Claude-enriched
fields (added by Phase 2), then writes the final JSON output files that
the Tauri frontend consumes.

Output files:
    {workspace}/_today/data/schedule.json
    {workspace}/_today/data/actions.json
    {workspace}/_today/data/emails.json
    {workspace}/_today/data/manifest.json
    {workspace}/_today/data/preps/*.json

The Rust backend deserializes these files via ``json_loader.rs``.  All
output keys use camelCase to match the ``serde(rename_all = "camelCase")``
annotations in ``types.rs``.

Calling convention (matches pty.rs):
    - cwd is set to the workspace root by the Rust executor
    - WORKSPACE env var is also set
    - No positional arguments are passed in production

For manual testing you may pass the workspace path as argv[1]:
    python3 scripts/deliver_today.py /path/to/workspace
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# ---------------------------------------------------------------------------
# Valid enum values (must match Rust types.rs enums exactly)
# ---------------------------------------------------------------------------

VALID_MEETING_TYPES = frozenset({
    "customer", "qbr", "training", "internal", "team_sync",
    "one_on_one", "partnership", "all_hands", "external", "personal",
})

# Meeting types that receive prep files with hasPrep = true
PREP_ELIGIBLE_TYPES = frozenset({"customer", "qbr", "partnership"})


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


def write_json(path: Path, data: Any) -> None:
    """Write *data* as pretty-printed JSON to *path*."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(data, indent=2, default=str, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def extract_focus_from_markdown(today_dir: Path) -> Optional[str]:
    """Extract the primary focus from ``81-suggested-focus.md``.

    Uses a multi-strategy approach to handle varying Phase 2 output
    formats:

        1. Bold checkbox item: ``- [ ] **Do the thing**``
        2. Plain checkbox item: ``- [ ] Do the thing (13 days overdue)``
        3. Priority section header: ``## Priority 1: The Focus Area``

    Args:
        today_dir: Path to the ``_today`` directory.

    Returns:
        A short focus string, or None if the file doesn't exist or
        no actionable item is found.
    """
    focus_path = today_dir / "81-suggested-focus.md"
    if not focus_path.exists():
        return None
    try:
        content = focus_path.read_text(encoding="utf-8")
        lines = content.splitlines()

        # Strategy 1: First bold item in a checkbox
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("- [ ] **"):
                match = re.search(r"\*\*(.+?)\*\*", stripped)
                if match:
                    return match.group(1)

        # Strategy 2: First unchecked checkbox item (plain text)
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("- [ ] "):
                text = stripped[6:].strip()
                # Clean common prefixes like "Address:"
                text = re.sub(r"^Address:\s*", "", text)
                # Strip trailing overdue metadata
                text = re.sub(r"\s*\(\d+ days? overdue\)$", "", text)
                if text:
                    return text

        # Strategy 3: First priority section header
        for line in lines:
            stripped = line.strip()
            match = re.match(r"^## Priority \d+:\s*(.+)$", stripped)
            if match:
                return match.group(1).strip()

        return None
    except OSError:
        return None


def _parse_sender(from_raw: str) -> Tuple[str, str]:
    """Parse a raw email From header into (display_name, email_address).

    Handles formats like ``"Name <email@example.com>"``,
    ``"email@example.com"``, and ``"<email@example.com>"``.

    Args:
        from_raw: Raw From header value.

    Returns:
        Tuple of (sender_name, sender_email).
    """
    if "<" in from_raw and ">" in from_raw:
        name = from_raw.split("<")[0].strip().strip('"').strip("'")
        email = from_raw.split("<")[1].split(">")[0].strip()
        return (name or email, email)
    return (from_raw.strip(), from_raw.strip())


# ---------------------------------------------------------------------------
# Normalisation helpers
# ---------------------------------------------------------------------------

def normalise_meeting_type(raw: str) -> str:
    """Normalise a meeting type string to a valid enum value.

    Args:
        raw: Raw meeting type from the directive.

    Returns:
        A valid meeting type string from VALID_MEETING_TYPES, defaulting
        to ``"internal"`` for unrecognised values.
    """
    normalised = raw.lower().replace(" ", "_").replace("-", "_")
    if normalised in VALID_MEETING_TYPES:
        return normalised
    return "internal"


# ---------------------------------------------------------------------------
# Time helpers
# ---------------------------------------------------------------------------

def format_time_display(iso_string: str) -> str:
    """Convert an ISO datetime string to a human-readable time like '9:00 AM'.

    Returns ``"All day"`` for date-only strings and falls back to the raw
    prefix for unparseable values.
    """
    if not iso_string or "T" not in iso_string:
        return "All day"
    try:
        dt = datetime.fromisoformat(iso_string.replace("Z", "+00:00"))
        # %-I is non-portable (Linux only); use lstrip('0') instead
        return dt.strftime("%I:%M %p").lstrip("0")
    except (ValueError, TypeError):
        return iso_string[:5] if len(iso_string) >= 5 else iso_string


def parse_iso_dt(iso_string: str) -> Optional[datetime]:
    """Parse an ISO datetime string into a tz-aware datetime, or None."""
    if not iso_string or "T" not in iso_string:
        return None
    try:
        return datetime.fromisoformat(iso_string.replace("Z", "+00:00"))
    except (ValueError, TypeError):
        return None


def greeting_for_hour(hour: int) -> str:
    """Return a time-appropriate greeting.

    Args:
        hour: Hour of the day (0-23).

    Returns:
        Greeting string.
    """
    if hour < 12:
        return "Good morning"
    if hour < 17:
        return "Good afternoon"
    return "Good evening"


# ---------------------------------------------------------------------------
# Meeting ID generation
# ---------------------------------------------------------------------------

def make_meeting_id(event: Dict[str, Any], meeting_type: str) -> str:
    """Generate a stable meeting ID from a calendar event.

    Format: ``HHMM-type-slug`` (e.g. ``"0900-customer-acme-sync"``).
    Falls back to a short hash prefix if time parsing fails.

    Args:
        event: Calendar event dict with ``summary`` and ``start`` keys.
        meeting_type: Normalised meeting type string.

    Returns:
        A URL-safe meeting ID string.
    """
    title = event.get("summary", "untitled")
    slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")[:40]

    start = event.get("start", "")
    time_prefix = ""
    if "T" in start:
        try:
            dt = datetime.fromisoformat(start.replace("Z", "+00:00"))
            time_prefix = dt.strftime("%H%M")
        except (ValueError, TypeError):
            pass

    if not time_prefix:
        raw = f"{start}-{title}"
        time_prefix = hashlib.md5(raw.encode()).hexdigest()[:6]

    return f"{time_prefix}-{meeting_type}-{slug}"


# ---------------------------------------------------------------------------
# Directive event/meeting cross-referencing
# ---------------------------------------------------------------------------

def classify_event(
    event: Dict[str, Any],
    meetings_by_type: Dict[str, List[Dict[str, Any]]],
) -> str:
    """Look up the meeting type for a calendar event by matching its id
    against the classified meetings dict from the directive.

    Args:
        event: Calendar event dict with an ``id`` key.
        meetings_by_type: The directive ``meetings`` dict keyed by type.

    Returns:
        Normalised meeting type string.
    """
    event_id = event.get("id")
    for mtype, meeting_list in meetings_by_type.items():
        for m in meeting_list:
            mid = m.get("event_id") or m.get("id")
            if mid == event_id:
                return normalise_meeting_type(mtype)
    return "internal"


def find_meeting_entry(
    event: Dict[str, Any],
    meetings_by_type: Dict[str, List[Dict[str, Any]]],
) -> Tuple[Optional[Dict[str, Any]], Optional[str]]:
    """Find the directive meeting entry and its account for a calendar event.

    Returns:
        Tuple of (meeting_entry, account) or (None, None).
    """
    event_id = event.get("id")
    for _mtype, meeting_list in meetings_by_type.items():
        for m in meeting_list:
            mid = m.get("event_id") or m.get("id")
            if mid == event_id:
                return m, m.get("account")
    return None, None


def find_meeting_context(
    account: Optional[str],
    event_id: Optional[str],
    meeting_contexts: List[Dict[str, Any]],
) -> Optional[Dict[str, Any]]:
    """Find the meeting context block matching an account or event_id.

    Tries account match first (CS profile), then event_id match (general
    profile), since general profile contexts may not have an account field.

    Args:
        account: Account name (may be None).
        event_id: Calendar event ID (may be None).
        meeting_contexts: List of context dicts from directive.

    Returns:
        Matching context dict, or None.
    """
    if account:
        for ctx in meeting_contexts:
            if ctx.get("account") == account:
                return ctx
    if event_id:
        for ctx in meeting_contexts:
            if ctx.get("event_id") == event_id:
                return ctx
    return None


def is_meeting_current(
    event: Dict[str, Any],
    now: datetime,
) -> bool:
    """Check whether a calendar event is currently in progress.

    Args:
        event: Calendar event dict with ``start`` and ``end`` ISO strings.
        now: Current datetime (should be tz-aware).

    Returns:
        True if *now* falls within the event's start/end window.
    """
    start_dt = parse_iso_dt(event.get("start", ""))
    end_dt = parse_iso_dt(event.get("end", ""))
    if start_dt is None or end_dt is None:
        return False
    # Ensure comparison is tz-aware
    if start_dt.tzinfo is None:
        start_dt = start_dt.replace(tzinfo=timezone.utc)
    if end_dt.tzinfo is None:
        end_dt = end_dt.replace(tzinfo=timezone.utc)
    if now.tzinfo is None:
        now = now.replace(tzinfo=timezone.utc)
    return start_dt <= now <= end_dt


# ---------------------------------------------------------------------------
# Prep summary builder (inline in schedule.json)
# ---------------------------------------------------------------------------

def build_prep_summary(
    meeting_context: Optional[Dict[str, Any]],
) -> Optional[Dict[str, Any]]:
    """Build a condensed prep summary for embedding in schedule.json.

    Args:
        meeting_context: Context block from the directive.

    Returns:
        A dict with ``atAGlance``, ``discuss``, ``watch``, ``wins`` keys,
        or None if no meaningful data is available.
    """
    if not meeting_context:
        return None

    account_data = meeting_context.get("account_data", {})
    at_a_glance: List[str] = []

    for key, label in [
        ("ring", "Ring"),
        ("arr", "ARR"),
        ("renewal", "Renewal"),
        ("health", "Health"),
    ]:
        val = account_data.get(key)
        if val:
            at_a_glance.append(f"{label}: {val}")

    # AI-enriched fields (Phase 2 may populate these)
    discuss = meeting_context.get("talking_points", [])[:4]
    watch = meeting_context.get("risks", [])[:3]
    wins = meeting_context.get("wins", [])[:3]

    if not at_a_glance and not discuss and not watch and not wins:
        return None

    return {
        "atAGlance": at_a_glance[:4],
        "discuss": discuss,
        "watch": watch,
        "wins": wins,
    }


# ---------------------------------------------------------------------------
# Core builders
# ---------------------------------------------------------------------------

def build_schedule(
    directive: Dict[str, Any],
    now: datetime,
) -> Dict[str, Any]:
    """Build the ``schedule.json`` payload from directive data.

    Conforms to the Rust ``JsonSchedule`` struct in ``json_loader.rs``.

    Args:
        directive: The full directive dictionary.
        now: Current datetime for isCurrent calculation.

    Returns:
        Dictionary ready to serialize as ``schedule.json``.
    """
    context = directive.get("context", {})
    date = context.get("date", now.strftime("%Y-%m-%d"))
    events = directive.get("calendar", {}).get("events", [])
    meetings_by_type = directive.get("meetings", {})
    meeting_contexts = directive.get("meeting_contexts", [])

    meetings_json: List[Dict[str, Any]] = []

    for event in events:
        meeting_type = classify_event(event, meetings_by_type)

        # Skip personal events -- dashboard does not show them
        if meeting_type == "personal":
            continue

        meeting_entry, account = find_meeting_entry(event, meetings_by_type)
        meeting_id = make_meeting_id(event, meeting_type)

        start = event.get("start", "")
        end = event.get("end", "")

        # Look up meeting context for prep data
        mc = find_meeting_context(account, event.get("id"), meeting_contexts)
        prep_summary = build_prep_summary(mc) if mc else None

        has_prep = meeting_type in PREP_ELIGIBLE_TYPES and mc is not None
        prep_file = f"preps/{meeting_id}.json" if has_prep else None

        meeting_obj: Dict[str, Any] = {
            "id": meeting_id,
            "time": format_time_display(start),
            "title": event.get("summary", "No title"),
            "type": meeting_type,
            "hasPrep": has_prep,
            "isCurrent": is_meeting_current(event, now),
        }

        if end:
            meeting_obj["endTime"] = format_time_display(end)
        if account:
            meeting_obj["account"] = account
        if prep_file:
            meeting_obj["prepFile"] = prep_file
        if prep_summary:
            meeting_obj["prepSummary"] = prep_summary

        meetings_json.append(meeting_obj)

    # Build top-level schedule object
    schedule: Dict[str, Any] = {
        "date": date,
        "meetings": meetings_json,
    }

    # AI-enriched overview fields (from Phase 2, or prepare_today context)
    schedule["greeting"] = (
        context.get("greeting")
        or greeting_for_hour(now.hour)
    )
    if context.get("summary"):
        schedule["summary"] = context["summary"]
    else:
        # Generate a sensible default summary
        total = len(meetings_json)
        customer_count = sum(
            1 for m in meetings_json if m["type"] in ("customer", "qbr")
        )
        parts = [f"{total} meeting{'s' if total != 1 else ''} today"]
        if customer_count:
            parts.append(
                f"{customer_count} customer call{'s' if customer_count != 1 else ''}"
            )
        schedule["summary"] = " with ".join(parts)

    if context.get("focus"):
        schedule["focus"] = context["focus"]

    return schedule


def build_actions(
    directive: Dict[str, Any],
    now: datetime,
) -> Dict[str, Any]:
    """Build the ``actions.json`` payload from directive data.

    Flattens overdue / due_today / due_this_week / waiting_on groups into
    a single list ordered by urgency.  Conforms to ``JsonActions`` in
    ``json_loader.rs``.

    Args:
        directive: The full directive dictionary.
        now: Current datetime (used for date field fallback).

    Returns:
        Dictionary ready to serialize as ``actions.json``.
    """
    context = directive.get("context", {})
    date = context.get("date", now.strftime("%Y-%m-%d"))
    raw_actions = directive.get("actions", {})

    overdue = raw_actions.get("overdue", [])
    due_today = raw_actions.get("due_today", [])
    due_this_week = raw_actions.get("due_this_week", [])
    waiting_on = raw_actions.get("waiting_on", [])

    actions_list: List[Dict[str, Any]] = []
    seen_ids: set = set()

    def _make_id(prefix: str, index: int, title: str) -> str:
        slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")[:30]
        aid = f"{prefix}-{index:03d}-{slug}"
        if aid in seen_ids:
            aid = f"{aid}-dup{index}"
        seen_ids.add(aid)
        return aid

    # Overdue -> P1, pending, isOverdue = true
    for i, task in enumerate(overdue):
        actions_list.append({
            "id": _make_id("overdue", i, task.get("title", "")),
            "title": task.get("title", "Unknown"),
            "account": task.get("account"),
            "priority": "P1",
            "status": "pending",
            "dueDate": task.get("due_date") or task.get("due"),
            "isOverdue": True,
            "daysOverdue": task.get("days_overdue", 0),
            "context": task.get("context"),
            "source": task.get("source"),
        })

    # Due today -> P1, pending
    for i, task in enumerate(due_today):
        actions_list.append({
            "id": _make_id("today", i, task.get("title", "")),
            "title": task.get("title", "Unknown"),
            "account": task.get("account"),
            "priority": "P1",
            "status": "pending",
            "dueDate": task.get("due_date") or task.get("due"),
            "isOverdue": False,
            "context": task.get("context"),
            "source": task.get("source"),
        })

    # Due this week -> P2, pending
    for i, task in enumerate(due_this_week):
        actions_list.append({
            "id": _make_id("week", i, task.get("title", "")),
            "title": task.get("title", "Unknown"),
            "account": task.get("account"),
            "priority": "P2",
            "status": "pending",
            "dueDate": task.get("due_date") or task.get("due"),
            "isOverdue": False,
            "context": task.get("context"),
            "source": task.get("source"),
        })

    # Waiting on -> P2, waiting
    for i, item in enumerate(waiting_on):
        actions_list.append({
            "id": _make_id("waiting", i, item.get("what", "")),
            "title": f"Waiting: {item.get('what', 'Unknown')}",
            "account": item.get("who"),
            "priority": "P2",
            "status": "waiting",
            "context": item.get("context"),
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


def build_emails(
    directive: Dict[str, Any],
    now: datetime,
) -> Dict[str, Any]:
    """Build the ``emails.json`` payload from directive data.

    Conforms to ``JsonEmails`` in ``json_loader.rs``.

    Supports two directive formats:
        - **New**: ``emails.classified`` contains all emails with a
          ``priority`` field (``"high"``, ``"medium"``, ``"low"``).
        - **Legacy**: ``emails.high_priority`` contains only high-priority
          email objects; medium/low are stored as counts only.

    Args:
        directive: The full directive dictionary.
        now: Current datetime (used for date field fallback).

    Returns:
        Dictionary ready to serialize as ``emails.json``.
    """
    context = directive.get("context", {})
    date = context.get("date", now.strftime("%Y-%m-%d"))
    raw_emails = directive.get("emails", {})

    # Prefer 'classified' (all emails with priority), fall back to 'high_priority'
    classified = raw_emails.get("classified", [])
    high_priority_only = raw_emails.get("high_priority", [])

    source = classified if classified else high_priority_only

    emails_list: List[Dict[str, Any]] = []
    for i, email in enumerate(source):
        eid = email.get("id", f"email-{i:03d}")
        from_raw = email.get("from", "")

        # Parse sender name and email from raw From header
        from_email = email.get("from_email", "")
        sender = email.get("from", "Unknown")
        if from_raw:
            parsed_name, parsed_email = _parse_sender(from_raw)
            sender = parsed_name
            if not from_email:
                from_email = parsed_email

        # Normalise priority: high stays high, everything else → normal
        raw_priority = email.get("priority", "high" if not classified else "normal")
        priority = "high" if raw_priority == "high" else "normal"

        emails_list.append({
            "id": eid,
            "sender": sender,
            "senderEmail": from_email,
            "subject": email.get("subject", "No subject"),
            "snippet": email.get("snippet"),
            "priority": priority,
        })

    # Compute stats
    high_count = sum(1 for e in emails_list if e["priority"] == "high")

    if classified:
        normal_count = len(emails_list) - high_count
    else:
        normal_count = raw_emails.get("medium_count", 0) + raw_emails.get("low_count", 0)

    needs_action = sum(
        1 for e in source
        if e.get("priority") == "high"
        and e.get("action_owner", "").lower() in ("you", "me", "")
    )

    return {
        "date": date,
        "stats": {
            "highPriority": high_count,
            "normalPriority": normal_count,
            "needsAction": needs_action,
        },
        "emails": emails_list,
    }


def build_prep(
    meeting: Dict[str, Any],
    meeting_type: str,
    meeting_id: str,
    meeting_context: Optional[Dict[str, Any]],
) -> Dict[str, Any]:
    """Build an individual meeting prep JSON document.

    Conforms to ``JsonPrep`` in ``json_loader.rs``.

    Args:
        meeting: Meeting dict from directive ``meetings`` section.
        meeting_type: Normalised meeting type string.
        meeting_id: The stable meeting ID.
        meeting_context: Optional context block from directive.

    Returns:
        Dictionary ready to serialize as ``preps/{meeting_id}.json``.
    """
    account = meeting.get("account")
    account_data: Dict[str, Any] = {}
    attendees_raw: List[Dict[str, Any]] = []

    if meeting_context:
        account_data = meeting_context.get("account_data", {})
        attendees_raw = meeting_context.get("attendees", [])

    # Quick context from account data
    # Use display-friendly labels (ARR not Arr, CSM not Csm)
    _DISPLAY_LABELS: Dict[str, str] = {
        "ring": "Ring",
        "arr": "ARR",
        "renewal": "Renewal",
        "health": "Health",
        "tier": "Tier",
        "csm": "CSM",
        "stage": "Stage",
    }
    quick_context: Dict[str, str] = {}
    for key in ("ring", "arr", "renewal", "health", "tier", "csm", "stage"):
        val = account_data.get(key)
        if val:
            quick_context[_DISPLAY_LABELS.get(key, key.title())] = str(val)

    # Attendees
    attendees: List[Dict[str, Any]] = []
    for att in attendees_raw:
        entry: Dict[str, Any] = {
            "name": att.get("name", att.get("email", "Unknown")),
        }
        if att.get("role"):
            entry["role"] = att["role"]
        if att.get("focus"):
            entry["focus"] = att["focus"]
        attendees.append(entry)

    # Time range
    start_display = meeting.get("start_display", "")
    end_display = meeting.get("end_display", "")
    time_range = (
        f"{start_display} - {end_display}"
        if start_display and end_display
        else start_display
    )

    prep: Dict[str, Any] = {
        "meetingId": meeting_id,
        "title": meeting.get("title", meeting.get("summary", "Meeting")),
        "type": meeting_type,
    }

    if time_range:
        prep["timeRange"] = time_range
    if account:
        prep["account"] = account
    if meeting_context and meeting_context.get("narrative"):
        prep["meetingContext"] = meeting_context["narrative"]
    if quick_context:
        prep["quickContext"] = quick_context
    if attendees:
        prep["attendees"] = attendees

    # Merge extra context fields using snake_case -> camelCase conversion
    if meeting_context:
        for field in (
            "since_last", "current_state", "risks",
            "talking_points", "questions", "key_principles",
        ):
            camel = re.sub(r"_([a-z])", lambda m: m.group(1).upper(), field)
            val = meeting_context.get(field)
            if val:
                prep[camel] = val

        # Strategic programs -> array of {name, status} objects
        programs = meeting_context.get("strategic_programs")
        if programs:
            prep["strategicPrograms"] = [
                (
                    {"name": p.get("name", str(p)), "status": p.get("status", "in_progress")}
                    if isinstance(p, dict)
                    else {"name": str(p), "status": "in_progress"}
                )
                for p in programs
            ]

        # Open items -> array of {title, dueDate, context, isOverdue}
        open_items = meeting_context.get("open_items")
        if open_items:
            prep["openItems"] = [
                {
                    "title": (
                        item.get("title", str(item))
                        if isinstance(item, dict) else str(item)
                    ),
                    "dueDate": item.get("due_date") if isinstance(item, dict) else None,
                    "context": item.get("context") if isinstance(item, dict) else None,
                    "isOverdue": (
                        item.get("is_overdue", False)
                        if isinstance(item, dict) else False
                    ),
                }
                for item in open_items
            ]

        # References -> array of {label, path, lastUpdated}
        references = meeting_context.get("references")
        if references:
            prep["references"] = [
                {
                    "label": (
                        ref.get("label", str(ref))
                        if isinstance(ref, dict) else str(ref)
                    ),
                    "path": ref.get("path") if isinstance(ref, dict) else None,
                    "lastUpdated": (
                        ref.get("last_updated")
                        if isinstance(ref, dict) else None
                    ),
                }
                for ref in references
            ]

    return prep


def build_manifest(
    date: str,
    schedule_data: Dict[str, Any],
    actions_data: Dict[str, Any],
    emails_data: Dict[str, Any],
    prep_paths: List[str],
    profile: Optional[str],
) -> Dict[str, Any]:
    """Build the ``manifest.json`` payload summarising all output files.

    Conforms to the Rust ``Manifest`` struct in ``json_loader.rs``.

    Args:
        date: ISO date string for today.
        schedule_data: Already-built schedule payload.
        actions_data: Already-built actions payload.
        emails_data: Already-built emails payload.
        prep_paths: Relative prep file paths (e.g. ``preps/0900-customer-slug.json``).
        profile: User profile name (may be None).

    Returns:
        Dictionary ready to serialize as ``manifest.json``.
    """
    meetings = schedule_data.get("meetings", [])
    total_meetings = len(meetings)
    customer_count = sum(
        1 for m in meetings if m.get("type") in ("customer", "qbr")
    )
    internal_count = sum(
        1 for m in meetings
        if m.get("type") in ("internal", "team_sync", "one_on_one", "all_hands")
    )

    actions_summary = actions_data.get("summary", {})
    actions_due = actions_summary.get("dueToday", 0)
    actions_overdue = actions_summary.get("overdue", 0)
    emails_flagged = emails_data.get("stats", {}).get("highPriority", 0)

    manifest: Dict[str, Any] = {
        "schemaVersion": "1.0.0",
        "date": date,
        "generatedAt": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "partial": False,
        "files": {
            "schedule": "schedule.json",
            "actions": "actions.json",
            "emails": "emails.json",
            "preps": prep_paths,
        },
        "stats": {
            "totalMeetings": total_meetings,
            "customerMeetings": customer_count,
            "internalMeetings": internal_count,
            "actionsDue": actions_due,
            "actionsOverdue": actions_overdue,
            "emailsFlagged": emails_flagged,
        },
    }

    if profile:
        manifest["profile"] = profile

    return manifest


# ---------------------------------------------------------------------------
# Prep file orchestration
# ---------------------------------------------------------------------------

def build_all_preps(
    directive: Dict[str, Any],
    events: List[Dict[str, Any]],
) -> List[Tuple[str, Dict[str, Any]]]:
    """Build prep JSON for every meeting that has context data.

    Returns a list of ``(relative_path, prep_data)`` tuples where
    ``relative_path`` is like ``"preps/0900-customer-slug.json"``.
    """
    meetings_by_type = directive.get("meetings", {})
    meeting_contexts = directive.get("meeting_contexts", [])
    results: List[Tuple[str, Dict[str, Any]]] = []

    for mtype, meeting_list in meetings_by_type.items():
        normalised_type = normalise_meeting_type(mtype)

        # Skip personal meetings -- no prep files generated
        if normalised_type == "personal":
            continue

        for meeting in meeting_list:
            account = meeting.get("account")
            event_id = meeting.get("event_id") or meeting.get("id")
            mc = find_meeting_context(account, event_id, meeting_contexts)

            # Only write a prep file if there is meaningful context
            if not mc and not account:
                continue

            # Resolve matching calendar event for stable ID generation
            matched_event: Optional[Dict[str, Any]] = None
            for ev in events:
                if ev.get("id") == event_id:
                    matched_event = ev
                    break

            if matched_event:
                meeting_id = make_meeting_id(matched_event, normalised_type)
            else:
                # Synthesise an ID from meeting fields directly
                title = meeting.get(
                    "title",
                    meeting.get("summary", account or "meeting"),
                )
                slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")[:40]
                start = meeting.get(
                    "start_display",
                    meeting.get("start", ""),
                )
                time_part = re.sub(r"[^0-9]", "", start)[:4] if start else "0000"
                meeting_id = f"{time_part}-{normalised_type}-{slug}"

            prep_data = build_prep(meeting, normalised_type, meeting_id, mc)
            rel_path = f"preps/{meeting_id}.json"
            results.append((rel_path, prep_data))

    return results


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> int:
    """Read the today directive and write all JSON output files."""
    workspace = resolve_workspace()
    today_dir = workspace / "_today"
    data_dir = today_dir / "data"
    preps_dir = data_dir / "preps"

    print("deliver_today: starting", file=sys.stderr)

    # ------------------------------------------------------------------
    # 1. Load directive (data/ location first, then legacy hidden file)
    # ------------------------------------------------------------------
    directive: Optional[Dict[str, Any]] = None

    primary_path = data_dir / "today-directive.json"
    legacy_path = today_dir / ".today-directive.json"

    directive = load_json(primary_path)
    if directive is not None:
        print(f"deliver_today: loaded {primary_path}", file=sys.stderr)
    else:
        directive = load_json(legacy_path)
        if directive is not None:
            print(f"deliver_today: loaded {legacy_path} (legacy)", file=sys.stderr)

    if directive is None:
        print(
            "Error: today directive not found. "
            "Checked:\n"
            f"  {primary_path}\n"
            f"  {legacy_path}",
            file=sys.stderr,
        )
        return 1

    # ------------------------------------------------------------------
    # 2. Ensure output directories exist
    # ------------------------------------------------------------------
    data_dir.mkdir(parents=True, exist_ok=True)
    preps_dir.mkdir(parents=True, exist_ok=True)

    # Clear stale prep files before writing fresh ones
    for old_prep in preps_dir.glob("*.json"):
        old_prep.unlink()
        print(f"deliver_today: removed stale prep {old_prep.name}", file=sys.stderr)

    now = datetime.now(timezone.utc)
    context = directive.get("context", {})
    date = context.get("date", now.strftime("%Y-%m-%d"))
    profile = context.get("profile")
    events = directive.get("calendar", {}).get("events", [])

    # Inject focus from Phase 2 markdown if not already in directive
    if not context.get("focus"):
        focus = extract_focus_from_markdown(today_dir)
        if focus:
            context["focus"] = focus
            print(f"deliver_today: extracted focus from markdown: {focus}", file=sys.stderr)

    # ------------------------------------------------------------------
    # 3. Build schedule.json
    # ------------------------------------------------------------------
    print("deliver_today: building schedule.json", file=sys.stderr)
    schedule_data = build_schedule(directive, now)
    write_json(data_dir / "schedule.json", schedule_data)

    # ------------------------------------------------------------------
    # 4. Build actions.json
    # ------------------------------------------------------------------
    print("deliver_today: building actions.json", file=sys.stderr)
    actions_data = build_actions(directive, now)
    write_json(data_dir / "actions.json", actions_data)

    # ------------------------------------------------------------------
    # 5. Build emails.json
    # ------------------------------------------------------------------
    print("deliver_today: building emails.json", file=sys.stderr)
    emails_data = build_emails(directive, now)
    write_json(data_dir / "emails.json", emails_data)

    # ------------------------------------------------------------------
    # 6. Build preps/*.json
    # ------------------------------------------------------------------
    print("deliver_today: building prep files", file=sys.stderr)
    preps = build_all_preps(directive, events)
    prep_manifest_paths: List[str] = []

    for rel_path, prep_data in preps:
        write_json(data_dir / rel_path, prep_data)
        prep_manifest_paths.append(rel_path)
        print(f"deliver_today:   {rel_path}", file=sys.stderr)

    if not preps:
        print("deliver_today:   (no prep files to write)", file=sys.stderr)

    # ------------------------------------------------------------------
    # 7. Build manifest.json
    # ------------------------------------------------------------------
    print("deliver_today: building manifest.json", file=sys.stderr)
    manifest_data = build_manifest(
        date=date,
        schedule_data=schedule_data,
        actions_data=actions_data,
        emails_data=emails_data,
        prep_paths=prep_manifest_paths,
        profile=profile,
    )
    write_json(data_dir / "manifest.json", manifest_data)

    # ------------------------------------------------------------------
    # Summary
    # ------------------------------------------------------------------
    total_files = 4 + len(preps)  # schedule + actions + emails + manifest + preps
    stats = manifest_data.get("stats", {})

    print(
        f"deliver_today: complete - "
        f"{total_files} files written, "
        f"{stats.get('totalMeetings', 0)} meetings, "
        f"{stats.get('actionsDue', 0)} actions due, "
        f"{stats.get('emailsFlagged', 0)} emails flagged",
        file=sys.stderr,
    )

    return 0


if __name__ == "__main__":
    sys.exit(main())
