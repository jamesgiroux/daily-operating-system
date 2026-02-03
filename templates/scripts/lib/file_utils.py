#!/usr/bin/env python3
"""
File utilities for daily operating system scripts.
Handles file operations, archival, and directory management.
"""

import os
import shutil
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Any

# Standard paths
VIP_ROOT = Path(__file__).parent.parent.parent
TODAY_DIR = VIP_ROOT / "_today"
ARCHIVE_DIR = TODAY_DIR / "archive"
TASKS_DIR = TODAY_DIR / "tasks"
AGENDA_DIR = TODAY_DIR / "90-agenda-needed"
INBOX_DIR = VIP_ROOT / "_inbox"
ACCOUNTS_DIR = VIP_ROOT / "Accounts"
PROJECTS_DIR = VIP_ROOT / "Projects"
LEADERSHIP_DIR = VIP_ROOT / "Leadership"


def ensure_today_structure() -> None:
    """
    Ensure the _today directory structure exists.
    """
    TODAY_DIR.mkdir(exist_ok=True)
    ARCHIVE_DIR.mkdir(exist_ok=True)
    TASKS_DIR.mkdir(exist_ok=True)
    AGENDA_DIR.mkdir(exist_ok=True)


def archive_daily_files(archive_date: datetime) -> List[Path]:
    """
    Archive daily files (NOT week-* files) to archive/YYYY-MM-DD/.

    Args:
        archive_date: Date for the archive folder

    Returns:
        List of archived file paths
    """
    date_str = archive_date.strftime('%Y-%m-%d')
    archive_path = ARCHIVE_DIR / date_str
    archive_path.mkdir(exist_ok=True)

    archived = []

    # Move all .md files in _today/ except week-* files
    for md_file in TODAY_DIR.glob('*.md'):
        if not md_file.name.startswith('week-'):
            dest = archive_path / md_file.name
            shutil.move(str(md_file), str(dest))
            archived.append(dest)

    # Move agenda-needed contents if any
    if AGENDA_DIR.exists():
        agenda_files = list(AGENDA_DIR.glob('*.md'))
        if agenda_files:
            agenda_archive = archive_path / '90-agenda-needed'
            agenda_archive.mkdir(exist_ok=True)
            for agenda_file in agenda_files:
                dest = agenda_archive / agenda_file.name
                shutil.move(str(agenda_file), str(dest))
                archived.append(dest)

    return archived


def archive_week_files(week_number: int) -> List[Path]:
    """
    Archive week-* files to archive/W[NN]/.

    Args:
        week_number: Week number for the archive folder

    Returns:
        List of archived file paths
    """
    week_str = f"W{week_number:02d}"
    archive_path = ARCHIVE_DIR / week_str
    archive_path.mkdir(exist_ok=True)

    archived = []

    # Move all week-* files
    for week_file in TODAY_DIR.glob('week-*.md'):
        dest = archive_path / week_file.name
        shutil.move(str(week_file), str(dest))
        archived.append(dest)

    return archived


def check_yesterday_archive(yesterday: datetime) -> bool:
    """
    Check if yesterday's files were archived.

    Args:
        yesterday: Yesterday's date

    Returns:
        True if archive exists, False otherwise
    """
    date_str = yesterday.strftime('%Y-%m-%d')
    archive_path = ARCHIVE_DIR / date_str
    return archive_path.exists()


def list_today_files() -> Dict[str, List[Path]]:
    """
    List all files in _today/ categorized by type.

    Returns:
        Dictionary with 'daily', 'week', 'agenda', 'tasks' lists
    """
    result = {
        'daily': [],
        'week': [],
        'agenda': [],
        'tasks': [],
    }

    # Daily files (non-week-* .md files)
    for md_file in TODAY_DIR.glob('*.md'):
        if md_file.name.startswith('week-'):
            result['week'].append(md_file)
        else:
            result['daily'].append(md_file)

    # Agenda files
    if AGENDA_DIR.exists():
        result['agenda'] = list(AGENDA_DIR.glob('*.md'))

    # Task files
    if TASKS_DIR.exists():
        result['tasks'] = list(TASKS_DIR.glob('*.md'))

    return result


