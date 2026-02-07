"""Rich meeting context gathering for a single meeting.

Extracted and enhanced from prepare_today.py per ADR-0030.
Informed by daily-csm MEETING-PREP.md — meeting prep should include:
  - Account dashboard (health, wins, risks, renewal)
  - Recent meeting history (SQLite)
  - Recent captures (wins/risks from post-meeting, I33)
  - Open actions for this account
  - File references (account tracker, summaries, archive)
"""

from __future__ import annotations

import re
import sqlite3
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional


@dataclass
class MeetingContext:
    """Rich context bundle for a single meeting."""
    event_id: str = ""
    title: str = ""
    start: str = ""
    meeting_type: str = ""
    account: str | None = None
    refs: dict[str, Any] = field(default_factory=dict)
    account_data: dict[str, Any] = field(default_factory=dict)
    recent_captures: list[dict[str, Any]] = field(default_factory=list)
    open_actions: list[dict[str, Any]] = field(default_factory=list)
    meeting_history: list[dict[str, Any]] = field(default_factory=list)
    unknown_domains: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        """Serialize to directive-compatible dict."""
        d: dict[str, Any] = {
            "event_id": self.event_id,
            "title": self.title,
            "start": self.start,
            "type": self.meeting_type,
            "refs": self.refs,
        }
        if self.account:
            d["account"] = self.account
        if self.account_data:
            d["account_data"] = self.account_data
        if self.recent_captures:
            d["recent_captures"] = self.recent_captures
        if self.open_actions:
            d["open_actions"] = self.open_actions
        if self.meeting_history:
            d["meeting_history"] = self.meeting_history
        if self.unknown_domains:
            d["unknown_domains"] = self.unknown_domains
        return d


def gather_meeting_context(
    meeting: dict[str, Any],
    workspace: Path,
    db_path: Path | None = None,
) -> MeetingContext:
    """Build rich context for a single meeting prep.

    Informed by daily-csm MEETING-PREP.md:
    - Account dashboard (health, wins, risks, renewal)
    - Recent meeting history (SQLite)
    - Recent captures (wins/risks from post-meeting, I33)
    - Open actions for this account
    - File references (account tracker, summaries, archive)

    Args:
        meeting: Classified meeting dict.
        workspace: Path to workspace root.
        db_path: Path to SQLite database. Defaults to ~/.dailyos/actions.db.

    Returns:
        MeetingContext with all gathered data.
    """
    meeting_type = meeting.get("type", "")
    ctx = MeetingContext(
        event_id=meeting.get("id", ""),
        title=meeting.get("title", ""),
        start=meeting.get("start", ""),
        meeting_type=meeting_type,
    )

    # Skip meetings that don't benefit from prep
    if meeting_type in ("personal", "all_hands"):
        return ctx

    accounts_dir = workspace / "Accounts"

    # For customer / qbr / training meetings, look for account files
    if meeting_type in ("customer", "qbr", "training") and accounts_dir.is_dir():
        account_name = _guess_account_name(meeting, accounts_dir)
        if account_name:
            ctx.account = account_name
            account_path = accounts_dir / account_name

            # File references (existing logic)
            dashboard = _find_file_in_dir(account_path, "dashboard.md")
            if dashboard:
                ctx.refs["account_dashboard"] = str(dashboard)

            archive_dir = workspace / "_archive"
            recent = _find_recent_summaries(account_name, archive_dir, limit=2)
            if recent:
                ctx.refs["meeting_history"] = [str(p) for p in recent]

            stakeholders = _find_file_in_dir(account_path, "stakeholders.md")
            if stakeholders:
                ctx.refs["stakeholder_map"] = str(stakeholders)

            actions_file = _find_file_in_dir(account_path, "actions.md")
            if actions_file:
                ctx.refs["account_actions"] = str(actions_file)

            # Dashboard data extraction (new: I33)
            if dashboard:
                ctx.account_data = _parse_dashboard(dashboard)

            # SQLite enrichment (new: I33)
            if db_path is None:
                db_path = Path.home() / ".dailyos" / "actions.db"

            if db_path.exists():
                # Recent captures for this account
                ctx.recent_captures = _get_captures_for_account(
                    db_path, account_name, days_back=14,
                )

                # Open actions for this account
                ctx.open_actions = _get_account_actions(db_path, account_name)

                # Recent meeting history
                ctx.meeting_history = _get_meeting_history(
                    db_path, account_name, lookback_days=30, limit=3,
                )

    # For external meetings with unknown domains, note them for research
    elif meeting_type == "external":
        unknown_domains = meeting.get("external_domains", [])
        if unknown_domains:
            ctx.unknown_domains = unknown_domains
            archive_dir = workspace / "_archive"
            for domain in unknown_domains[:3]:
                mentions = _search_archive(domain, archive_dir, max_results=3)
                if mentions:
                    ctx.refs[f"archive_{domain}"] = [str(p) for p in mentions]

    # For internal / team_sync / one_on_one, find last meeting
    elif meeting_type in ("internal", "team_sync", "one_on_one"):
        archive_dir = workspace / "_archive"
        title = meeting.get("title", "")
        if title:
            recent = _find_recent_summaries(title, archive_dir, limit=1)
            if recent:
                ctx.refs["last_meeting"] = str(recent[0])

    return ctx


