"""Tests for meeting_prep.py context gathering across all meeting types."""

from __future__ import annotations

import sqlite3
import tempfile
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any

import pytest

import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
from ops.meeting_prep import gather_meeting_context, MeetingContext  # noqa: E402


# Use dates relative to today so SQLite date('now', '-N days') comparisons work
_TODAY = datetime.now()
_YESTERDAY = (_TODAY - timedelta(days=1)).strftime("%Y-%m-%d")
_LAST_WEEK = (_TODAY - timedelta(days=7)).strftime("%Y-%m-%d")
_TWO_WEEKS_AGO = (_TODAY - timedelta(days=14)).strftime("%Y-%m-%d")
_LAST_MONTH = (_TODAY - timedelta(days=30)).strftime("%Y-%m-%d")
_TWO_MONTHS_AGO = (_TODAY - timedelta(days=55)).strftime("%Y-%m-%d")


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

def _make_meeting(**overrides: Any) -> dict[str, Any]:
    """Build a minimal meeting dict with optional overrides."""
    meeting: dict[str, Any] = {
        "id": "evt-001",
        "title": "Weekly Sync",
        "start": "2025-02-07T10:00:00Z",
        "type": "team_sync",
    }
    meeting.update(overrides)
    return meeting


def _create_test_db(tmp_dir: Path) -> Path:
    """Create a SQLite DB with meetings_history, captures, and actions tables."""
    db_path = tmp_dir / "actions.db"
    conn = sqlite3.connect(str(db_path))
    conn.executescript("""
        CREATE TABLE meetings_history (
            id TEXT, title TEXT, meeting_type TEXT,
            start_time TEXT, summary TEXT, account_id TEXT
        );
        CREATE TABLE captures (
            id TEXT, meeting_id TEXT, meeting_title TEXT,
            capture_type TEXT, content TEXT, captured_at TEXT, account_id TEXT
        );
        CREATE TABLE actions (
            id TEXT, title TEXT, priority TEXT,
            status TEXT, due_date TEXT, account_id TEXT
        );
    """)
    conn.commit()
    conn.close()
    return db_path


def _seed_meeting_history(db_path: Path, rows: list[tuple]) -> None:
    conn = sqlite3.connect(str(db_path))
    conn.executemany(
        "INSERT INTO meetings_history (id, title, meeting_type, start_time, summary, account_id) "
        "VALUES (?, ?, ?, ?, ?, ?)",
        rows,
    )
    conn.commit()
    conn.close()


def _seed_captures(db_path: Path, rows: list[tuple]) -> None:
    conn = sqlite3.connect(str(db_path))
    conn.executemany(
        "INSERT INTO captures (id, meeting_id, meeting_title, capture_type, content, captured_at, account_id) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        rows,
    )
    conn.commit()
    conn.close()


def _seed_actions(db_path: Path, rows: list[tuple]) -> None:
    conn = sqlite3.connect(str(db_path))
    conn.executemany(
        "INSERT INTO actions (id, title, priority, status, due_date, account_id) "
        "VALUES (?, ?, ?, ?, ?, ?)",
        rows,
    )
    conn.commit()
    conn.close()


@pytest.fixture
def workspace(tmp_path: Path) -> Path:
    """Create a minimal workspace directory structure."""
    (tmp_path / "_archive").mkdir()
    (tmp_path / "_inbox").mkdir()
    (tmp_path / "Accounts").mkdir()
    return tmp_path


# ---------------------------------------------------------------------------
# Customer meeting: existing behavior preserved
# ---------------------------------------------------------------------------

class TestCustomerMeeting:
    def test_customer_gets_account_data(self, workspace: Path):
        """Customer meetings with a matching account dir get file refs and SQLite data."""
        # Set up account directory
        acme = workspace / "Accounts" / "Acme"
        acme.mkdir(parents=True)
        (acme / "dashboard.md").write_text("# Acme Dashboard\n## Quick View\nARR: $500K\n")

        # Set up archive with a matching file
        date_dir = workspace / "_archive" / _YESTERDAY
        date_dir.mkdir(parents=True)
        (date_dir / "acme-weekly-sync.md").write_text("# Acme Weekly Sync\nNotes here")

        # Set up DB
        db_path = _create_test_db(workspace)
        _seed_actions(db_path, [
            ("a1", "Follow up on renewal", "P1", "pending", _YESTERDAY, "Acme"),
        ])
        _seed_meeting_history(db_path, [
            ("m1", "Acme Weekly Sync", "customer", _LAST_WEEK, "Discussed renewal", "Acme"),
        ])

        meeting = _make_meeting(type="customer", title="Acme Weekly Sync")
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)

        assert ctx.account == "Acme"
        assert "account_dashboard" in ctx.refs
        # Regex captures after the $ sign, so "500K" not "$500K"
        assert ctx.account_data.get("arr") == "500K"
        assert len(ctx.open_actions) == 1
        assert len(ctx.meeting_history) == 1


