"""Shared configuration, workspace resolution, Google auth, and utilities.

Extracted from prepare_today.py and prepare_week.py per ADR-0030.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path
from typing import Any, Optional

# ---------------------------------------------------------------------------
# Paths
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

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

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
    "marketing",
    "promo",
    "promotions",
    "info@",
    "updates@",
    "news@",
    "do-not-reply",
    "donotreply",
    "notify",
    "mailer-daemon",
)

# Bulk/marketing sender domains (I21: FYI classification expansion)
BULK_SENDER_DOMAINS = frozenset({
    "mailchimp.com",
    "sendgrid.net",
    "mandrillapp.com",
    "hubspot.com",
    "marketo.com",
    "pardot.com",
    "intercom.io",
    "customer.io",
    "mailgun.org",
    "postmarkapp.com",
    "amazonses.com",
})

# Noreply local-part patterns (I21)
NOREPLY_LOCAL_PARTS = frozenset({
    "noreply",
    "no-reply",
    "donotreply",
    "do-not-reply",
    "mailer-daemon",
})

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

def _info(msg: str) -> None:
    """Print progress info to stderr."""
    print(f"  {msg}", file=sys.stderr)


def _warn(msg: str) -> None:
    """Print a warning to stderr."""
    print(f"  WARN: {msg}", file=sys.stderr)


# ---------------------------------------------------------------------------
# JSON I/O
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
# Workspace resolution
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
# Config loading
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
# Account domain hints
# ---------------------------------------------------------------------------

def build_account_domain_hints(workspace: Path) -> set[str]:
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
# Google API: authentication (I18: per-process credential + service caching)
# ---------------------------------------------------------------------------

_cached_credentials: Any | None = None
_cached_services: dict[str, Any] = {}


def build_google_credentials() -> Any | None:
    """Load and refresh Google OAuth2 credentials, or return None.

    Caches credentials within the process to avoid redundant token
    reads/refreshes when prepare_today.py calls both calendar and Gmail.
    """
    global _cached_credentials

    # Return cached if still valid
    if _cached_credentials is not None and _cached_credentials.valid:
        return _cached_credentials

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

    _cached_credentials = creds
    return creds


def build_calendar_service() -> Any | None:
    """Return an authenticated Google Calendar API service, or None.

    Cached per-process (I18).
    """
    if "calendar" in _cached_services:
        return _cached_services["calendar"]

    creds = build_google_credentials()
    if creds is None:
        return None

    try:
        from googleapiclient.discovery import build
        svc = build("calendar", "v3", credentials=creds, cache_discovery=False)
        _cached_services["calendar"] = svc
        return svc
    except Exception as exc:
        _warn(f"Calendar service build failed: {exc}")
        return None


def build_gmail_service() -> Any | None:
    """Return an authenticated Gmail API service, or None.

    Cached per-process (I18).
    """
    if "gmail" in _cached_services:
        return _cached_services["gmail"]

    creds = build_google_credentials()
    if creds is None:
        return None

    try:
        from googleapiclient.discovery import build
        svc = build("gmail", "v1", credentials=creds, cache_discovery=False)
        _cached_services["gmail"] = svc
        return svc
    except Exception as exc:
        _warn(f"Gmail service build failed: {exc}")
        return None
