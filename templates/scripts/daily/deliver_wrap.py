#!/usr/bin/env python3
"""
Phase 3: Wrap Delivery Script
Handles post-AI delivery operations for /wrap command.

After Claude has executed AI tasks (Phase 2), this script:
1. Reads enriched directive with AI outputs
2. Archives today's files to archive/YYYY-MM-DD/
3. Updates week overview prep status
4. Updates master task list with status changes
5. Writes wrap summary file
6. Generates completion output

Usage:
    python3 _tools/deliver_wrap.py [--directive FILE] [--skip-archive]
"""

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent / 'lib'))

from file_utils import (
    archive_daily_files, list_today_files, ensure_today_structure,
    TODAY_DIR, ARCHIVE_DIR, VIP_ROOT
)

# Paths
DIRECTIVE_FILE = TODAY_DIR / ".wrap-directive.json"


def load_directive(path: Path) -> Optional[Dict[str, Any]]:
    """
    Load the directive file.

    Args:
        path: Path to directive JSON file

    Returns:
        Directive dictionary or None if failed
    """
    if not path.exists():
        print(f"Error: Directive file not found: {path}", file=sys.stderr)
        return None

    try:
        with open(path) as f:
            return json.load(f)
    except Exception as e:
        print(f"Error: Failed to load directive: {e}", file=sys.stderr)
        return None


def update_week_overview_prep_status(reconciliation: List[Dict]) -> bool:
    """
    Update week overview with completed meeting prep status.

    Args:
        reconciliation: List of prep reconciliation items

    Returns:
        True if updated, False otherwise
    """
    week_overview = TODAY_DIR / "week-00-overview.md"

    if not week_overview.exists():
        return False

    content = week_overview.read_text()

    for recon in reconciliation:
        account = recon.get('account', '')
        new_status = recon.get('new_status', '✅ Done')

        if not account:
            continue

        # Update any line containing the account with new status
        # This is simplified - production code should be more robust
        lines = content.split('\n')
        for i, line in enumerate(lines):
            if account in line and '|' in line:
                # Find and replace prep status in this row
                cells = line.split('|')
                for j, cell in enumerate(cells):
                    # Look for status indicators
                    if any(status in cell for status in ['📋', '📅', '✏️', '👥', '🔄']):
                        cells[j] = f' {new_status} '
                        break
                lines[i] = '|'.join(cells)
                break

        content = '\n'.join(lines)

    week_overview.write_text(content)
    return True


def update_master_task_list(task_updates: List[Dict]) -> int:
    """
    Update master task list with task status changes.

    Args:
        task_updates: List of task update dictionaries

    Returns:
        Number of tasks updated
    """
    master_list = TODAY_DIR / "tasks/master-task-list.md"

    if not master_list.exists():
        return 0

    content = master_list.read_text()
    updates_made = 0

    for update in task_updates:
        title = update.get('title', '')
        new_status = update.get('new_status', '')

        if not title or not new_status:
            continue

        # Update the task line
        if new_status.lower() == 'completed':
            # Change [ ] to [x]
            old_pattern = f"- [ ] **{title}**"
            new_pattern = f"- [x] **{title}**"
            if old_pattern in content:
                content = content.replace(old_pattern, new_pattern, 1)
                updates_made += 1

        # Add completion date comment
        # This is simplified - production code would be more sophisticated

    master_list.write_text(content)
    return updates_made