# ---------------------------------------------------------------------------
# Team sync: meeting_history by title, open_actions, archive ref
# ---------------------------------------------------------------------------

class TestTeamSync:
    def test_team_sync_gets_title_based_context(self, workspace: Path):
        """Team sync meetings get meeting history by title and pending actions."""
        # Archive file matching the meeting title
        date_dir = workspace / "_archive" / _YESTERDAY
        date_dir.mkdir(parents=True)
        (date_dir / "weekly-sync.md").write_text("# Weekly Sync\nLast week's notes")

        # DB with title-matched history and actions
        db_path = _create_test_db(workspace)
        _seed_meeting_history(db_path, [
            ("m1", "Weekly Sync", "team_sync", _LAST_WEEK, "Status updates", None),
            ("m2", "Weekly Sync", "team_sync", _TWO_WEEKS_AGO, "Planning session", None),
        ])
        _seed_captures(db_path, [
            ("c1", "m1", "Weekly Sync", "decision", "Decided to ship v2", _LAST_WEEK, None),
        ])
        _seed_actions(db_path, [
            ("a1", "Update roadmap doc", "P2", "pending", _YESTERDAY, None),
            ("a2", "Review PR #42", "P1", "pending", _YESTERDAY, None),
            ("a3", "Done task", "P3", "completed", _LAST_WEEK, None),
        ])

        meeting = _make_meeting(type="team_sync", title="Weekly Sync")
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)

        assert "last_meeting" in ctx.refs
        assert len(ctx.meeting_history) == 2
        assert len(ctx.recent_captures) == 1
        # Only pending/waiting actions (not completed)
        assert len(ctx.open_actions) == 2

    def test_team_sync_no_title_still_gets_actions(self, workspace: Path):
        """Team sync without a title still gets pending actions."""
        db_path = _create_test_db(workspace)
        _seed_actions(db_path, [
            ("a1", "Some task", "P1", "pending", _YESTERDAY, None),
        ])

        meeting = _make_meeting(type="team_sync", title="")
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)

        assert len(ctx.meeting_history) == 0
        assert len(ctx.open_actions) == 1


# ---------------------------------------------------------------------------
# 1:1: 3 archive refs, 60-day meeting history, captures by title
# ---------------------------------------------------------------------------

class TestOneOnOne:
    def test_one_on_one_gets_deep_lookback(self, workspace: Path):
        """1:1 meetings get up to 3 archive refs and 60-day history."""
        # Multiple archive files
        days = [_YESTERDAY, _LAST_WEEK, _TWO_WEEKS_AGO, _LAST_MONTH]
        for day in days:
            date_dir = workspace / "_archive" / day
            date_dir.mkdir(parents=True, exist_ok=True)
            (date_dir / "jane-1-1.md").write_text(f"# Jane 1:1\nNotes from {day}")

        # DB with history (all within 60-day window)
        db_path = _create_test_db(workspace)
        _seed_meeting_history(db_path, [
            ("m1", "Jane 1:1", "one_on_one", _LAST_WEEK, "Career chat", None),
            ("m2", "Jane 1:1", "one_on_one", _TWO_WEEKS_AGO, "Sprint retro", None),
            ("m3", "Jane 1:1", "one_on_one", _TWO_MONTHS_AGO, "End of year", None),
        ])
        _seed_captures(db_path, [
            ("c1", "m1", "Jane 1:1", "action", "Jane to update OKRs", _LAST_WEEK, None),
        ])
        _seed_actions(db_path, [
            ("a1", "Prepare promo doc", "P1", "pending", _YESTERDAY, None),
        ])

        meeting = _make_meeting(type="one_on_one", title="Jane 1:1")
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)

        # Up to 3 archive refs
        assert "recent_meetings" in ctx.refs
        assert len(ctx.refs["recent_meetings"]) == 3
        # Meeting history from SQLite (60-day window)
        assert len(ctx.meeting_history) == 3
        assert len(ctx.recent_captures) == 1
        assert len(ctx.open_actions) == 1

    def test_one_on_one_serialization(self, workspace: Path):
        """1:1 context serializes recent_meetings as a list in refs."""
        date_dir = workspace / "_archive" / _YESTERDAY
        date_dir.mkdir(parents=True)
        (date_dir / "bob-1-1.md").write_text("# Bob 1:1")

        db_path = _create_test_db(workspace)

        meeting = _make_meeting(type="one_on_one", title="Bob 1:1")
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)
        d = ctx.to_dict()

        assert d["type"] == "one_on_one"
        assert "recent_meetings" in d["refs"]
        assert isinstance(d["refs"]["recent_meetings"], list)


