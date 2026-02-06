#!/usr/bin/env python3
"""Phase 1: Today Preparation Script for DailyOS Daybreak.

Gathers deterministic data (calendar, email, actions, workspace state) and
writes a JSON directive for Claude Code to enrich in Phase 2.

This script is **self-contained**: it uses only the Python standard library
plus google-api-python-client.  No local library imports.

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

import json
import os
import re
import sys
from datetime import datetime, date, timedelta, timezone
from pathlib import Path
from typing import Any, Optional

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

TOKEN_PATH = Path.home() / ".dailyos" / "google" / "token.json"
CONFIG_PATH = Path.home() / ".dailyos" / "config.json"

SCOPES = [
    "https://www.googleapis.com/auth/calendar",
    "https://www.googleapis.com/auth/gmail.modify",
    "https://www.googleapis.com/auth/gmail.compose",
    "https://www.googleapis.com/auth/spreadsheets",
    "https://www.googleapis.com/auth/documents",
    "https://www.googleapis.com/auth/drive",
]

# Work-day boundaries for gap analysis (24h clock)
WORK_DAY_START_HOUR = 9
WORK_DAY_END_HOUR = 17

# Minimum gap length worth reporting (minutes)
MIN_GAP_MINUTES = 30

# All-Hands attendee threshold (per MEETING-TYPES.md)
ALL_HANDS_THRESHOLD = 50

# Personal email domains (not tied to any organization)
PERSONAL_EMAIL_DOMAINS = frozenset({
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "hotmail.com",
    "yahoo.com",
    "icloud.com",
    "me.com",
    "live.com",
})

# Email priority keywords
HIGH_PRIORITY_SUBJECT_KEYWORDS = (
    "urgent",
    "asap",
    "action required",
    "please respond",
    "deadline",
    "escalation",
    "critical",
)

LOW_PRIORITY_SIGNALS = (
    "newsletter",
    "digest",
    "notification",
    "automated",
    "noreply",
    "no-reply",
    "unsubscribe",
)


def _build_account_domain_hints(workspace: Path) -> set[str]:
    """Scan Accounts/ directory for customer name slugs.

    Used to enhance email classification by matching sender domains
    against known customer account names.  For example, the account
    directory ``Bring-a-Trailer`` yields the slug ``bringatrailer``,
    which matches the domain base of ``bringatrailer.com``.

    Returns a set of lowercased slugs with non-alphanumeric chars removed.
    """
    accounts_dir = workspace / "Accounts"
    if not accounts_dir.is_dir():
        return set()

    hints: set[str] = set()
    try:
        for d in accounts_dir.iterdir():
            if d.is_dir() and not d.name.startswith((".", "_")):
                slug = re.sub(r"[^a-z0-9]", "", d.name.lower())
                if len(slug) >= 3:
                    hints.add(slug)
    except OSError:
        pass
    return hints


# ---------------------------------------------------------------------------
# Helpers: JSON I/O
# ---------------------------------------------------------------------------

def load_json(path: Path) -> Optional[dict[str, Any]]:
    """Read and parse a JSON file, returning None on any failure."""
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def write_json(path: Path, data: dict[str, Any]) -> None:
    """Atomically write a JSON file (create parent dirs as needed)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, default=str), encoding="utf-8")


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
# Helpers: config loading
# ---------------------------------------------------------------------------

def load_config() -> dict[str, Any]:
    """Load ~/.dailyos/config.json if available."""
    return load_json(CONFIG_PATH) or {}


def get_profile(config: dict[str, Any]) -> str:
    """Extract the user profile from config, defaulting to 'general'."""
    return config.get("profile", "general")


def get_user_domain(config: dict[str, Any]) -> str:
    """Derive the user's email domain for internal/external classification.

    Resolution chain:
        1. config.json  -> userEmail field
        2. config.json  -> email field
        3. token.json   -> client_email (rare, but some tokens carry it)
        4. Empty string (classification degrades gracefully)
    """
    # Try config fields
    for key in ("userEmail", "email"):
        email = config.get(key, "")
        if email and "@" in email:
            return email.split("@")[1].lower()

    # Try token file
    token_data = load_json(TOKEN_PATH)
    if token_data:
        for key in ("account", "client_email"):
            email = token_data.get(key, "")
            if email and "@" in email:
                return email.split("@")[1].lower()

    return ""