def sync_completions_to_source_files() -> Dict[str, int]:
    """
    Sync completed tasks from master task list back to their source account files.

    Finds tasks marked [x] in master list that have a Source: field pointing
    to an account action file, then marks the matching item complete in that file.

    Returns:
        Dict with 'synced' count and 'files_updated' count
    """
    import re

    master_list = TODAY_DIR / "tasks/master-task-list.md"

    if not master_list.exists():
        return {'synced': 0, 'files_updated': 0}

    content = master_list.read_text()

    # Parse completed tasks with their metadata
    # Pattern matches: - [x] **Title** followed by metadata lines
    completed_tasks = []
    lines = content.split('\n')
    i = 0

    while i < len(lines):
        line = lines[i]

        # Check for completed task
        match = re.match(r'^-\s*\[x\]\s*\*\*(.+?)\*\*', line)
        if match:
            title = match.group(1)
            task_data = {'title': title, 'source': None, 'account': None, 'task_id': None}

            # Extract task ID if present (e.g., `2026-01-20-002`)
            id_match = re.search(r'`(\d{4}-\d{2}-\d{2}-\d+)`', line)
            if id_match:
                task_data['task_id'] = id_match.group(1)

            # Look at following lines for metadata
            j = i + 1
            while j < len(lines) and lines[j].startswith('  '):
                meta_line = lines[j].strip()

                # Extract Source field
                if meta_line.startswith('- Source:') or meta_line.startswith('Source:'):
                    source_match = re.search(r'Source:\s*`?([^`\n]+)`?', meta_line)
                    if source_match:
                        task_data['source'] = source_match.group(1).strip()

                # Extract Account field
                if meta_line.startswith('- Account:') or meta_line.startswith('Account:'):
                    account_match = re.search(r'Account:\s*(.+)', meta_line)
                    if account_match:
                        task_data['account'] = account_match.group(1).strip()

                j += 1

            if task_data['source']:
                completed_tasks.append(task_data)

            i = j
        else:
            i += 1

    # Now sync each completed task to its source file
    synced = 0
    files_updated = set()

    for task in completed_tasks:
        source_path = VIP_ROOT / task['source']

        if not source_path.exists():
            continue

        try:
            source_content = source_path.read_text()
            original_content = source_content

            # Try to find and mark the matching task
            # Strategy 1: Match by task ID if available
            if task['task_id']:
                pattern = rf'^(\s*-\s*)\[ \](.+?{re.escape(task["task_id"])}.*)$'
                source_content, count = re.subn(pattern, r'\1[x]\2', source_content, flags=re.MULTILINE)
                if count > 0:
                    synced += count
                    files_updated.add(str(source_path))

            # Strategy 2: Match by title (fuzzy match)
            if source_content == original_content:  # No match by ID
                # Escape special regex chars in title but allow some flexibility
                title_words = task['title'].split()[:4]  # First 4 words
                if title_words:
                    # Build pattern that matches these words in sequence
                    title_pattern = r'.*?'.join(re.escape(w) for w in title_words)
                    pattern = rf'^(\s*-\s*)\[ \](\s*\*?\*?{title_pattern}.*)$'
                    source_content, count = re.subn(pattern, r'\1[x]\2', source_content, count=1, flags=re.MULTILINE | re.IGNORECASE)
                    if count > 0:
                        synced += count
                        files_updated.add(str(source_path))

            # Write back if changed
            if source_content != original_content:
                source_path.write_text(source_content)

        except Exception as e:
            print(f"  ⚠️  Error syncing to {task['source']}: {e}")
            continue

    return {'synced': synced, 'files_updated': len(files_updated)}


