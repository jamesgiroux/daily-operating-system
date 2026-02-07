"""Stable ID generation for meetings and actions.

Extracted from deliver_today.py per ADR-0030.
"""

from __future__ import annotations

import hashlib
import re
from datetime import datetime
from typing import Any


def make_action_id(prefix: str, title: str, account: str = "", due: str = "") -> str:
    """Generate a content-stable action ID.

    Uses a hash of title + account + due date so the same action always
    gets the same ID regardless of its position in the list.  This
    prevents zombie duplicates when surrounding actions change order
    between briefing runs (I23).
    """
    key = f"{title.lower().strip()}|{account.lower().strip()}|{due.strip()}"
    h = hashlib.sha256(key.encode()).hexdigest()[:10]
    return f"{prefix}-{h}"


def make_meeting_id(event: dict[str, Any], meeting_type: str) -> str:
    """Generate a stable meeting ID from a calendar event.

    Format: ``HHMM-type-slug`` (e.g. ``"0900-customer-acme-sync"``).
    Falls back to a short hash prefix if time parsing fails.
    """
    title = event.get("summary", event.get("title", "untitled"))
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