# ---------------------------------------------------------------------------
# Google API: authentication
# ---------------------------------------------------------------------------

def _build_google_credentials() -> Any | None:
    """Load and refresh Google OAuth2 credentials, or return None."""
    try:
        from google.oauth2.credentials import Credentials
        from google.auth.transport.requests import Request
    except ImportError:
        _warn("google-api-python-client not installed. Google data will be empty.")
        return None

    if not TOKEN_PATH.exists():
        _warn(f"Google token not found at {TOKEN_PATH}. Google data will be empty.")
        return None

    try:
        creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)
    except (ValueError, KeyError) as exc:
        _warn(f"Google token format invalid: {exc}. Google data will be empty.")
        return None

    if creds.expired and creds.refresh_token:
        try:
            creds.refresh(Request())
            # Persist refreshed token
            TOKEN_PATH.write_text(creds.to_json())
        except Exception as exc:
            _warn(f"Token refresh failed: {exc}. Google data will be empty.")
            return None

    if not creds.valid:
        _warn("Google credentials are invalid. Google data will be empty.")
        return None

    return creds


def _build_calendar_service() -> Any | None:
    """Return an authenticated Google Calendar API service, or None."""
    creds = _build_google_credentials()
    if creds is None:
        return None

    try:
        from googleapiclient.discovery import build
        return build("calendar", "v3", credentials=creds, cache_discovery=False)
    except Exception as exc:
        _warn(f"Calendar service build failed: {exc}")
        return None


def _build_gmail_service() -> Any | None:
    """Return an authenticated Gmail API service, or None."""
    creds = _build_google_credentials()
    if creds is None:
        return None

    try:
        from googleapiclient.discovery import build
        return build("gmail", "v1", credentials=creds, cache_discovery=False)
    except Exception as exc:
        _warn(f"Gmail service build failed: {exc}")
        return None


# ---------------------------------------------------------------------------
# Google Calendar: fetch today's events
# ---------------------------------------------------------------------------