# ---------------------------------------------------------------------------
# Partnership: with and without known account
# ---------------------------------------------------------------------------

class TestPartnership:
    def test_partnership_with_known_account(self, workspace: Path):
        """Partnership with matching account dir gets account-based lookups."""
        partner = workspace / "Accounts" / "PartnerCo"
        partner.mkdir(parents=True)
        (partner / "dashboard.md").write_text("# PartnerCo\nPartner dashboard")
        (partner / "stakeholders.md").write_text("# Stakeholders\n- Alice")

        db_path = _create_test_db(workspace)
        _seed_actions(db_path, [
            ("a1", "Send joint proposal", "P1", "pending", _YESTERDAY, "PartnerCo"),
        ])
        _seed_meeting_history(db_path, [
            ("m1", "PartnerCo Sync", "partnership", _LAST_WEEK, "Joint roadmap", "PartnerCo"),
        ])

        meeting = _make_meeting(
            type="partnership",
            title="PartnerCo Sync",
        )
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)

        assert ctx.account == "PartnerCo"
        assert "dashboard" in ctx.refs
        assert "stakeholders" in ctx.refs
        assert len(ctx.open_actions) == 1
        assert len(ctx.meeting_history) == 1

    def test_partnership_unknown_account_falls_back_to_title(self, workspace: Path):
        """Partnership without account match uses title-based SQLite queries."""
        db_path = _create_test_db(workspace)
        _seed_meeting_history(db_path, [
            ("m1", "Vendor X Quarterly", "partnership", _LAST_WEEK, "Review", None),
        ])
        _seed_captures(db_path, [
            ("c1", "m1", "Vendor X Quarterly", "risk", "Budget concern", _LAST_WEEK, None),
        ])

        meeting = _make_meeting(type="partnership", title="Vendor X Quarterly")
        ctx = gather_meeting_context(meeting, workspace, db_path=db_path)

        assert ctx.account is None
        assert len(ctx.meeting_history) == 1
        assert len(ctx.recent_captures) == 1


# ---------------------------------------------------------------------------
# Personal / All Hands: skip (empty context)
# ---------------------------------------------------------------------------

class TestSkipMeetingTypes:
    def test_personal_returns_empty_context(self, workspace: Path):
        meeting = _make_meeting(type="personal", title="Lunch")
        ctx = gather_meeting_context(meeting, workspace)
        d = ctx.to_dict()

        assert d["refs"] == {}
        assert "open_actions" not in d
        assert "meeting_history" not in d

    def test_all_hands_returns_empty_context(self, workspace: Path):
        meeting = _make_meeting(type="all_hands", title="Company All Hands")
        ctx = gather_meeting_context(meeting, workspace)
        d = ctx.to_dict()

        assert d["refs"] == {}
        assert "open_actions" not in d


# ---------------------------------------------------------------------------
# Graceful degradation: missing DB
# ---------------------------------------------------------------------------

class TestMissingDb:
    def test_team_sync_without_db_still_gets_archive(self, workspace: Path):
        """When SQLite doesn't exist, archive refs still work."""
        date_dir = workspace / "_archive" / "2025-02-06"
        date_dir.mkdir(parents=True)
        (date_dir / "weekly-sync.md").write_text("# Weekly Sync notes")

        # Point to a nonexistent DB
        fake_db = workspace / "nonexistent.db"

        meeting = _make_meeting(type="team_sync", title="Weekly Sync")
        ctx = gather_meeting_context(meeting, workspace, db_path=fake_db)

        assert "last_meeting" in ctx.refs
        assert len(ctx.meeting_history) == 0
        assert len(ctx.open_actions) == 0

    def test_one_on_one_without_db_still_gets_archive(self, workspace: Path):
        """1:1 without DB still returns archive refs."""
        date_dir = workspace / "_archive" / "2025-02-06"
        date_dir.mkdir(parents=True)
        (date_dir / "jane-1-1.md").write_text("# Jane 1:1")

        fake_db = workspace / "nonexistent.db"

        meeting = _make_meeting(type="one_on_one", title="Jane 1:1")
        ctx = gather_meeting_context(meeting, workspace, db_path=fake_db)

        assert "recent_meetings" in ctx.refs
        assert len(ctx.meeting_history) == 0