def gather_all_meeting_contexts(
    classified: list[dict[str, Any]],
    workspace: Path,
    db_path: Path | None = None,
) -> list[dict[str, Any]]:
    """Build contexts for all meetings that need prep.

    Convenience wrapper over gather_meeting_context() for batch use.
    Returns list of dicts (directive-compatible).
    """
    contexts: list[dict[str, Any]] = []
    for meeting in classified:
        if meeting.get("type") in ("personal", "all_hands"):
            continue
        ctx = gather_meeting_context(meeting, workspace, db_path)
        contexts.append(ctx.to_dict())
    return contexts


# ---------------------------------------------------------------------------
# Dashboard parsing (new capability for I33)
# ---------------------------------------------------------------------------

def _parse_dashboard(dashboard_path: Path) -> dict[str, Any]:
    """Best-effort extraction of Quick View data from account dashboard.

    Parses markdown dashboard for:
    - Quick View table (ARR, Health Score, Renewal Date)
    - Recent Wins list
    - Current Risks list

    This is text parsing — doesn't need to be perfect; best-effort
    extraction that Phase 2 AI can work with.
    """
    data: dict[str, Any] = {}
    try:
        content = dashboard_path.read_text(encoding="utf-8")
    except OSError:
        return data

    # Extract key-value pairs from table rows or "Key: Value" lines
    kv_patterns = [
        (r"(?:ARR|Annual Revenue|MRR)\s*[:\|]\s*\$?([\d,\.]+[KMB]?)", "arr"),
        (r"(?:Health\s*(?:Score)?)\s*[:\|]\s*(\w+)", "health"),
        (r"(?:Renewal\s*(?:Date)?)\s*[:\|]\s*([\d\-/]+)", "renewal"),
        (r"(?:Ring|Tier)\s*[:\|]\s*(\d+)", "ring"),
        (r"(?:CSM|Account Manager)\s*[:\|]\s*(.+?)(?:\n|\|)", "csm"),
    ]

    for pattern, key in kv_patterns:
        match = re.search(pattern, content, re.IGNORECASE)
        if match:
            data[key] = match.group(1).strip()

    # Extract Recent Wins section
    wins = _extract_section_items(content, "Recent Wins")
    if wins:
        data["recent_wins"] = wins[:5]

    # Extract Current Risks section
    risks = _extract_section_items(content, "Current Risks")
    if risks:
        data["current_risks"] = risks[:5]

    return data


def _extract_section_items(content: str, section_name: str) -> list[str]:
    """Extract bullet items from a markdown section."""
    # Find the section header
    pattern = rf"#+\s*{re.escape(section_name)}.*?\n((?:[\s\S]*?))(?=\n#|\Z)"
    match = re.search(pattern, content, re.IGNORECASE)
    if not match:
        return []

    section_text = match.group(1)
    items: list[str] = []
    for line in section_text.splitlines():
        stripped = line.strip()
        if stripped.startswith(("- ", "* ", "• ")):
            item = stripped.lstrip("-*• ").strip()
            if item:
                items.append(item)
    return items


# ---------------------------------------------------------------------------
# SQLite queries (Python reads DB directly since it's a file)
# ---------------------------------------------------------------------------

