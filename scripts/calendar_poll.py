#!/usr/bin/env python3
"""Fetch today's Google Calendar events for the Daybreak native app.

Called by the Rust backend as a subprocess every 5 minutes.
Outputs a JSON array of calendar events to stdout.
All diagnostic messages go to stderr.

Exit codes:
    0 - Success (JSON array on stdout, may be empty)
    1 - General error (missing dependencies, network, etc.)
    2 - Authentication failure (token expired/revoked, needs re-auth)

Usage:
    python3 calendar_poll.py [workspace_path]
"""

from __future__ import annotations

import json
import os
import sys
from datetime import datetime, time, timedelta, timezone
from pathlib import Path
from typing import Any, Optional

# ---------------------------------------------------------------------------
# Dependency check
# ---------------------------------------------------------------------------

try:
    from google.auth.exceptions import RefreshError
    from google.auth.transport.requests import Request
    from google.oauth2.credentials import Credentials
    from googleapiclient.discovery import build
    from googleapiclient.errors import HttpError
except ImportError:
    print(
        "Missing Google API packages. "
        "Install with: pip install google-auth google-auth-oauthlib google-api-python-client",
        file=sys.stderr,
    )
    sys.exit(1)


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

TOKEN_PATH = Path.home() / ".dailyos" / "google" / "token.json"

# Scopes needed for read-only calendar access
SCOPES = ["https://www.googleapis.com/auth/calendar.readonly"]

# Title keywords for meeting type classification (case-insensitive)
_QBR_KEYWORDS = ("qbr", "business review", "quarterly review")
_ONE_ON_ONE_KEYWORDS = ("1:1", "1-on-1", "one on one", "one-on-one")
_TEAM_SYNC_KEYWORDS = ("standup", "stand-up", "sync", "team", "scrum", "daily standup")
_ALL_HANDS_KEYWORDS = ("all hands", "all-hands", "town hall")
_ALL_HANDS_ATTENDEE_THRESHOLD = 50


# ---------------------------------------------------------------------------
# Authentication
# ---------------------------------------------------------------------------


def load_credentials() -> Credentials:
    """Load and refresh Google OAuth2 credentials.

    Returns valid credentials or exits with code 2 on auth failure.
    """
    if not TOKEN_PATH.exists():
        print(
            f"Token not found at {TOKEN_PATH}. "
            "Run Google setup first: dailyos google-setup",
            file=sys.stderr,
        )
        sys.exit(2)

    try:
        creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)
    except Exception as exc:
        print(f"Failed to load token: {exc}", file=sys.stderr)
        sys.exit(2)

    if creds.valid:
        return creds

    if creds.expired and creds.refresh_token:
        try:
            creds.refresh(Request())
        except RefreshError as exc:
            print(f"Token refresh failed (re-auth required): {exc}", file=sys.stderr)
            sys.exit(2)
        except Exception as exc:
            print(f"Token refresh error: {exc}", file=sys.stderr)
            sys.exit(2)

        # Persist refreshed token
        try:
            TOKEN_PATH.parent.mkdir(parents=True, exist_ok=True)
            TOKEN_PATH.write_text(creds.to_json())
            os.chmod(TOKEN_PATH, 0o600)
        except OSError as exc:
            # Non-fatal: token works in memory even if we cannot persist
            print(f"Warning: could not save refreshed token: {exc}", file=sys.stderr)

        return creds

    # No refresh token or some other invalid state
    print(
        "Token is invalid and cannot be refreshed. Re-authenticate with: dailyos google-setup",
        file=sys.stderr,
    )
    sys.exit(2)


# ---------------------------------------------------------------------------
# Calendar fetching
# ---------------------------------------------------------------------------


def get_owner_domain(service: Any) -> Optional[str]:
    """Resolve the calendar owner's email domain from the primary calendar.

    Used to distinguish internal vs. external attendees without requiring
    workspace configuration.
    """
    try:
        cal = service.calendars().get(calendarId="primary").execute()
        owner_email: str = cal.get("id", "")
        if "@" in owner_email:
            return owner_email.split("@", 1)[1].lower()
    except HttpError:
        pass
    return None


def fetch_today_events(service: Any) -> list[dict[str, Any]]:
    """Fetch today's events from the primary calendar."""
    now = datetime.now(timezone.utc)
    local_today = datetime.now().date()

    # Build time window: start of today (local) to end of today (local)
    day_start = datetime.combine(local_today, time.min).astimezone(timezone.utc)
    day_end = datetime.combine(local_today, time.max).astimezone(timezone.utc)

    events_result = (
        service.events()
        .list(
            calendarId="primary",
            timeMin=day_start.isoformat(),
            timeMax=day_end.isoformat(),
            maxResults=100,
            singleEvents=True,
            orderBy="startTime",
        )
        .execute()
    )

    return events_result.get("items", [])


# ---------------------------------------------------------------------------
# Meeting type classification
# ---------------------------------------------------------------------------


