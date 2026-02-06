#!/usr/bin/env python3
"""
Google OAuth authentication for DailyOS Daybreak.

Called by the Rust backend as a subprocess. Opens the user's browser
for Google consent, captures the redirect on localhost, saves the token.

Usage:
    python3 google_auth.py [workspace_path]

Output (stdout):
    {"status": "success", "email": "user@example.com"}
    {"status": "error", "message": "..."}

Token storage: ~/.dailyos/google/token.json
Credentials:   ~/.dailyos/google/credentials.json
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

GOOGLE_DIR: Path = Path.home() / ".dailyos" / "google"
CREDENTIALS_FILE: Path = GOOGLE_DIR / "credentials.json"
TOKEN_FILE: Path = GOOGLE_DIR / "token.json"

# MVP scopes: read-only calendar + read-only gmail
SCOPES: list[str] = [
    "https://www.googleapis.com/auth/calendar.readonly",
    "https://www.googleapis.com/auth/gmail.readonly",
]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _output(payload: dict[str, Any]) -> None:
    """Write JSON to stdout and exit.

    All diagnostic/log messages go to stderr so that only the JSON
    result appears on stdout for the Rust caller to parse.
    """
    print(json.dumps(payload), flush=True)


def _error(message: str) -> None:
    """Write an error result to stdout and exit.

    Exits with code 0 so the Rust caller reads stdout (where the JSON is).
    Error is communicated via the JSON status field, not the exit code.
    Exit code 1 is reserved for unhandled crashes.
    """
    _output({"status": "error", "message": message})
    sys.exit(0)


def _ensure_directory() -> None:
    """Create the token directory with secure permissions if needed."""
    if not GOOGLE_DIR.exists():
        GOOGLE_DIR.mkdir(parents=True, exist_ok=True)
        os.chmod(GOOGLE_DIR, 0o700)


def _get_email(creds: Any) -> str:
    """Fetch the authenticated user's email address via the Gmail API.

    Falls back to the OAuth2 userinfo endpoint if Gmail fails, and
    returns "authenticated" as a last resort so the caller always
    gets a usable response.
    """
    try:
        from googleapiclient.discovery import build

        service = build("gmail", "v1", credentials=creds)
        profile = service.users().getProfile(userId="me").execute()
        return profile.get("emailAddress", "unknown")
    except Exception:
        pass

    # Fallback: userinfo endpoint
    try:
        from googleapiclient.discovery import build

        service = build("oauth2", "v2", credentials=creds)
        info = service.userinfo().get().execute()
        return info.get("email", "authenticated")
    except Exception:
        return "authenticated"


# ---------------------------------------------------------------------------
# Core auth flow
# ---------------------------------------------------------------------------


def _check_dependencies() -> None:
    """Verify required Google libraries are installed."""
    try:
        from google.oauth2.credentials import Credentials  # noqa: F401
        from google_auth_oauthlib.flow import InstalledAppFlow  # noqa: F401
        from google.auth.transport.requests import Request  # noqa: F401
    except ImportError:
        _error(
            "Required packages not installed. "
            "Run: pip install google-auth-oauthlib google-api-python-client"
        )


def _try_existing_token() -> Any | None:
    """Load and refresh an existing token if possible.

    Returns valid Credentials or None.
    """
    if not TOKEN_FILE.exists():
        return None

    from google.oauth2.credentials import Credentials
    from google.auth.transport.requests import Request

    try:
        creds = Credentials.from_authorized_user_file(str(TOKEN_FILE), SCOPES)
    except Exception as exc:
        print(f"[google_auth] Could not load token: {exc}", file=sys.stderr)
        return None

    if creds.valid:
        return creds

    if creds.expired and creds.refresh_token:
        try:
            creds.refresh(Request())
            _save_token(creds)
            return creds
        except Exception as exc:
            print(f"[google_auth] Token refresh failed: {exc}", file=sys.stderr)
            # Token is stale; fall through to full re-auth
            return None

    return None


def _run_consent_flow() -> Any:
    """Run the full OAuth consent flow via the system browser.

    Opens a browser window and starts a temporary localhost HTTP server
    on a random port to capture the redirect.
    """
    from google_auth_oauthlib.flow import InstalledAppFlow

    if not CREDENTIALS_FILE.exists():
        _error(
            f"Google credentials not found at {CREDENTIALS_FILE}. "
            "Download credentials.json from Google Cloud Console "
            "(OAuth 2.0 Desktop App) and place it at "
            f"{CREDENTIALS_FILE}"
        )

    # Validate the credentials file before using it
    try:
        with open(CREDENTIALS_FILE) as f:
            creds_data = json.load(f)
        if "installed" not in creds_data:
            key = "web" if "web" in creds_data else None
            hint = (
                " This looks like a Web Application credential. "
                "Use Desktop App type instead."
                if key == "web"
                else ""
            )
            _error(
                f"Invalid credentials.json format.{hint} "
                "Expected 'installed' key for Desktop App OAuth credentials."
            )
    except json.JSONDecodeError as exc:
        _error(f"credentials.json is not valid JSON: {exc}")

    try:
        flow = InstalledAppFlow.from_client_secrets_file(
            str(CREDENTIALS_FILE), SCOPES
        )
        # port=0 lets the OS pick a random available port
        creds = flow.run_local_server(port=0)
    except Exception as exc:
        _error(f"OAuth consent flow failed: {exc}")

    return creds


def _save_token(creds: Any) -> None:
    """Persist credentials to disk with secure permissions."""
    _ensure_directory()
    with open(TOKEN_FILE, "w") as f:
        f.write(creds.to_json())
    os.chmod(TOKEN_FILE, 0o600)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point. Authenticate and output JSON result."""
    # Workspace argument (unused by auth, but accepted for interface parity
    # with other scripts the Rust executor calls).
    _workspace = sys.argv[1] if len(sys.argv) > 1 else os.getcwd()

    _check_dependencies()

    # Try reusing an existing token first
    creds = _try_existing_token()

    if creds is None:
        # No valid token -- run full browser consent flow
        creds = _run_consent_flow()
        _save_token(creds)

    email = _get_email(creds)
    _output({"status": "success", "email": email})


if __name__ == "__main__":
    main()