def list_inbox_files() -> List[Path]:
    """
    List all files in _inbox/ that need processing.

    Returns:
        List of file paths
    """
    if not INBOX_DIR.exists():
        return []

    # Get markdown files, excluding system files
    files = [
        f for f in INBOX_DIR.glob('*.md')
        if not f.name.startswith('.')
        and not any(skip in f.name.lower() for skip in ['roadmap', 'prompt', 'architecture', 'test', 'checklist'])
    ]

    return sorted(files, key=lambda f: f.stat().st_mtime, reverse=True)


def count_inbox_pending() -> int:
    """
    Count the number of files pending in inbox.

    Returns:
        Number of pending files
    """
    return len(list_inbox_files())


def find_account_dashboard(account: str) -> Optional[Path]:
    """
    Find the dashboard file for an account.

    Args:
        account: Account name (supports "Parent / BU" format)

    Returns:
        Path to dashboard file or None
    """
    # Handle multi-BU format
    if ' / ' in account:
        parent, bu = account.split(' / ', 1)
        account_path = ACCOUNTS_DIR / parent / bu
    else:
        account_path = ACCOUNTS_DIR / account

    if not account_path.exists():
        return None

    # Look for dashboard file
    customer_info = account_path / '01-Customer-Information'
    if customer_info.exists():
        dashboards = list(customer_info.glob('*dashboard*.md'))
        if dashboards:
            return dashboards[0]

    return None


def find_recent_meeting_summaries(account: str, limit: int = 3) -> List[Path]:
    """
    Find recent meeting summaries for an account.

    Args:
        account: Account name
        limit: Maximum number to return

    Returns:
        List of meeting summary file paths (most recent first)
    """
    # Handle multi-BU format
    if ' / ' in account:
        parent, bu = account.split(' / ', 1)
        account_path = ACCOUNTS_DIR / parent / bu
    else:
        account_path = ACCOUNTS_DIR / account

    meetings_dir = account_path / '02-Meetings'
    if not meetings_dir.exists():
        return []

    # Get summaries sorted by modification time
    summaries = list(meetings_dir.glob('*.md'))
    summaries.sort(key=lambda f: f.stat().st_mtime, reverse=True)

    return summaries[:limit]


def find_account_action_file(account: str) -> Optional[Path]:
    """
    Find the current action items file for an account.

    Args:
        account: Account name

    Returns:
        Path to action file or None
    """
    # Handle multi-BU format
    if ' / ' in account:
        parent, bu = account.split(' / ', 1)
        account_path = ACCOUNTS_DIR / parent / bu
    else:
        account_path = ACCOUNTS_DIR / account

    actions_dir = account_path / '04-Action-Items'
    if not actions_dir.exists():
        return None

    # Look for current-actions.md first
    current = actions_dir / 'current-actions.md'
    if current.exists():
        return current

    # Fall back to most recent action file
    action_files = list(actions_dir.glob('*.md'))
    if action_files:
        action_files.sort(key=lambda f: f.stat().st_mtime, reverse=True)
        return action_files[0]

    return None


def find_project_index(project: str) -> Optional[Path]:
    """
    Find the index file for a project.

    Args:
        project: Project name

    Returns:
        Path to project index or None
    """
    project_path = PROJECTS_DIR / project

    if not project_path.exists():
        return None

    index = project_path / '00-Index.md'
    if index.exists():
        return index

    # Try alternative names
    for alt in ['README.md', 'index.md', 'overview.md']:
        alt_path = project_path / alt
        if alt_path.exists():
            return alt_path

    return None


def get_file_age_days(filepath: Path) -> int:
    """
    Get the age of a file in days since last modification.

    Args:
        filepath: Path to the file

    Returns:
        Age in days
    """
    if not filepath.exists():
        return -1

    mtime = datetime.fromtimestamp(filepath.stat().st_mtime)
    age = datetime.now() - mtime
    return age.days


def check_yesterday_transcripts(yesterday: datetime) -> List[Path]:
    """
    Check for unprocessed transcripts from yesterday.

    Args:
        yesterday: Yesterday's date

    Returns:
        List of unprocessed transcript file paths
    """
    date_str = yesterday.strftime('%Y-%m-%d')
    transcripts = [
        f for f in list_inbox_files()
        if date_str in f.name and 'transcript' in f.name.lower()
    ]
    return transcripts


def format_path_for_display(path: Path) -> str:
    """
    Format a path for display relative to workspace root.

    Args:
        path: Full path

    Returns:
        Relative path string
    """
    try:
        return str(path.relative_to(VIP_ROOT))
    except ValueError:
        return str(path)
