"""Tests for deliver_today.py action deduplication (I23)."""

from __future__ import annotations

import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict
from unittest.mock import patch

import pytest

# Import the module under test
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
import deliver_today  # noqa: E402


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

def _minimal_directive(**overrides: Any) -> Dict[str, Any]:
    """Build a minimal directive with optional action overrides."""
    directive: Dict[str, Any] = {
        "context": {"date": "2025-02-07"},
        "calendar": {"events": []},
        "meetings": {},
        "actions": {},
    }
    directive.update(overrides)
    return directive


NOW = datetime(2025, 2, 7, 14, 0, 0, tzinfo=timezone.utc)


# ---------------------------------------------------------------------------
# _make_id is category-agnostic
# ---------------------------------------------------------------------------

class TestMakeIdCategoryAgnostic:
    """Verify that _make_id produces the same hash regardless of old category prefix."""

    def test_same_action_same_id_across_categories(self):
        """An action with identical title/account/due gets the same ID
        whether it appears as overdue, today, or week.  The dedup layer
        collapses them to one entry, proving the IDs matched."""
        directive = _minimal_directive(actions={
            "overdue": [{"title": "Renew contract", "account": "Acme", "due_date": "2025-02-01"}],
            "due_today": [{"title": "Renew contract", "account": "Acme", "due_date": "2025-02-01"}],
            "due_this_week": [{"title": "Renew contract", "account": "Acme", "due_date": "2025-02-01"}],
        })

        with patch.object(Path, "exists", return_value=False):
            result = deliver_today.build_actions(directive, NOW)

        # Dedup collapses all three into one (proves IDs matched)
        assert len(result["actions"]) == 1
        assert result["actions"][0]["id"].startswith("action-")
        # First occurrence wins: overdue -> isOverdue=True
        assert result["actions"][0]["isOverdue"] is True

    def test_id_prefix_is_action(self):
        """IDs should use the fixed 'action-' prefix, not category-based prefixes."""
        directive = _minimal_directive(actions={
            "overdue": [{"title": "Task A", "account": "X"}],
            "waiting_on": [{"what": "Reply", "who": "Jane"}],
        })

        with patch.object(Path, "exists", return_value=False):
            result = deliver_today.build_actions(directive, NOW)

        for action in result["actions"]:
            assert action["id"].startswith("action-"), f"Expected 'action-' prefix, got: {action['id']}"

    def test_different_actions_different_ids(self):
        """Different title/account/due combinations get different IDs."""
        directive = _minimal_directive(actions={
            "due_today": [
                {"title": "Task A", "account": "Acme", "due_date": "2025-02-07"},
                {"title": "Task B", "account": "Acme", "due_date": "2025-02-07"},
            ],
        })

        with patch.object(Path, "exists", return_value=False):
            result = deliver_today.build_actions(directive, NOW)

        ids = [a["id"] for a in result["actions"]]
        assert ids[0] != ids[1]


# ---------------------------------------------------------------------------
# Within-briefing dedup
# ---------------------------------------------------------------------------

class TestWithinBriefingDedup:
    """Verify that duplicate actions within the same briefing are deduplicated."""

    def test_dedup_removes_cross_category_duplicates(self):
        """Same action in overdue + due_today should appear only once."""
        directive = _minimal_directive(actions={
            "overdue": [{"title": "Send report", "account": "Acme", "due_date": "2025-02-01"}],
            "due_today": [{"title": "Send report", "account": "Acme", "due_date": "2025-02-01"}],
        })

        with patch.object(Path, "exists", return_value=False):
            result = deliver_today.build_actions(directive, NOW)

        assert len(result["actions"]) == 1
        # First occurrence wins (overdue -> P1, isOverdue=True)
        assert result["actions"][0]["isOverdue"] is True

    def test_dedup_preserves_distinct_actions(self):
        """Actions with different titles are not deduplicated."""
        directive = _minimal_directive(actions={
            "overdue": [{"title": "Task A", "account": "Acme"}],
            "due_today": [{"title": "Task B", "account": "Acme"}],
        })

        with patch.object(Path, "exists", return_value=False):
            result = deliver_today.build_actions(directive, NOW)

        assert len(result["actions"]) == 2

    def test_first_occurrence_wins_priority_order(self):
        """Overdue occurrence takes precedence over week occurrence."""
        directive = _minimal_directive(actions={
            "overdue": [{"title": "Review PR", "account": "Corp", "due_date": "2025-01-30"}],
            "due_this_week": [{"title": "Review PR", "account": "Corp", "due_date": "2025-01-30"}],
        })

        with patch.object(Path, "exists", return_value=False):
            result = deliver_today.build_actions(directive, NOW)

        assert len(result["actions"]) == 1
        assert result["actions"][0]["priority"] == "P1"
        assert result["actions"][0]["isOverdue"] is True


