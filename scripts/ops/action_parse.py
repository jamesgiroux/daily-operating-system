"""Action parsing from workspace markdown + SQLite state merge.

Extracted from prepare_today.py and prepare_week.py per ADR-0030.
Addresses I23: pre-checks SQLite before extracting from markdown
to avoid re-extracting completed actions.
"""

from __future__ import annotations

import re
import sqlite3
from dataclasses import dataclass, field
from datetime import date, timedelta
from pathlib import Path
from typing import Any


# Action line patterns
_CHECKBOX_RE = re.compile(r"^\s*-\s*\[\s*\]\s*(.+)$")
_DUE_RE = re.compile(r"due[:\s]+(\d{4}-\d{2}-\d{2})", re.IGNORECASE)
_PRIORITY_RE = re.compile(r"\b(P[123])\b", re.IGNORECASE)
_ACCOUNT_RE = re.compile(r"@(\S+)")
_CONTEXT_RE = re.compile(r"#(\S+)")
_WAITING_RE = re.compile(r"\b(waiting|blocked|pending)\b", re.IGNORECASE)


@dataclass
class ActionResult:
    """Result of parsing workspace actions."""
    overdue: list[dict[str, Any]] = field(default_factory=list)
    due_today: list[dict[str, Any]] = field(default_factory=list)
    due_this_week: list[dict[str, Any]] = field(default_factory=list)
    waiting_on: list[dict[str, Any]] = field(default_factory=list)

    def to_dict(self) -> dict[str, list[dict[str, Any]]]:
        """Serialize to directive-compatible dict."""
        return {
            "overdue": self.overdue,
            "due_today": self.due_today,
            "due_this_week": self.due_this_week,
            "waiting_on": self.waiting_on,
        }


def parse_workspace_actions(
    workspace: Path,
    db_path: Path | None = None,
) -> ActionResult:
    """Parse actions from workspace markdown + merge SQLite state.

    Addresses I23: pre-checks SQLite before extracting from markdown
    to avoid re-extracting completed actions.

    Args:
        workspace: Path to workspace root.
        db_path: Path to SQLite database. Defaults to ~/.dailyos/actions.db.

    Returns:
        ActionResult with categorized actions.
    """
    result = ActionResult()

    # Load existing action titles from SQLite (I23 pre-check)
    existing_titles: set[str] = set()
    if db_path is None:
        db_path = Path.home() / ".dailyos" / "actions.db"
    if db_path.exists():
        existing_titles = _load_existing_titles(db_path)

    # Find actions.md
    actions_path = workspace / "actions.md"
    if not actions_path.exists():
        actions_path = workspace / "_today" / "actions.md"
        if not actions_path.exists():
            return result

    try:
        content = actions_path.read_text(encoding="utf-8")
    except OSError:
        return result

    today = date.today()
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

        # I23: Skip if this action already exists in SQLite
        if title.lower().strip() in existing_titles:
            continue

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
            result.waiting_on.append(action)
            continue

        # Categorize by due date
        if due_date is not None:
            if due_date < today:
                days_overdue = (today - due_date).days
                action["days_overdue"] = days_overdue
                result.overdue.append(action)
            elif due_date == today:
                result.due_today.append(action)
            elif monday <= due_date <= friday:
                result.due_this_week.append(action)
        else:
            # No due date: treat as due_this_week (low priority)
            result.due_this_week.append(action)

    return result


def fetch_actions_from_db(
    workspace: Path,
    monday: date,
    friday: date,
    db_path: Path | None = None,
) -> dict[str, list[dict[str, Any]]]:
    """Read overdue and this-week actions directly from SQLite.

    Used by the /week orchestrator which reads from SQLite rather than
    parsing markdown (the week directive was always SQLite-based).

    Returns {"overdue": [...], "thisWeek": [...]}.
    """
    result: dict[str, list[dict[str, Any]]] = {
        "overdue": [],
        "thisWeek": [],
    }

    if db_path is None:
        # Week script used workspace-local DB path
        db_path = workspace / "_today" / "data" / "dailyos.db"
    if not db_path.exists():
        return result

    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
    except sqlite3.Error:
        return result

    # Check if table exists
    cursor = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='actions'",
    )
    if cursor.fetchone() is None:
        conn.close()
        return result

    today_str = date.today().isoformat()
    monday_str = monday.isoformat()
    friday_str = friday.isoformat()

    try:
        # Overdue
        cursor = conn.execute(
            """SELECT id, title, priority, status, due_date, account_id
               FROM actions
               WHERE status != 'completed'
                 AND due_date IS NOT NULL
                 AND due_date < ?
               ORDER BY due_date ASC""",
            (today_str,),
        )
        for row in cursor.fetchall():
            due = row["due_date"]
            days_overdue = (date.today() - date.fromisoformat(due)).days if due else 0
            result["overdue"].append({
                "id": row["id"],
                "title": row["title"],
                "priority": row["priority"],
                "status": row["status"],
                "dueDate": due,
                "accountId": row["account_id"],
                "daysOverdue": days_overdue,
            })

        # This week
        cursor = conn.execute(
            """SELECT id, title, priority, status, due_date, account_id
               FROM actions
               WHERE status != 'completed'
                 AND due_date IS NOT NULL
                 AND due_date >= ?
                 AND due_date <= ?
               ORDER BY due_date ASC""",
            (monday_str, friday_str),
        )
        for row in cursor.fetchall():
            result["thisWeek"].append({
                "id": row["id"],
                "title": row["title"],
                "priority": row["priority"],
                "status": row["status"],
                "dueDate": row["due_date"],
                "accountId": row["account_id"],
            })
    except sqlite3.Error:
        pass
    finally:
        conn.close()

    return result


# ---------------------------------------------------------------------------
# SQLite pre-check (I23)
# ---------------------------------------------------------------------------

def _load_existing_titles(db_path: Path) -> set[str]:
    """Load all existing action titles from SQLite for dedup pre-check.

    Returns a set of lowercased, stripped titles.
    """
    titles: set[str] = set()
    try:
        conn = sqlite3.connect(str(db_path))
        cursor = conn.execute("SELECT LOWER(TRIM(title)) FROM actions")
        for row in cursor.fetchall():
            if row[0]:
                titles.add(row[0])
        conn.close()
    except (sqlite3.Error, OSError):
        pass
    return titles
