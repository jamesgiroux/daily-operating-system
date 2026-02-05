#!/usr/bin/env python3
"""
Calendar utilities for daily operating system scripts.
Handles calendar fetching, event classification, and time calculations.
"""

import json
import subprocess
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple

# Path to Google API script
GOOGLE_API_PATH = Path(__file__).parent.parent.parent / ".config/google/google_api.py"

# Cache for API availability check
_api_available_cache: Optional[Tuple[bool, str]] = None


def check_google_api_available() -> Tuple[bool, str]:
    """
    Check if Google API is available and authenticated.

    Returns:
        Tuple of (available: bool, reason: str)
        - (True, "ok") if available
        - (False, reason) with explanation if not
    """
    global _api_available_cache

    # Return cached result if available
    if _api_available_cache is not None:
        return _api_available_cache

    # Check 1: Does the google_api.py script exist?
    if not GOOGLE_API_PATH.exists():
        _api_available_cache = (False, "google_api.py not found - run setup wizard")
        return _api_available_cache

    # Check 2: Does credentials.json exist?
    creds_path = GOOGLE_API_PATH.parent / "credentials.json"
    if not creds_path.exists():
        _api_available_cache = (False, "credentials.json not found - complete Google API setup")
        return _api_available_cache

    # Check 3: Does token.json exist? (indicates prior authentication)
    token_path = GOOGLE_API_PATH.parent / "token.json"
    if not token_path.exists():
        _api_available_cache = (False, "not authenticated - run: python3 .config/google/google_api.py calendar list 1")
        return _api_available_cache

    # All checks passed
    _api_available_cache = (True, "ok")
    return _api_available_cache


def extract_json_from_output(output: str) -> str:
    """
    Extract JSON from output that may contain warning messages.

    The Google API script may print Python warnings before the JSON output.
    This function finds the first JSON array or object and returns it.

    Args:
        output: Raw stdout that may contain warnings + JSON

    Returns:
        The JSON portion of the output, or empty string if not found
    """
    if not output:
        return ""

    # Find the first [ or { which starts JSON
    for i, char in enumerate(output):
        if char == '[' or char == '{':
            return output[i:]

    return output


def fetch_calendar_events(days: int = 1) -> List[Dict[str, Any]]:
    """
    Fetch calendar events for the specified number of days.

    Args:
        days: Number of days to fetch (default 1 for today)

    Returns:
        List of event dictionaries with id, summary, start, end, attendees
    """
    try:
        result = subprocess.run(
            ["python3", str(GOOGLE_API_PATH), "calendar", "list", str(days)],
            capture_output=True,
            text=True,
            timeout=30
        )

        if result.returncode != 0:
            print(f"Warning: Calendar fetch failed: {result.stderr}", file=sys.stderr)
            return []

        output = result.stdout.strip()
        if not output or output == "No upcoming events found.":
            return []

        # Extract JSON from output (handles warnings printed before JSON)
        json_str = extract_json_from_output(output)
        if not json_str:
            return []

        return json.loads(json_str)

    except subprocess.TimeoutExpired:
        print("Warning: Calendar fetch timed out", file=sys.stderr)
        return []
    except json.JSONDecodeError as e:
        print(f"Warning: Failed to parse calendar response: {e}", file=sys.stderr)
        return []
    except Exception as e:
        print(f"Warning: Calendar fetch error: {e}", file=sys.stderr)
        return []


def get_event_details(event_id: str) -> Optional[Dict[str, Any]]:
    """
    Get detailed information about a specific calendar event.

    Args:
        event_id: The calendar event ID

    Returns:
        Event details dictionary or None if failed
    """
    try:
        result = subprocess.run(
            ["python3", str(GOOGLE_API_PATH), "calendar", "get", event_id],
            capture_output=True,
            text=True,
            timeout=30
        )

        if result.returncode != 0:
            return None

        # Extract JSON from output (handles warnings printed before JSON)
        json_str = extract_json_from_output(result.stdout)
        if not json_str:
            return None

        return json.loads(json_str)

    except Exception as e:
        print(f"Warning: Failed to get event details: {e}", file=sys.stderr)
        return None


def create_calendar_event(title: str, start: str, end: str, description: str = "") -> Optional[Dict[str, Any]]:
    """
    Create a new calendar event.

    Args:
        title: Event title
        start: Start time in ISO format (e.g., 2026-01-12T09:00:00-05:00)
        end: End time in ISO format
        description: Optional event description

    Returns:
        Created event info or None if failed
    """
    try:
        cmd = ["python3", str(GOOGLE_API_PATH), "calendar", "create", title, start, end]
        if description:
            cmd.append(description)

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)

        if result.returncode != 0:
            print(f"Warning: Failed to create event: {result.stderr}", file=sys.stderr)
            return None

        # Extract JSON from output (handles warnings printed before JSON)
        json_str = extract_json_from_output(result.stdout)
        if not json_str:
            return None

        return json.loads(json_str)

    except Exception as e:
        print(f"Warning: Event creation error: {e}", file=sys.stderr)
        return None


def get_week_dates(reference_date: datetime = None) -> Tuple[datetime, datetime, int]:
    """
    Get Monday and Friday of the week containing the reference date.

    Args:
        reference_date: Date to use (default: today)

    Returns:
        Tuple of (monday, friday, week_number)
    """
    if reference_date is None:
        reference_date = datetime.now()

    monday = reference_date - timedelta(days=reference_date.weekday())
    friday = monday + timedelta(days=4)
    week_number = reference_date.isocalendar()[1]

    return monday, friday, week_number


