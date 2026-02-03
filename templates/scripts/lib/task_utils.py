#!/usr/bin/env python3
"""
Task utilities for daily operating system scripts.
Handles master task list parsing, action item aggregation, and task management.
"""

import os
import re
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple

# Standard paths
VIP_ROOT = Path(__file__).parent.parent.parent
MASTER_TASK_LIST = VIP_ROOT / "_today/tasks/master-task-list.md"
ACCOUNTS_DIR = VIP_ROOT / "Accounts"


def load_master_task_list() -> Dict[str, Any]:
    """
    Load and parse the master task list.

    Returns:
        Dictionary with parsed tasks categorized by status
    """
    if not MASTER_TASK_LIST.exists():
        return {'tasks': [], 'sections': {}}

    content = MASTER_TASK_LIST.read_text()
    return parse_task_list_content(content)


def parse_task_list_content(content: str) -> Dict[str, Any]:
    """
    Parse task list markdown content into structured data.

    Args:
        content: Markdown content of task list

    Returns:
        Dictionary with 'tasks' list and 'sections' dict
    """
    tasks = []
    sections = {}
    current_section = 'unsectioned'

    lines = content.split('\n')
    i = 0

    while i < len(lines):
        line = lines[i]

        # Detect section headers
        if line.startswith('## '):
            current_section = line[3:].strip()
            sections[current_section] = []
            i += 1
            continue

        # Detect task items (checkbox pattern)
        checkbox_match = re.match(r'^- \[([ xX])\] (.+)$', line)
        if checkbox_match:
            completed = checkbox_match.group(1).lower() == 'x'
            task_text = checkbox_match.group(2)

            # Parse task metadata from text and following lines
            task = parse_task_line(task_text, completed)
            task['section'] = current_section

            # Look for indented metadata lines
            i += 1
            while i < len(lines) and lines[i].startswith('  - '):
                metadata_line = lines[i][4:].strip()
                parse_task_metadata(task, metadata_line)
                i += 1

            tasks.append(task)
            if current_section in sections:
                sections[current_section].append(task)
            continue

        i += 1

    return {'tasks': tasks, 'sections': sections}


def parse_task_line(text: str, completed: bool) -> Dict[str, Any]:
    """
    Parse a task line to extract structured information.

    Args:
        text: The task text after the checkbox
        completed: Whether the task is checked

    Returns:
        Task dictionary with extracted fields
    """
    task = {
        'text': text,
        'completed': completed,
        'account': None,
        'owner': None,
        'due': None,
        'priority': None,
        'source': None,
        'id': None,
    }

    # Extract bold task title
    bold_match = re.search(r'\*\*(.+?)\*\*', text)
    if bold_match:
        task['title'] = bold_match.group(1)
    else:
        task['title'] = text

    # Extract task ID (backtick pattern like `2026-01-12-agenda-001`)
    id_match = re.search(r'`([^`]+)`', text)
    if id_match:
        task['id'] = id_match.group(1)

    return task


def parse_task_metadata(task: Dict, line: str) -> None:
    """
    Parse a task metadata line and update the task dict.

    Args:
        task: Task dictionary to update
        line: Metadata line (e.g., "Account: Acme Corp")
    """
    if ':' not in line:
        return

    key, value = line.split(':', 1)
    key = key.strip().lower()
    value = value.strip()

    if key == 'account':
        task['account'] = value
    elif key == 'owner':
        task['owner'] = value
    elif key == 'due':
        task['due'] = parse_due_date(value)
        task['due_raw'] = value
    elif key == 'priority':
        task['priority'] = value
    elif key == 'source':
        task['source'] = value
    elif key == 'context':
        task['context'] = value
    elif key == 'requested by':
        task['requested_by'] = value


def parse_due_date(date_str: str) -> Optional[datetime]:
    """
    Parse a due date string into a datetime.

    Args:
        date_str: Date string (various formats)

    Returns:
        datetime object or None
    """
    # Remove "(X days overdue)" or similar suffixes
    date_str = re.sub(r'\s*\([^)]+\)\s*$', '', date_str).strip()

    # Try various formats
    formats = [
        '%Y-%m-%d',
        '%B %d, %Y',
        '%B %d',
        '%b %d, %Y',
        '%b %d',
    ]

    for fmt in formats:
        try:
            parsed = datetime.strptime(date_str, fmt)
            # If no year in format, assume current year
            if parsed.year == 1900:
                parsed = parsed.replace(year=datetime.now().year)
            return parsed
        except ValueError:
            continue

    # Handle relative dates
    lower = date_str.lower()
    today = datetime.now().replace(hour=0, minute=0, second=0, microsecond=0)

    if lower == 'today':
        return today
    elif lower == 'tomorrow':
        return today + timedelta(days=1)
    elif lower == 'yesterday':
        return today - timedelta(days=1)

    return None


def get_tasks_due_on(tasks: List[Dict], target_date: datetime) -> List[Dict]:
    """
    Get tasks due on a specific date.

    Args:
        tasks: List of task dictionaries
        target_date: Date to filter for

    Returns:
        List of tasks due on that date
    """
    target = target_date.replace(hour=0, minute=0, second=0, microsecond=0)
    return [t for t in tasks if t.get('due') and t['due'].date() == target.date()]


def get_overdue_tasks(tasks: List[Dict], reference_date: datetime = None) -> List[Dict]:
    """
    Get tasks that are overdue.

    Args:
        tasks: List of task dictionaries
        reference_date: Date to compare against (default: today)

    Returns:
        List of overdue tasks sorted by due date
    """
    if reference_date is None:
        reference_date = datetime.now()

    reference = reference_date.replace(hour=0, minute=0, second=0, microsecond=0)

    overdue = [
        t for t in tasks
        if t.get('due') and t['due'].date() < reference.date() and not t.get('completed')
    ]

    return sorted(overdue, key=lambda t: t['due'])