def write_wrap_summary(directive: Dict, ai_outputs: Dict, archive_path: Path) -> Path:
    """
    Write the wrap summary file.

    Args:
        directive: The directive dictionary
        ai_outputs: AI-generated outputs (impact, etc.)
        archive_path: Path where files were archived

    Returns:
        Path to written file
    """
    context = directive.get('context', {})
    date = context.get('date', datetime.now().strftime('%Y-%m-%d'))

    # Build meetings table
    completed = directive.get('completed_meetings', [])
    transcript_status = {s['event_id']: s for s in directive.get('transcript_status', [])}

    meeting_rows = []
    for meeting in completed:
        event_id = meeting.get('event_id')
        status = transcript_status.get(event_id, {})

        transcript_icon = {
            'processed': '✅',
            'in_inbox': '⚠️',
            'missing': '❌',
            'not_applicable': '-',
        }.get(status.get('status', 'not_applicable'), '-')

        summary_icon = '✅' if status.get('summary_exists') else '❌'
        actions_icon = '✅' if status.get('actions_exists') else '-'

        meeting_rows.append(
            f"| {meeting.get('account', meeting.get('title', 'Unknown'))} | "
            f"{meeting.get('start', '')[:5] if meeting.get('start') else '-'} | "
            f"{transcript_icon} | {summary_icon} | {actions_icon} |"
        )

    meetings_table = "| Account | Time | Transcript | Summary | Actions |\n"
    meetings_table += "|---------|------|------------|---------|----------|\n"
    meetings_table += "\n".join(meeting_rows) if meeting_rows else "| - | - | - | - | - |"

    # Build tasks section
    tasks_due = directive.get('tasks_due_today', [])
    completed_tasks = [t for t in tasks_due if t.get('new_status') == 'Completed']
    open_tasks = [t for t in tasks_due if t.get('new_status') != 'Completed']

    completed_items = []
    for task in completed_tasks:
        completed_items.append(f"- [x] {task.get('title', 'Unknown')}")

    open_items = []
    for task in open_tasks:
        open_items.append(f"- [ ] {task.get('title', 'Unknown')} - Status: {task.get('new_status', 'Unknown')}")

    # Build inbox status
    inbox_files = directive.get('inbox_files', [])

    # Build impact section (from AI outputs)
    customer_outcomes = ai_outputs.get('customer_outcomes', 'No customer outcomes captured.')
    personal_impact = ai_outputs.get('personal_impact', 'No personal impact captured.')

    content = f"""# Day Wrap Summary - {date}

## Meetings Completed

{meetings_table}

## Action Items Reconciled

### Completed Today
{chr(10).join(completed_items) if completed_items else "No tasks completed today."}

### Still Open (Carried Forward)
{chr(10).join(open_items) if open_items else "All tasks complete!"}

## Impacts Captured

### Customer Outcomes
{customer_outcomes}

### Personal Impact
{personal_impact}

## Inbox Status
- Files pending: {len(inbox_files)}
{chr(10).join([f"  - {f['name']}" for f in inbox_files[:5]]) if inbox_files else "  - Inbox empty ✅"}

## Archive Status
- Archived to: `{archive_path.relative_to(VIP_ROOT)}`

---
*Wrapped at: {datetime.now().strftime('%Y-%m-%d %H:%M')}*
*Ready for tomorrow's /today*
"""

    output_path = archive_path / "wrap-summary.md"
    output_path.write_text(content)

    return output_path


def display_completion_summary(directive: Dict, archive_path: Path, files_archived: int) -> None:
    """
    Display the completion summary.

    Args:
        directive: The directive dictionary
        archive_path: Path where files were archived
        files_archived: Number of files archived
    """
    date = directive.get('context', {}).get('date', 'Today')
    completed = directive.get('completed_meetings', [])
    transcript_status = directive.get('transcript_status', [])
    tasks_due = directive.get('tasks_due_today', [])
    inbox_files = directive.get('inbox_files', [])

    processed = len([s for s in transcript_status if s['status'] == 'processed'])
    in_inbox = len([s for s in transcript_status if s['status'] == 'in_inbox'])
    missing = len([s for s in transcript_status if s['status'] == 'missing'])

    completed_tasks = len([t for t in tasks_due if t.get('new_status') == 'Completed'])
    open_tasks = len(tasks_due) - completed_tasks

    print("\n" + "━" * 60)
    print(f"DAY WRAP COMPLETE - {date}")
    print("━" * 60)
    print()
    print(f"✅ Meetings: {len(completed)} completed")
    if transcript_status:
        print(f"   - Transcripts processed: {processed}")
        if in_inbox:
            print(f"   ⚠️  In inbox: {in_inbox}")
        if missing:
            print(f"   ❌ Missing: {missing}")

    print(f"✅ Actions: {completed_tasks} completed, {open_tasks} carried forward")
    print(f"✅ Archived: {files_archived} files to {archive_path.name}/")

    if inbox_files:
        print(f"⚠️  Inbox: {len(inbox_files)} files pending")

    print()
    if missing:
        print("Outstanding items for tomorrow:")
        for status in transcript_status:
            if status['status'] == 'missing':
                print(f"  - Process {status['account']} transcript when available")

    print()
    print("Good night! 🌙")
    print("━" * 60)


