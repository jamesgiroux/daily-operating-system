"""Version management utilities for DailyOS."""

import json
import re
import subprocess
from datetime import datetime, date
from pathlib import Path
from typing import Optional, Dict, List, Tuple

# Core installation path
CORE_PATH = Path.home() / '.dailyos'


def get_core_version() -> str:
    """
    Get version from core repo.

    Returns:
        Version string (e.g., "0.4.0") or "0.0.0" if not found
    """
    version_file = CORE_PATH / 'VERSION'
    if version_file.exists():
        return version_file.read_text().strip()
    return '0.0.0'


def get_workspace_version(workspace: Path) -> str:
    """
    Get installed version for a workspace.

    Args:
        workspace: Path to the workspace directory

    Returns:
        Version string or "0.0.0" if not tracked
    """
    version_file = workspace / '.dailyos-version'
    if version_file.exists():
        return version_file.read_text().strip()
    return '0.0.0'


def set_workspace_version(workspace: Path, version: str) -> None:
    """
    Set the installed version for a workspace.

    Args:
        workspace: Path to the workspace directory
        version: Version string to set
    """
    version_file = workspace / '.dailyos-version'
    version_file.write_text(version + '\n')


def get_ejected_skills(workspace: Path) -> List[str]:
    """
    Get list of ejected (user-owned) skills.

    Args:
        workspace: Path to the workspace directory

    Returns:
        List of ejected skill/command names
    """
    ejected_file = workspace / '.dailyos-ejected'
    if ejected_file.exists():
        try:
            return json.loads(ejected_file.read_text())
        except json.JSONDecodeError:
            return []
    return []


def set_ejected_skills(workspace: Path, ejected: List[str]) -> None:
    """
    Set the list of ejected skills.

    Args:
        workspace: Path to the workspace directory
        ejected: List of ejected skill/command names
    """
    ejected_file = workspace / '.dailyos-ejected'
    ejected_file.write_text(json.dumps(ejected, indent=2) + '\n')


def add_ejected_skill(workspace: Path, name: str) -> None:
    """Add a skill to the ejected list."""
    ejected = get_ejected_skills(workspace)
    if name not in ejected:
        ejected.append(name)
        set_ejected_skills(workspace, ejected)


def remove_ejected_skill(workspace: Path, name: str) -> None:
    """Remove a skill from the ejected list."""
    ejected = get_ejected_skills(workspace)
    if name in ejected:
        ejected.remove(name)
        set_ejected_skills(workspace, ejected)


def get_skipped_versions(workspace: Path) -> List[str]:
    """Get list of versions the user chose to skip."""
    skip_file = workspace / '.dailyos-skip'
    if skip_file.exists():
        try:
            return json.loads(skip_file.read_text())
        except json.JSONDecodeError:
            return []
    return []


def skip_version(workspace: Path, version: str) -> None:
    """Mark a version as skipped."""
    skipped = get_skipped_versions(workspace)
    if version not in skipped:
        skipped.append(version)
        skip_file = workspace / '.dailyos-skip'
        skip_file.write_text(json.dumps(skipped) + '\n')


def compare_versions(v1: str, v2: str) -> int:
    """
    Compare two semantic versions.

    Returns:
        -1 if v1 < v2, 0 if equal, 1 if v1 > v2
    """
    def parse_version(v: str) -> Tuple[int, ...]:
        # Handle versions like "0.4.0" or "0.4.0-beta"
        match = re.match(r'(\d+)\.(\d+)\.(\d+)', v)
        if match:
            return tuple(int(x) for x in match.groups())
        return (0, 0, 0)

    p1 = parse_version(v1)
    p2 = parse_version(v2)

    if p1 < p2:
        return -1
    elif p1 > p2:
        return 1
    return 0


def check_for_updates(workspace: Path) -> Optional[Dict]:
    """
    Check if updates are available.

    Args:
        workspace: Path to the workspace directory

    Returns:
        Dict with update info if update available, None otherwise
    """
    core_version = get_core_version()
    workspace_version = get_workspace_version(workspace)
    skipped = get_skipped_versions(workspace)

    # Skip if this version was explicitly skipped
    if core_version in skipped:
        return None

    # Check if core is newer
    if compare_versions(core_version, workspace_version) > 0:
        return {
            'current': workspace_version,
            'available': core_version,
            'changelog': get_changelog_entries(workspace_version, core_version),
            'ejected': get_ejected_skills(workspace),
        }
    return None


def should_check_today(workspace: Path) -> bool:
    """
    Check if we should prompt for updates (max once per day).

    Args:
        workspace: Path to the workspace directory

    Returns:
        True if we should check, False if already checked today
    """
    check_file = workspace / '.dailyos-last-check'
    if not check_file.exists():
        return True
    try:
        last_check = datetime.fromisoformat(check_file.read_text().strip())
        return last_check.date() < date.today()
    except (ValueError, OSError):
        return True


def record_check(workspace: Path) -> None:
    """Record that we checked for updates today."""
    check_file = workspace / '.dailyos-last-check'
    check_file.write_text(datetime.now().isoformat() + '\n')