def get_business_days_ahead(start_date: datetime, num_days: int) -> List[datetime]:
    """
    Get the next N business days (Mon-Fri) starting from start_date.

    Args:
        start_date: Starting date
        num_days: Number of business days to return

    Returns:
        List of datetime objects for business days
    """
    business_days = []
    current = start_date

    while len(business_days) < num_days:
        current += timedelta(days=1)
        if current.weekday() < 5:  # Mon=0 through Fri=4
            business_days.append(current)

    return business_days


def filter_events_by_date(events: List[Dict], target_date: datetime) -> List[Dict]:
    """
    Filter events to only those on a specific date.

    Args:
        events: List of event dictionaries
        target_date: Date to filter for

    Returns:
        List of events on that date
    """
    target_str = target_date.strftime('%Y-%m-%d')
    filtered = []

    for event in events:
        start = event.get('start', '')
        if start.startswith(target_str):
            filtered.append(event)

    return filtered


def filter_events_by_status(events: List[Dict], current_time: datetime = None) -> Dict[str, List[Dict]]:
    """
    Categorize events by status: past, in_progress, upcoming.

    Args:
        events: List of event dictionaries
        current_time: Reference time (default: now)

    Returns:
        Dictionary with 'past', 'in_progress', 'upcoming' keys
    """
    if current_time is None:
        current_time = datetime.now()

    result = {'past': [], 'in_progress': [], 'upcoming': []}

    for event in events:
        start_str = event.get('start', '')
        end_str = event.get('end', '')

        try:
            # Handle both datetime and date-only formats
            if 'T' in start_str:
                start = datetime.fromisoformat(start_str.replace('Z', '+00:00'))
                end = datetime.fromisoformat(end_str.replace('Z', '+00:00'))
                # Make comparison timezone-naive
                if start.tzinfo:
                    start = start.replace(tzinfo=None)
                if end.tzinfo:
                    end = end.replace(tzinfo=None)
            else:
                # All-day event
                start = datetime.strptime(start_str, '%Y-%m-%d')
                end = datetime.strptime(end_str, '%Y-%m-%d')

            if current_time > end:
                result['past'].append(event)
            elif current_time >= start:
                result['in_progress'].append(event)
            else:
                result['upcoming'].append(event)

        except (ValueError, TypeError):
            # If parsing fails, treat as upcoming
            result['upcoming'].append(event)

    return result


def parse_event_time(time_str: str) -> Optional[datetime]:
    """
    Parse an event time string to datetime.

    Args:
        time_str: ISO format time string

    Returns:
        datetime object or None if parsing fails
    """
    try:
        if 'T' in time_str:
            dt = datetime.fromisoformat(time_str.replace('Z', '+00:00'))
            return dt.replace(tzinfo=None)
        else:
            return datetime.strptime(time_str, '%Y-%m-%d')
    except (ValueError, TypeError):
        return None


def format_time_for_display(time_str: str) -> str:
    """
    Format a time string for display (e.g., "9:00 AM").

    Args:
        time_str: ISO format time string

    Returns:
        Formatted time string
    """
    dt = parse_event_time(time_str)
    if dt is None:
        return time_str
    return dt.strftime('%-I:%M %p')


def format_time_for_filename(time_str: str) -> str:
    """
    Format a time string for filename (e.g., "0900").

    Args:
        time_str: ISO format time string

    Returns:
        Time formatted as HHMM
    """
    dt = parse_event_time(time_str)
    if dt is None:
        return "0000"
    return dt.strftime('%H%M')


def calculate_meeting_gaps(events: List[Dict], day_start: int = 9, day_end: int = 17) -> List[Dict]:
    """
    Find gaps between meetings that could be used for focus time.

    Args:
        events: List of event dictionaries for a day
        day_start: Start of workday (hour, default 9)
        day_end: End of workday (hour, default 17)

    Returns:
        List of gap dictionaries with start, end, duration_minutes
    """
    if not events:
        return []

    # Sort events by start time
    sorted_events = sorted(events, key=lambda e: e.get('start', ''))

    gaps = []

    # Check for gap before first meeting
    first_start = parse_event_time(sorted_events[0].get('start', ''))
    if first_start:
        day_start_time = first_start.replace(hour=day_start, minute=0, second=0)
        if first_start > day_start_time:
            duration = int((first_start - day_start_time).total_seconds() / 60)
            if duration >= 30:  # Only gaps of 30+ minutes
                gaps.append({
                    'start': day_start_time.isoformat(),
                    'end': first_start.isoformat(),
                    'duration_minutes': duration
                })

    # Check for gaps between meetings
    for i in range(len(sorted_events) - 1):
        current_end = parse_event_time(sorted_events[i].get('end', ''))
        next_start = parse_event_time(sorted_events[i + 1].get('start', ''))

        if current_end and next_start and next_start > current_end:
            duration = int((next_start - current_end).total_seconds() / 60)
            if duration >= 30:
                gaps.append({
                    'start': current_end.isoformat(),
                    'end': next_start.isoformat(),
                    'duration_minutes': duration
                })

    # Check for gap after last meeting
    last_end = parse_event_time(sorted_events[-1].get('end', ''))
    if last_end:
        day_end_time = last_end.replace(hour=day_end, minute=0, second=0)
        if last_end < day_end_time:
            duration = int((day_end_time - last_end).total_seconds() / 60)
            if duration >= 30:
                gaps.append({
                    'start': last_end.isoformat(),
                    'end': day_end_time.isoformat(),
                    'duration_minutes': duration
                })

    return gaps