def get_tasks_for_week(tasks: List[Dict], monday: datetime) -> List[Dict]:
    """
    Get tasks due during a specific week (Mon-Fri).

    Args:
        tasks: List of task dictionaries
        monday: Monday of the target week

    Returns:
        List of tasks due that week
    """
    friday = monday + timedelta(days=4)
    return [
        t for t in tasks
        if t.get('due') and monday.date() <= t['due'].date() <= friday.date() and not t.get('completed')
    ]


def get_tasks_for_accounts(tasks: List[Dict], accounts: List[str]) -> List[Dict]:
    """
    Get tasks related to specific accounts.

    Args:
        tasks: List of task dictionaries
        accounts: List of account names to filter for

    Returns:
        List of matching tasks
    """
    account_set = set(a.lower() for a in accounts)
    return [
        t for t in tasks
        if t.get('account') and t['account'].lower() in account_set
    ]


def scan_account_action_files() -> List[Dict]:
    """
    Scan all account action files for uncompleted tasks.

    Returns:
        List of task dictionaries from account files
    """
    tasks = []

    if not ACCOUNTS_DIR.exists():
        return tasks

    # Find all action files
    action_files = list(ACCOUNTS_DIR.glob('*/04-Action-Items/*.md'))
    action_files.extend(ACCOUNTS_DIR.glob('*/*/04-Action-Items/*.md'))  # Multi-BU

    for action_file in action_files:
        # Extract account name from path
        parts = action_file.relative_to(ACCOUNTS_DIR).parts
        if len(parts) >= 2:
            # Could be "Account/04-Action-Items" or "Parent/BU/04-Action-Items"
            if parts[1] == '04-Action-Items':
                account = parts[0]
            elif len(parts) >= 3 and parts[2] == '04-Action-Items':
                account = f"{parts[0]} / {parts[1]}"
            else:
                account = parts[0]
        else:
            account = 'Unknown'

        try:
            content = action_file.read_text()
            file_tasks = parse_task_list_content(content)

            # Tag tasks with account and source
            for task in file_tasks['tasks']:
                if not task.get('completed'):
                    task['account'] = task.get('account') or account
                    task['source_file'] = str(action_file)
                    tasks.append(task)
        except Exception as e:
            print(f"Warning: Failed to parse {action_file}: {e}")

    return tasks


def filter_tasks_by_owner(tasks: List[Dict], owner: str = 'james') -> List[Dict]:
    """
    Filter tasks to only those owned by a specific person.

    Args:
        tasks: List of task dictionaries
        owner: Owner name to filter for (case-insensitive)

    Returns:
        List of tasks owned by that person (includes unassigned)
    """
    owner_lower = owner.lower()
    return [
        t for t in tasks
        if not t.get('owner') or owner_lower in t['owner'].lower() or t['owner'].lower() == 'unassigned'
    ]


def calculate_days_overdue(due_date: datetime, reference: datetime = None) -> int:
    """
    Calculate how many days a task is overdue.

    Args:
        due_date: Task due date
        reference: Reference date (default: today)

    Returns:
        Number of days overdue (negative if not yet due)
    """
    if reference is None:
        reference = datetime.now()

    delta = reference.date() - due_date.date()
    return delta.days


def format_task_for_directive(task: Dict) -> Dict[str, Any]:
    """
    Format a task for inclusion in a JSON directive.

    Args:
        task: Task dictionary

    Returns:
        Serializable task dictionary
    """
    formatted = {
        'title': task.get('title', task.get('text', '')),
        'completed': task.get('completed', False),
        'account': task.get('account'),
        'owner': task.get('owner'),
        'priority': task.get('priority'),
        'source': task.get('source'),
        'context': task.get('context'),
    }

    # Handle datetime serialization
    if task.get('due'):
        formatted['due'] = task['due'].strftime('%Y-%m-%d')
        formatted['days_overdue'] = calculate_days_overdue(task['due'])
    else:
        formatted['due'] = None
        formatted['days_overdue'] = None

    return formatted


def extract_waiting_on() -> List[Dict[str, Any]]:
    """
    Extract Waiting On (Delegated) items from master task list.

    Parses the table in the "Waiting On (Delegated)" section with columns:
    | Who | What | Asked | Days | Context |

    Returns:
        List of waiting-on item dictionaries
    """
    if not MASTER_TASK_LIST.exists():
        return []

    content = MASTER_TASK_LIST.read_text()
    waiting_on = []
    in_waiting_section = False

    for line in content.split('\n'):
        # Detect section start
        if '## Waiting On (Delegated)' in line:
            in_waiting_section = True
            continue

        # Detect section end (next H2)
        if in_waiting_section and line.startswith('## '):
            break

        # Parse table rows (skip header and separator)
        if in_waiting_section and '|' in line:
            # Skip header row and separator row
            if line.startswith('|--') or '| Who |' in line or '|-----|' in line:
                continue

            parts = [p.strip() for p in line.split('|')]
            # Remove empty first/last elements from split
            parts = [p for p in parts if p]

            if len(parts) >= 4:
                who = parts[0]
                what = parts[1]
                asked = parts[2]
                days = parts[3]
                context = parts[4] if len(parts) >= 5 else ''

                # Skip if 'Who' column is empty or header-like
                if who and who not in ['Who', '-', '']:
                    waiting_on.append({
                        'who': who,
                        'what': what,
                        'asked': asked,
                        'days': days,
                        'context': context
                    })

    return waiting_on