def get_changelog_entries(from_version: str, to_version: str) -> List[str]:
    """
    Get changelog entries between two versions.

    Args:
        from_version: Starting version (exclusive)
        to_version: Ending version (inclusive)

    Returns:
        List of changelog entry strings
    """
    changelog_file = CORE_PATH / 'CHANGELOG.md'
    if not changelog_file.exists():
        return []

    content = changelog_file.read_text()
    entries = []

    # Parse changelog sections
    current_version = None
    current_entries = []
    in_relevant_section = False

    for line in content.split('\n'):
        # Check for version header
        version_match = re.match(r'^## \[(\d+\.\d+\.\d+)\]', line)
        if version_match:
            version = version_match.group(1)

            # Save previous section if relevant
            if in_relevant_section and current_entries:
                entries.extend(current_entries)

            # Check if this version is in our range
            current_version = version
            current_entries = []

            # Include versions > from_version and <= to_version
            if compare_versions(version, from_version) > 0 and \
               compare_versions(version, to_version) <= 0:
                in_relevant_section = True
            else:
                in_relevant_section = False
            continue

        # Collect entries from relevant sections
        if in_relevant_section:
            # Look for list items under Added/Changed/Fixed
            if line.startswith('- '):
                entry = line[2:].strip()
                current_entries.append(entry)

    # Don't forget last section
    if in_relevant_section and current_entries:
        entries.extend(current_entries)

    return entries[:10]  # Limit to 10 entries


def git_pull_core() -> Tuple[bool, str]:
    """
    Pull latest from core repo.

    Returns:
        Tuple of (success, message)
    """
    if not (CORE_PATH / '.git').exists():
        return False, "Core is not a git repository"

    # Check if there's a remote configured
    try:
        result = subprocess.run(
            ['git', 'remote', '-v'],
            cwd=CORE_PATH,
            capture_output=True,
            text=True,
            timeout=10
        )
        if not result.stdout.strip():
            # No remote configured - local-only installation
            return True, "No remote configured (local installation)"
    except Exception:
        pass

    try:
        result = subprocess.run(
            ['git', 'pull', '--ff-only'],
            cwd=CORE_PATH,
            capture_output=True,
            text=True,
            timeout=30
        )
        output = result.stdout + result.stderr

        # Check for common "no tracking" error
        if "no tracking information" in output.lower():
            return True, "No tracking branch (local installation)"

        return result.returncode == 0, output.strip()
    except subprocess.TimeoutExpired:
        return False, "Git pull timed out"
    except Exception as e:
        return False, str(e)


def git_fetch_core() -> Tuple[bool, str]:
    """
    Fetch latest from core repo remote.

    Returns:
        Tuple of (success, message)
    """
    if not (CORE_PATH / '.git').exists():
        return False, "Core is not a git repository"

    try:
        result = subprocess.run(
            ['git', 'fetch'],
            cwd=CORE_PATH,
            capture_output=True,
            text=True,
            timeout=30
        )
        return result.returncode == 0, result.stdout + result.stderr
    except Exception as e:
        return False, str(e)


def check_remote_updates() -> Optional[str]:
    """
    Check if there are updates available on the remote.

    Returns:
        New version string if available, None otherwise
    """
    # First fetch
    success, _ = git_fetch_core()
    if not success:
        return None

    # Check if we're behind
    try:
        result = subprocess.run(
            ['git', 'rev-list', '--count', 'HEAD..origin/master'],
            cwd=CORE_PATH,
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode == 0:
            behind = int(result.stdout.strip())
            if behind > 0:
                # Get the version from origin
                result = subprocess.run(
                    ['git', 'show', 'origin/master:VERSION'],
                    cwd=CORE_PATH,
                    capture_output=True,
                    text=True,
                    timeout=10
                )
                if result.returncode == 0:
                    return result.stdout.strip()
    except Exception:
        pass

    return None


def is_symlink_intact(workspace: Path, name: str, subdir: str = '') -> bool:
    """
    Check if a symlink is properly pointing to core.

    Args:
        workspace: Workspace path
        name: File/directory name
        subdir: Subdirectory within .claude (e.g., 'commands', 'skills')

    Returns:
        True if symlink exists and points to correct location
    """
    if subdir:
        workspace_path = workspace / '.claude' / subdir / name
        core_path = CORE_PATH / subdir / name
    else:
        workspace_path = workspace / name
        core_path = CORE_PATH / name

    if not workspace_path.is_symlink():
        return False

    try:
        target = workspace_path.resolve()
        expected = core_path.resolve()
        return target == expected
    except OSError:
        return False


def get_workspace_status(workspace: Path) -> Dict:
    """
    Get comprehensive status of a workspace.

    Args:
        workspace: Path to workspace

    Returns:
        Dict with status information
    """
    status = {
        'workspace': str(workspace),
        'core_version': get_core_version(),
        'workspace_version': get_workspace_version(workspace),
        'ejected': get_ejected_skills(workspace),
        'symlinks': {},
        'problems': [],
    }

    # Check main symlinks
    for name in ['_tools', '_ui']:
        path = workspace / name
        if path.is_symlink():
            if path.resolve().exists():
                status['symlinks'][name] = 'ok'
            else:
                status['symlinks'][name] = 'broken'
                status['problems'].append(f'{name} symlink is broken')
        elif path.exists():
            status['symlinks'][name] = 'not_symlinked'
        else:
            status['symlinks'][name] = 'missing'
            status['problems'].append(f'{name} is missing')

    return status