def _has_video_conferencing(event: dict[str, Any]) -> bool:
    """Check if the event has a video conferencing link."""
    if event.get("hangoutLink"):
        return True
    for entry_point in event.get("conferenceData", {}).get("entryPoints", []):
        if entry_point.get("entryPointType") == "video":
            return True
    return False


def classify_meeting(
    event: dict[str, Any],
    attendee_emails: list[str],
    owner_domain: Optional[str],
) -> str:
    """Classify a calendar event into a meeting type string.

    Classification priority (first match wins):
        1. All-hands (50+ attendees or title match)
        2. QBR title keywords
        3. 1:1 title keywords
        4. Team sync title keywords
        5. Video conference with >2 external attendees -> "customer"
        6. All same domain -> "internal"
        7. Mixed domains -> "external"
        8. Default -> "internal"
    """
    title = (event.get("summary") or "").lower()
    num_attendees = len(attendee_emails)

    # --- Scale-based hard override ---
    if num_attendees >= _ALL_HANDS_ATTENDEE_THRESHOLD:
        return "all_hands"

    # --- Title-based overrides ---
    if any(kw in title for kw in _ALL_HANDS_KEYWORDS):
        return "all_hands"

    if any(kw in title for kw in _QBR_KEYWORDS):
        return "qbr"

    if any(kw in title for kw in _ONE_ON_ONE_KEYWORDS):
        return "one_on_one"

    if any(kw in title for kw in _TEAM_SYNC_KEYWORDS):
        return "team_sync"

    # --- Domain-based classification ---
    if owner_domain and attendee_emails:
        external_emails = [
            e for e in attendee_emails
            if "@" in e and e.split("@", 1)[1].lower() != owner_domain
        ]
        external_count = len(external_emails)

        # Video + >2 external attendees -> customer meeting
        if _has_video_conferencing(event) and external_count > 2:
            return "customer"

        if external_count == 0:
            return "internal"

        # Has external attendees from different domains
        external_domains = {
            e.split("@", 1)[1].lower() for e in external_emails if "@" in e
        }
        if len(external_domains) > 0:
            return "external"

    return "internal"


# ---------------------------------------------------------------------------
# Event formatting
# ---------------------------------------------------------------------------


def format_event(
    event: dict[str, Any],
    owner_domain: Optional[str],
) -> dict[str, Any]:
    """Transform a raw Google Calendar event into the Daybreak schema."""
    # Determine start/end and all-day status
    start_raw = event.get("start", {})
    end_raw = event.get("end", {})

    is_all_day = "date" in start_raw and "dateTime" not in start_raw

    if is_all_day:
        # All-day events: use the date value, normalize to ISO8601 with T00:00:00
        start_iso = start_raw["date"] + "T00:00:00"
        end_iso = end_raw.get("date", start_raw["date"]) + "T00:00:00"
    else:
        start_iso = start_raw.get("dateTime", "")
        end_iso = end_raw.get("dateTime", "")

    # Collect attendee emails (excluding the resource calendars)
    attendee_emails: list[str] = []
    for attendee in event.get("attendees", []):
        email = attendee.get("email", "")
        # Skip resource rooms (they contain "resource.calendar.google.com")
        if "resource.calendar.google.com" in email:
            continue
        if email:
            attendee_emails.append(email)

    meeting_type = classify_meeting(event, attendee_emails, owner_domain)

    return {
        "id": event.get("id", ""),
        "title": event.get("summary", "(No title)"),
        "start": start_iso,
        "end": end_iso,
        "meetingType": meeting_type,
        "account": None,
        "attendees": attendee_emails,
        "isAllDay": is_all_day,
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point. Outputs JSON array to stdout, diagnostics to stderr."""
    # Accept workspace path argument for forward compatibility (currently unused)
    _workspace = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()

    creds = load_credentials()

    try:
        service = build("calendar", "v3", credentials=creds, cache_discovery=False)
    except Exception as exc:
        print(f"Failed to build Calendar service: {exc}", file=sys.stderr)
        sys.exit(1)

    owner_domain = get_owner_domain(service)

    try:
        raw_events = fetch_today_events(service)
    except HttpError as exc:
        status = exc.resp.status if exc.resp else 0
        if status in (401, 403):
            print(f"Calendar API auth error ({status}): {exc}", file=sys.stderr)
            sys.exit(2)
        print(f"Calendar API error ({status}): {exc}", file=sys.stderr)
        sys.exit(1)
    except Exception as exc:
        print(f"Failed to fetch calendar events: {exc}", file=sys.stderr)
        sys.exit(1)

    # Filter out declined events
    formatted: list[dict[str, Any]] = []
    for event in raw_events:
        # Skip cancelled events
        if event.get("status") == "cancelled":
            continue

        # Skip events where the user has declined
        my_response = None
        for attendee in event.get("attendees", []):
            if attendee.get("self"):
                my_response = attendee.get("responseStatus")
                break

        if my_response == "declined":
            continue

        formatted.append(format_event(event, owner_domain))

    # Output clean JSON to stdout (nothing else)
    json.dump(formatted, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