def _get_captures_for_account(
    db_path: Path,
    account_id: str,
    days_back: int = 14,
) -> list[dict[str, Any]]:
    """Query recent captures (wins/risks) for an account from SQLite."""
    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        cursor = conn.execute(
            """SELECT id, meeting_id, meeting_title, capture_type, content, captured_at
               FROM captures
               WHERE account_id = ?1
                 AND captured_at >= date('now', ?2)
               ORDER BY captured_at DESC""",
            (account_id, f"-{days_back} days"),
        )
        results = [
            {
                "id": row["id"],
                "meeting_title": row["meeting_title"],
                "type": row["capture_type"],
                "content": row["content"],
                "captured_at": row["captured_at"],
            }
            for row in cursor.fetchall()
        ]
        conn.close()
        return results
    except (sqlite3.Error, OSError):
        return []


def _get_account_actions(
    db_path: Path,
    account_id: str,
) -> list[dict[str, Any]]:
    """Query open actions for an account from SQLite."""
    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        cursor = conn.execute(
            """SELECT id, title, priority, status, due_date
               FROM actions
               WHERE account_id = ?1
                 AND status IN ('pending', 'waiting')
               ORDER BY priority, due_date""",
            (account_id,),
        )
        results = [
            {
                "id": row["id"],
                "title": row["title"],
                "priority": row["priority"],
                "status": row["status"],
                "due_date": row["due_date"],
            }
            for row in cursor.fetchall()
        ]
        conn.close()
        return results
    except (sqlite3.Error, OSError):
        return []


def _get_meeting_history(
    db_path: Path,
    account_id: str,
    lookback_days: int = 30,
    limit: int = 3,
) -> list[dict[str, Any]]:
    """Query recent meetings for an account from SQLite."""
    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        cursor = conn.execute(
            """SELECT id, title, meeting_type, start_time, summary
               FROM meetings_history
               WHERE account_id = ?1
                 AND start_time >= date('now', ?2)
               ORDER BY start_time DESC
               LIMIT ?3""",
            (account_id, f"-{lookback_days} days", limit),
        )
        results = [
            {
                "id": row["id"],
                "title": row["title"],
                "type": row["meeting_type"],
                "start_time": row["start_time"],
                "summary": row["summary"],
            }
            for row in cursor.fetchall()
        ]
        conn.close()
        return results
    except (sqlite3.Error, OSError):
        return []


# ---------------------------------------------------------------------------
# File search helpers (from prepare_today.py)
# ---------------------------------------------------------------------------

def _guess_account_name(
    meeting: dict[str, Any],
    accounts_dir: Path,
) -> str | None:
    """Try to match a meeting to a known account directory."""
    if not accounts_dir.is_dir():
        return None

    title_lower = meeting.get("title", "").lower()
    external_domains = meeting.get("external_domains", [])

    try:
        account_names = [d.name for d in accounts_dir.iterdir() if d.is_dir()]
    except OSError:
        return None

    for name in account_names:
        if name.lower() in title_lower:
            return name
        for domain in external_domains:
            domain_base = domain.split(".")[0].lower()
            if domain_base == name.lower() or name.lower() in domain_base:
                return name

    return None


def _find_file_in_dir(directory: Path, filename: str) -> Path | None:
    """Find a file by name in a directory (case-insensitive)."""
    if not directory.is_dir():
        return None

    exact = directory / filename
    if exact.exists():
        return exact

    target_lower = filename.lower()
    try:
        for item in directory.iterdir():
            if item.is_file() and item.name.lower() == target_lower:
                return item
    except OSError:
        pass

    # Search one level of subdirectories (e.g. 01-Customer-Information/)
    try:
        for subdir in directory.iterdir():
            if subdir.is_dir() and not subdir.name.startswith((".", "_")):
                for item in subdir.iterdir():
                    if item.is_file() and item.name.lower().endswith(target_lower):
                        return item
    except OSError:
        pass

    return None


def _find_recent_summaries(
    search_term: str,
    archive_dir: Path,
    limit: int = 2,
) -> list[Path]:
    """Find recent meeting summaries mentioning a search term."""
    if not archive_dir.is_dir():
        return []

    search_lower = search_term.lower()
    search_slug = re.sub(r"[^a-z0-9]+", "-", search_lower).strip("-")

    matches: list[tuple[float, Path]] = []

    try:
        date_dirs = sorted(
            [d for d in archive_dir.iterdir() if d.is_dir()],
            key=lambda d: d.name,
            reverse=True,
        )[:30]

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

    matches.sort(key=lambda x: x[0], reverse=True)
    return [m[1] for m in matches[:limit]]


def _search_archive(
    query: str,
    archive_dir: Path,
    max_results: int = 5,
    lookback_dirs: int = 14,
) -> list[Path]:
    """Search recent archive files for content matching a query."""
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