# ---------------------------------------------------------------------------
# SQLite pre-check
# ---------------------------------------------------------------------------

class TestSQLitePreCheck:
    """Verify that actions already in SQLite are skipped."""

    def _create_temp_db(self, titles: list[str]) -> Path:
        """Create a temporary SQLite DB with the given action titles."""
        tmp = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
        db_path = Path(tmp.name)
        tmp.close()

        conn = sqlite3.connect(str(db_path))
        conn.execute(
            "CREATE TABLE actions (id TEXT, title TEXT, status TEXT)"
        )
        for i, title in enumerate(titles):
            conn.execute(
                "INSERT INTO actions (id, title, status) VALUES (?, ?, ?)",
                (f"action-{i}", title, "pending"),
            )
        conn.commit()
        conn.close()
        return db_path

    def test_skips_existing_titles(self):
        """Actions whose title already exists in SQLite are not emitted."""
        db_path = self._create_temp_db(["Send report", "Follow up with client"])

        directive = _minimal_directive(actions={
            "due_today": [
                {"title": "Send report", "account": "Acme"},
                {"title": "New task", "account": "Acme"},
                {"title": "Follow up with client", "account": "Beta"},
            ],
        })

        with patch.object(Path, "home", return_value=db_path.parent.parent):
            # Patch so that Path.home() / ".dailyos" / "actions.db" resolves to our temp DB
            dailyos_dir = db_path.parent
            with patch.object(Path, "home", return_value=dailyos_dir.parent):
                # We need more precise control: patch the db_path construction
                pass

        # Direct approach: patch Path.home so ~/.dailyos/actions.db points to temp
        dailyos_dir = db_path.parent
        fake_home = dailyos_dir
        # Create the .dailyos directory structure
        dotdir = fake_home / ".dailyos"
        dotdir.mkdir(exist_ok=True)
        target_db = dotdir / "actions.db"
        # Copy the db
        import shutil
        shutil.copy2(str(db_path), str(target_db))

        with patch.object(Path, "home", return_value=fake_home):
            result = deliver_today.build_actions(directive, NOW)

        # Only "New task" should survive
        assert len(result["actions"]) == 1
        assert result["actions"][0]["title"] == "New task"

        # Cleanup
        target_db.unlink(missing_ok=True)
        dotdir.rmdir()
        db_path.unlink(missing_ok=True)

    def test_case_insensitive_title_match(self):
        """SQLite pre-check is case-insensitive."""
        db_path = self._create_temp_db(["send report"])

        fake_home = db_path.parent
        dotdir = fake_home / ".dailyos"
        dotdir.mkdir(exist_ok=True)
        target_db = dotdir / "actions.db"
        import shutil
        shutil.copy2(str(db_path), str(target_db))

        directive = _minimal_directive(actions={
            "due_today": [
                {"title": "Send Report", "account": "Acme"},
                {"title": "Other task", "account": "Acme"},
            ],
        })

        with patch.object(Path, "home", return_value=fake_home):
            result = deliver_today.build_actions(directive, NOW)

        assert len(result["actions"]) == 1
        assert result["actions"][0]["title"] == "Other task"

        # Cleanup
        target_db.unlink(missing_ok=True)
        dotdir.rmdir()
        db_path.unlink(missing_ok=True)

    def test_missing_db_is_graceful(self):
        """When actions.db doesn't exist, all actions pass through."""
        directive = _minimal_directive(actions={
            "due_today": [
                {"title": "Task A", "account": "Acme"},
                {"title": "Task B", "account": "Beta"},
            ],
        })

        # Point home to a temp dir with no .dailyos/actions.db
        with tempfile.TemporaryDirectory() as tmpdir:
            with patch.object(Path, "home", return_value=Path(tmpdir)):
                result = deliver_today.build_actions(directive, NOW)

        assert len(result["actions"]) == 2

    def test_waiting_on_skipped_by_precheck(self):
        """Waiting-on actions are also filtered by SQLite pre-check."""
        db_path = self._create_temp_db(["Waiting: Reply from vendor"])

        fake_home = db_path.parent
        dotdir = fake_home / ".dailyos"
        dotdir.mkdir(exist_ok=True)
        target_db = dotdir / "actions.db"
        import shutil
        shutil.copy2(str(db_path), str(target_db))

        directive = _minimal_directive(actions={
            "waiting_on": [
                {"what": "Reply from vendor", "who": "Jane"},
                {"what": "Approval from legal", "who": "Bob"},
            ],
        })

        with patch.object(Path, "home", return_value=fake_home):
            result = deliver_today.build_actions(directive, NOW)

        assert len(result["actions"]) == 1
        assert "Approval from legal" in result["actions"][0]["title"]

        # Cleanup
        target_db.unlink(missing_ok=True)
        dotdir.rmdir()
        db_path.unlink(missing_ok=True)