def fetch_today_events() -> list[dict[str, Any]]:
    """Fetch calendar events from midnight to midnight today.

    Returns a list of normalized event dicts with keys:
        id, summary, start, end, attendees, organizer, description, location,
        is_recurring.
    """
    service = _build_calendar_service()
    if service is None:
        return []

    today = date.today()
    time_min = datetime.combine(today, datetime.min.time()).isoformat() + "Z"
    time_max = datetime.combine(
        today + timedelta(days=1), datetime.min.time()
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
# Google Gmail: fetch unread emails
# ---------------------------------------------------------------------------

def fetch_unread_emails(max_results: int = 30) -> list[dict[str, Any]]:
    """Fetch unread emails from the last 24 hours.

    Returns a list of dicts with keys:
        id, thread_id, from, subject, snippet, date.
    """
    service = _build_gmail_service()
    if service is None:
        return []

    try:
        # Search for unread emails from last 24h
        results = (
            service.users()
            .messages()
            .list(
                userId="me",
                q="is:unread newer_than:1d",
                maxResults=max_results,
            )
            .execute()
        )

        messages = results.get("messages", [])
        if not messages:
            return []

        emails: list[dict[str, Any]] = []
        for msg_stub in messages:
            try:
                msg = (
                    service.users()
                    .messages()
                    .get(
                        userId="me",
                        id=msg_stub["id"],
                        format="metadata",
                        metadataHeaders=["From", "Subject", "Date"],
                    )
                    .execute()
                )

                headers = {
                    h["name"]: h["value"]
                    for h in msg.get("payload", {}).get("headers", [])
                }

                emails.append({
                    "id": msg.get("id", ""),
                    "thread_id": msg.get("threadId", ""),
                    "from": headers.get("From", ""),
                    "subject": headers.get("Subject", ""),
                    "snippet": msg.get("snippet", ""),
                    "date": headers.get("Date", ""),
                })
            except Exception:
                # Skip individual messages that fail
                continue

        return emails

    except Exception as exc:
        _warn(f"Gmail API error: {exc}")
        return []


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

    # External attendees present -> customer or external
    result["external_domains"] = sorted(external_domains)

    # Apply title override if set (e.g., QBR with external attendees)
    if title_override_type:
        result["type"] = title_override_type
    else:
        # Default: customer (will be refined in meeting_contexts by account matching)
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
    """Bucket event IDs into past, in_progress, and upcoming.

    Args:
        events: List of event dicts (must have 'id', 'start', 'end').
        now: Current time (aware datetime).

    Returns:
        Dict with keys 'past', 'in_progress', 'upcoming' containing event IDs.
    """
    result: dict[str, list[str]] = {
        "past": [],
        "in_progress": [],
        "upcoming": [],
    }

    # Ensure now is timezone-aware
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
# Calendar gap analysis
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# Email classification
# ---------------------------------------------------------------------------

def _extract_email_address(from_field: str) -> str:
    """Extract bare email from a 'From' header like 'Name <email@example.com>'."""
    if "<" in from_field and ">" in from_field:
        return from_field.split("<")[1].split(">")[0].lower()
    return from_field.strip().lower()


def _extract_domain(email_addr: str) -> str:
    """Extract domain from an email address."""
    if "@" in email_addr:
        return email_addr.split("@")[1].lower()
    return ""


def classify_email_priority(
    email: dict[str, Any],
    customer_domains: set[str],
    user_domain: str,
    account_hints: set[str] | None = None,
) -> str:
    """Classify email priority: 'high', 'medium', or 'low'.

    High: from customer domains, from known account domains, or subject
          contains urgency keywords.
    Medium: from internal colleagues, or meeting-related.
    Low: newsletters, automated, GitHub notifications.
    """
    from_raw = email.get("from", "")
    from_addr = _extract_email_address(from_raw)
    domain = _extract_domain(from_addr)
    subject_lower = email.get("subject", "").lower()

    # HIGH: Customer domains (from today's meeting attendees)
    if domain in customer_domains:
        return "high"

    # HIGH: Sender domain matches a known customer account
    if account_hints and domain:
        domain_base = domain.split(".")[0]
        for hint in account_hints:
            if hint == domain_base or (len(hint) >= 4 and hint in domain_base):
                return "high"

    # HIGH: Urgency keywords in subject
    if any(kw in subject_lower for kw in HIGH_PRIORITY_SUBJECT_KEYWORDS):
        return "high"

    # LOW: Newsletters, automated, GitHub
    from_lower = from_raw.lower()
    if any(signal in from_lower or signal in subject_lower for signal in LOW_PRIORITY_SIGNALS):
        return "low"
    if "github.com" in domain:
        return "low"

    # MEDIUM: Internal colleagues
    if user_domain and domain == user_domain:
        return "medium"

    # MEDIUM: Meeting-related
    if any(kw in subject_lower for kw in ("meeting", "calendar", "invite")):
        return "medium"

    return "medium"


# ---------------------------------------------------------------------------
# Actions: parse actions.md
# ---------------------------------------------------------------------------

_CHECKBOX_RE = re.compile(r"^\s*-\s*\[\s*\]\s*(.+)$")
_DUE_RE = re.compile(r"due[:\s]+(\d{4}-\d{2}-\d{2})", re.IGNORECASE)
_PRIORITY_RE = re.compile(r"\b(P[123])\b", re.IGNORECASE)
_ACCOUNT_RE = re.compile(r"@(\S+)")
_CONTEXT_RE = re.compile(r"#(\S+)")
_WAITING_RE = re.compile(r"\b(waiting|blocked|pending)\b", re.IGNORECASE)


def parse_actions(workspace: Path) -> dict[str, list[dict[str, Any]]]:
    """Parse {workspace}/actions.md for open action items.

    Returns categorized actions:
        overdue, due_today, due_this_week, waiting_on
    """
    result: dict[str, list[dict[str, Any]]] = {
        "overdue": [],
        "due_today": [],
        "due_this_week": [],
        "waiting_on": [],
    }

    actions_path = workspace / "actions.md"
    if not actions_path.exists():
        # Also check _today/actions.md
        actions_path = workspace / "_today" / "actions.md"
        if not actions_path.exists():
            return result

    try:
        content = actions_path.read_text(encoding="utf-8")
    except OSError:
        return result

    today = date.today()
    # Monday of current week
    monday = today - timedelta(days=today.weekday())
    friday = monday + timedelta(days=4)

    for line in content.splitlines():
        match = _CHECKBOX_RE.match(line)
        if not match:
            continue

        text = match.group(1).strip()

        # Extract metadata
        due_match = _DUE_RE.search(text)
        priority_match = _PRIORITY_RE.search(text)
        account_match = _ACCOUNT_RE.search(text)
        context_match = _CONTEXT_RE.search(text)

        due_date: date | None = None
        if due_match:
            try:
                due_date = date.fromisoformat(due_match.group(1))
            except ValueError:
                pass

        # Clean the title: remove metadata markers
        title = text
        for pattern in (_DUE_RE, _PRIORITY_RE, _ACCOUNT_RE, _CONTEXT_RE):
            title = pattern.sub("", title)
        title = re.sub(r"\s+", " ", title).strip()

        action: dict[str, Any] = {
            "title": title,
            "account": account_match.group(1) if account_match else None,
            "due_date": due_date.isoformat() if due_date else None,
            "priority": priority_match.group(1).upper() if priority_match else "P3",
            "context": context_match.group(1) if context_match else None,
            "raw": text,
        }

        # Check for "waiting on" items
        if _WAITING_RE.search(text):
            result["waiting_on"].append(action)
            continue

        # Categorize by due date
        if due_date is not None:
            if due_date < today:
                days_overdue = (today - due_date).days
                action["days_overdue"] = days_overdue
                result["overdue"].append(action)
            elif due_date == today:
                result["due_today"].append(action)
            elif monday <= due_date <= friday:
                result["due_this_week"].append(action)
        else:
            # No due date: treat as due_this_week (low priority)
            result["due_this_week"].append(action)

    return result


# ---------------------------------------------------------------------------
# Meeting contexts (reference approach -- DEC19)
# ---------------------------------------------------------------------------

def gather_meeting_contexts(
    classified: list[dict[str, Any]],
    workspace: Path,
) -> list[dict[str, Any]]:
    """Build file-reference contexts for meetings that need prep.

    For customer meetings, finds account dashboards and recent meeting
    summaries.  These are path references -- Claude reads the actual
    files during Phase 2 enrichment.
    """
    contexts: list[dict[str, Any]] = []
    accounts_dir = workspace / "Accounts"

    for meeting in classified:
        meeting_type = meeting.get("type", "")

        # Only gather context for meetings that benefit from prep
        if meeting_type in ("personal", "all_hands"):
            continue

        ctx: dict[str, Any] = {
            "event_id": meeting.get("id"),
            "title": meeting.get("title"),
            "start": meeting.get("start"),
            "type": meeting_type,
            "refs": {},
        }

        # For customer / qbr / training meetings, look for account files
        if meeting_type in ("customer", "qbr", "training") and accounts_dir.is_dir():
            account_name = _guess_account_name(meeting, accounts_dir)
            if account_name:
                ctx["account"] = account_name
                account_path = accounts_dir / account_name

                # Dashboard file
                dashboard = _find_file_in_dir(account_path, "dashboard.md")
                if dashboard:
                    ctx["refs"]["account_dashboard"] = str(dashboard)

                # Recent meeting summaries (search archive)
                archive_dir = workspace / "_archive"
                recent = _find_recent_summaries(account_name, archive_dir, limit=2)
                if recent:
                    ctx["refs"]["meeting_history"] = [str(p) for p in recent]

                # Stakeholder map
                stakeholders = _find_file_in_dir(account_path, "stakeholders.md")
                if stakeholders:
                    ctx["refs"]["stakeholder_map"] = str(stakeholders)

                # Account actions
                actions = _find_file_in_dir(account_path, "actions.md")
                if actions:
                    ctx["refs"]["account_actions"] = str(actions)

        # For external meetings with unknown domains, note them for research
        elif meeting_type == "external":
            unknown_domains = meeting.get("external_domains", [])
            if unknown_domains:
                ctx["unknown_domains"] = unknown_domains
                # Search archive for any mentions
                archive_dir = workspace / "_archive"
                for domain in unknown_domains[:3]:  # Limit search effort
                    mentions = _search_archive(domain, archive_dir, max_results=3)
                    if mentions:
                        ctx["refs"][f"archive_{domain}"] = [str(p) for p in mentions]

        # For internal / team_sync / one_on_one, find last meeting
        elif meeting_type in ("internal", "team_sync", "one_on_one"):
            archive_dir = workspace / "_archive"
            title = meeting.get("title", "")
            if title:
                recent = _find_recent_summaries(title, archive_dir, limit=1)
                if recent:
                    ctx["refs"]["last_meeting"] = str(recent[0])

        contexts.append(ctx)

    return contexts


def _guess_account_name(
    meeting: dict[str, Any],
    accounts_dir: Path,
) -> str | None:
    """Try to match a meeting to a known account directory.

    Heuristic: check if any account directory name appears in the meeting
    title or in the external email domains.
    """
    if not accounts_dir.is_dir():
        return None

    title_lower = meeting.get("title", "").lower()
    external_domains = meeting.get("external_domains", [])

    try:
        account_names = [d.name for d in accounts_dir.iterdir() if d.is_dir()]
    except OSError:
        return []

    for name in account_names:
        # Check title
        if name.lower() in title_lower:
            return name
        # Check domain (simplified: account name in domain)
        for domain in external_domains:
            # Strip TLD and compare
            domain_base = domain.split(".")[0].lower()
            if domain_base == name.lower() or name.lower() in domain_base:
                return name

    return None


def _find_file_in_dir(directory: Path, filename: str) -> Path | None:
    """Find a file by name in a directory (case-insensitive)."""
    if not directory.is_dir():
        return None

    # Exact match first
    exact = directory / filename
    if exact.exists():
        return exact

    # Case-insensitive search
    target_lower = filename.lower()
    try:
        for item in directory.iterdir():
            if item.is_file() and item.name.lower() == target_lower:
                return item
    except OSError:
        pass

    return None


def _find_recent_summaries(
    search_term: str,
    archive_dir: Path,
    limit: int = 2,
) -> list[Path]:
    """Find recent meeting summaries mentioning a search term.

    Searches the archive directory for .md files whose names contain
    the search term (case-insensitive).  Returns the most recent matches
    sorted by modification time.
    """
    if not archive_dir.is_dir():
        return []

    search_lower = search_term.lower()
    # Normalize for filename matching (e.g., "Acme Corp" -> "acme-corp")
    search_slug = re.sub(r"[^a-z0-9]+", "-", search_lower).strip("-")

    matches: list[tuple[float, Path]] = []

    try:
        # Walk recent archive dates (limit depth for performance)
        date_dirs = sorted(
            [d for d in archive_dir.iterdir() if d.is_dir()],
            key=lambda d: d.name,
            reverse=True,
        )[:30]  # Last 30 archive dates

        for date_dir in date_dirs:
            try:
                for f in date_dir.iterdir():
                    if not f.is_file() or f.suffix != ".md":
                        continue
                    fname_lower = f.name.lower()
                    if search_lower in fname_lower or search_slug in fname_lower:
                        matches.append((f.stat().st_mtime, f))
            except OSError:
                continue
    except OSError:
        pass

    # Sort by modification time (most recent first) and limit
    matches.sort(key=lambda x: x[0], reverse=True)
    return [m[1] for m in matches[:limit]]


def _search_archive(
    query: str,
    archive_dir: Path,
    max_results: int = 5,
    lookback_dirs: int = 14,
) -> list[Path]:
    """Search recent archive files for content matching a query.

    Scans file content (not just names) in the most recent archive dates.
    Returns matching file paths.
    """
    if not archive_dir.is_dir():
        return []

    query_lower = query.lower()
    matches: list[Path] = []

    try:
        date_dirs = sorted(
            [d for d in archive_dir.iterdir() if d.is_dir()],
            key=lambda d: d.name,
            reverse=True,
        )[:lookback_dirs]

        for date_dir in date_dirs:
            if len(matches) >= max_results:
                break
            try:
                for f in date_dir.rglob("*.md"):
                    if len(matches) >= max_results:
                        break
                    try:
                        content = f.read_text(errors="ignore").lower()
                        if query_lower in content:
                            matches.append(f)
                    except OSError:
                        continue
            except OSError:
                continue
    except OSError:
        pass

    return matches


# ---------------------------------------------------------------------------
# Workspace file inventory
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
# AI task generation
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
# Logging helper
# ---------------------------------------------------------------------------

def _info(msg: str) -> None:
    """Print progress info to stderr."""
    print(f"  {msg}", file=sys.stderr)


def _warn(msg: str) -> None:
    """Print a warning to stderr."""
    print(f"  WARN: {msg}", file=sys.stderr)


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

    # ---- Step 2: Fetch calendar events ----
    _info("")
    _info("Step 2: Fetching calendar events...")

    raw_events = fetch_today_events()
    _info(f"  Found {len(raw_events)} events")

    # ---- Step 3: Classify meetings ----
    _info("")
    _info("Step 3: Classifying meetings...")

    classified: list[dict[str, Any]] = []
    for ev in raw_events:
        classified.append(classify_meeting(ev, user_domain))

    # Bucket by type
    meetings_by_type: dict[str, list[dict[str, Any]]] = {
        "customer": [],
        "internal": [],
        "team_sync": [],
        "one_on_one": [],
        "partnership": [],
        "qbr": [],
        "training": [],
        "external": [],
        "all_hands": [],
        "personal": [],
    }

    for ev in classified:
        mt = ev.get("type", "unknown")
        if mt in meetings_by_type:
            meetings_by_type[mt].append(ev)
        else:
            meetings_by_type["external"].append(ev)

    type_counts = {k: len(v) for k, v in meetings_by_type.items() if v}
    for mt, count in sorted(type_counts.items()):
        _info(f"    {mt}: {count}")

    # ---- Step 4: Time classification (past / in_progress / upcoming) ----
    _info("")
    _info("Step 4: Classifying event times...")

    time_status = classify_event_times(raw_events, now)
    _info(f"  Past: {len(time_status['past'])}, "
          f"In progress: {len(time_status['in_progress'])}, "
          f"Upcoming: {len(time_status['upcoming'])}")

    # ---- Step 5: Calendar gaps ----
    _info("")
    _info("Step 5: Analyzing calendar gaps...")

    gaps = compute_gaps(raw_events, today)
    total_gap_minutes = sum(g["duration_minutes"] for g in gaps)
    _info(f"  {len(gaps)} gaps totaling {total_gap_minutes} min of focus time")

    # ---- Step 6: Fetch emails ----
    _info("")
    _info("Step 6: Fetching emails...")

    raw_emails = fetch_unread_emails(max_results=30)
    _info(f"  Found {len(raw_emails)} unread emails")

    # Classify emails
    # Build customer domain set from external meeting attendees
    customer_domains: set[str] = set()
    for ev in meetings_by_type.get("customer", []):
        for domain in ev.get("external_domains", []):
            customer_domains.add(domain)

    # Enhance with known account domain hints from Accounts/ directory
    account_hints = _build_account_domain_hints(workspace)
    _info(f"  Account domain hints: {len(account_hints)}")

    all_classified_emails: list[dict[str, Any]] = []
    emails_high: list[dict[str, Any]] = []
    emails_medium_count = 0
    emails_low_count = 0

    for email in raw_emails:
        priority = classify_email_priority(
            email, customer_domains, user_domain, account_hints,
        )
        from_raw = email.get("from", "")
        email_obj: dict[str, Any] = {
            "id": email.get("id"),
            "thread_id": email.get("thread_id"),
            "from": from_raw,
            "from_email": _extract_email_address(from_raw),
            "subject": email.get("subject"),
            "snippet": email.get("snippet"),
            "date": email.get("date"),
            "priority": priority,
        }
        all_classified_emails.append(email_obj)
        if priority == "high":
            emails_high.append(email_obj)
        elif priority == "medium":
            emails_medium_count += 1
        else:
            emails_low_count += 1

    _info(f"  High: {len(emails_high)}, Medium: {emails_medium_count}, Low: {emails_low_count}")

    # ---- Step 7: Parse actions ----
    _info("")
    _info("Step 7: Parsing action items...")

    actions = parse_actions(workspace)
    _info(f"  Overdue: {len(actions['overdue'])}, "
          f"Due today: {len(actions['due_today'])}, "
          f"Due this week: {len(actions['due_this_week'])}, "
          f"Waiting on: {len(actions['waiting_on'])}")

    # ---- Step 8: Meeting contexts (reference approach -- DEC19) ----
    _info("")
    _info("Step 8: Gathering meeting contexts...")

    meeting_contexts = gather_meeting_contexts(classified, workspace)
    refs_count = sum(len(ctx.get("refs", {})) for ctx in meeting_contexts)
    _info(f"  {len(meeting_contexts)} meetings with context, {refs_count} file references")

    # ---- Step 9: File inventory ----
    _info("")
    _info("Step 9: Inventorying workspace files...")

    existing_today = inventory_today_files(workspace)
    inbox_pending = count_inbox_pending(workspace)
    _info(f"  Existing _today/ files: {len(existing_today)}")
    _info(f"  Inbox pending: {inbox_pending}")

    # ---- Step 10: Generate AI tasks ----
    _info("")
    _info("Step 10: Generating AI task list...")

    ai_tasks = generate_ai_tasks(classified, time_status, emails_high)
    _info(f"  {len(ai_tasks)} AI tasks generated")

    # ---- Build directive ----
    # Strip attendee lists from the classified output to keep directive lean.
    # Full attendee data is available from the raw events if Claude needs it.
    lean_events = [
        {
            "id": ev.get("id"),
            "summary": ev.get("summary"),
            "start": ev.get("start"),
            "end": ev.get("end"),
        }
        for ev in raw_events
    ]

    # Strip attendees from meeting type buckets for directive size
    def _lean_meeting(m: dict[str, Any]) -> dict[str, Any]:
        return {
            k: v for k, v in m.items()
            if k != "attendees"
        }

    lean_meetings = {
        mt: [_lean_meeting(m) for m in meetings]
        for mt, meetings in meetings_by_type.items()
    }

    directive: dict[str, Any] = {
        "command": "today",
        "generated_at": now.isoformat(),
        "context": context,
        "calendar": {
            "events": lean_events,
            "past": time_status["past"],
            "in_progress": time_status["in_progress"],
            "upcoming": time_status["upcoming"],
            "gaps": gaps,
        },
        "meetings": lean_meetings,
        "meeting_contexts": meeting_contexts,
        "actions": actions,
        "emails": {
            "high_priority": emails_high,
            "classified": all_classified_emails,
            "medium_count": emails_medium_count,
            "low_count": emails_low_count,
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
    print(f"  Events:    {len(raw_events)}", file=sys.stderr)
    print(f"  Customer:  {len(meetings_by_type['customer'])}", file=sys.stderr)
    print(f"  Actions:   {len(actions['overdue'])} overdue, {len(actions['due_today'])} due today", file=sys.stderr)
    print(f"  Emails:    {len(emails_high)} high priority", file=sys.stderr)
    print(f"  AI tasks:  {len(ai_tasks)}", file=sys.stderr)
    print(f"  Focus:     {total_gap_minutes} min available", file=sys.stderr)
    print("", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