def cleanup_directive(path: Path) -> None:
    """
    Remove the directive file after successful delivery.

    Args:
        path: Path to directive file
    """
    if path.exists():
        path.unlink()


def main():
    """Main delivery orchestrator."""
    parser = argparse.ArgumentParser(description='Deliver wrap files')
    parser.add_argument('--directive', type=str, default=str(DIRECTIVE_FILE), help='Directive file path')
    parser.add_argument('--skip-archive', action='store_true', help='Skip file archival')
    parser.add_argument('--keep-directive', action='store_true', help='Keep directive file after delivery')
    parser.add_argument('--ai-outputs', type=str, help='JSON file with AI outputs (impact, etc.)')
    args = parser.parse_args()

    print("=" * 60)
    print("PHASE 3: WRAP DELIVERY")
    print("=" * 60)

    # Load directive
    directive_path = Path(args.directive)
    directive = load_directive(directive_path)

    if not directive:
        print("\nError: Could not load directive. Run prepare_wrap.py first.")
        return 1

    # Load AI outputs if provided
    ai_outputs = {}
    if args.ai_outputs:
        ai_path = Path(args.ai_outputs)
        if ai_path.exists():
            with open(ai_path) as f:
                ai_outputs = json.load(f)

    context = directive.get('context', {})
    date_str = context.get('date', datetime.now().strftime('%Y-%m-%d'))
    today = datetime.strptime(date_str, '%Y-%m-%d')

    # Ensure structure
    ensure_today_structure()

    # Step 1: Update week overview prep status
    print("\nStep 1: Updating week overview prep status...")
    reconciliation = directive.get('prep_reconciliation', [])
    if update_week_overview_prep_status(reconciliation):
        print(f"  ✅ Updated {len(reconciliation)} meeting statuses")
    else:
        print("  ⚠️  No week overview found")

    # Step 2: Update master task list
    print("\nStep 2: Updating master task list...")
    task_updates = directive.get('tasks_due_today', [])
    updated_count = update_master_task_list(task_updates)
    print(f"  ✅ Updated {updated_count} tasks")

    # Step 2B: Sync completions back to source account files
    print("\nStep 2B: Syncing completions to source files...")
    sync_results = sync_completions_to_source_files()
    if sync_results['synced'] > 0:
        print(f"  ✅ Synced {sync_results['synced']} items to {sync_results['files_updated']} source files")
    else:
        print("  ℹ️  No completed items with source files to sync")

    # Step 3: Archive today's files
    archive_path = ARCHIVE_DIR / date_str
    files_archived = 0

    if not args.skip_archive:
        print("\nStep 3: Archiving today's files...")
        today_files = list_today_files()

        if today_files['daily']:
            archived = archive_daily_files(today)
            files_archived = len(archived)
            print(f"  ✅ Archived {files_archived} files to {archive_path.name}/")
        else:
            print("  ⚠️  No files to archive")
            archive_path.mkdir(exist_ok=True)
    else:
        print("\nStep 3: Skipping archive")
        archive_path.mkdir(exist_ok=True)

    # Step 4: Write wrap summary
    print("\nStep 4: Writing wrap summary...")
    summary_path = write_wrap_summary(directive, ai_outputs, archive_path)
    print(f"  ✅ {summary_path.name}")

    # Cleanup
    if not args.keep_directive:
        print("\nCleaning up directive file...")
        cleanup_directive(directive_path)
        print("  ✅ Directive removed")

    # Display completion summary
    display_completion_summary(directive, archive_path, files_archived)

    return 0


if __name__ == "__main__":
    sys.exit(main())
