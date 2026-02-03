"""
Workspace detection and configuration for DailyOS.

This module provides smart workspace detection that allows `dailyos start`
to work from any directory by:

1. Checking for explicit -w flag
2. Checking current directory for .dailyos-version
3. Checking stored default workspace in config
4. Auto-scanning common locations for workspaces

Config is stored at ~/.dailyos/config.json
"""

import json
import os
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Tuple, Dict

from version import CORE_PATH

# Config file location
CONFIG_PATH = CORE_PATH / 'config.json'

# Default locations to scan for workspaces
DEFAULT_SCAN_LOCATIONS = [
    Path.home() / 'Documents',
    Path.home() / 'workspace',
    Path.home() / 'projects',
    Path.home() / 'dev',
]

# Directories to skip when scanning
SKIP_DIRECTORIES = {
    '.git', '.svn', '.hg',
    'node_modules', '__pycache__',
    '.venv', 'venv', '.env',
    'build', 'dist', 'target',
    '.cache', '.npm', '.yarn',
}

# Default config schema
DEFAULT_CONFIG = {
    'version': '1.0',
    'default_workspace': None,
    'scan_locations': [str(loc) for loc in DEFAULT_SCAN_LOCATIONS if loc.exists()],
    'scan_depth': 2,
    'known_workspaces': [],
    'preferences': {
        'auto_save_default': True,
        'prompt_on_multiple': True,
    }
}


class WorkspaceConfig:
    """
    Manages ~/.dailyos/config.json for workspace configuration.

    Provides methods to:
    - Load/save configuration
    - Get/set default workspace
    - Track known workspaces
    - Configure scan locations
    """

    def __init__(self):
        self._config: Optional[Dict] = None

    def load(self) -> Dict:
        """
        Load configuration from disk.

        Returns:
            Configuration dictionary, with defaults applied for missing keys
        """
        if self._config is not None:
            return self._config

        if CONFIG_PATH.exists():
            try:
                self._config = json.loads(CONFIG_PATH.read_text())
                # Merge with defaults to handle missing keys
                self._config = self._merge_with_defaults(self._config)
            except (json.JSONDecodeError, OSError):
                self._config = DEFAULT_CONFIG.copy()
        else:
            self._config = DEFAULT_CONFIG.copy()

        return self._config

    def _merge_with_defaults(self, config: Dict) -> Dict:
        """Merge loaded config with defaults for any missing keys."""
        result = DEFAULT_CONFIG.copy()
        result.update(config)

        # Ensure nested dicts are merged
        if 'preferences' in config:
            result['preferences'] = {**DEFAULT_CONFIG['preferences'], **config['preferences']}

        return result

    def save(self, config: Optional[Dict] = None) -> None:
        """
        Save configuration to disk.

        Args:
            config: Configuration to save. If None, saves current config.
        """
        if config is not None:
            self._config = config

        if self._config is None:
            return

        # Ensure parent directory exists
        CONFIG_PATH.parent.mkdir(parents=True, exist_ok=True)

        CONFIG_PATH.write_text(json.dumps(self._config, indent=2) + '\n')

    def get_default_workspace(self) -> Optional[Path]:
        """
        Get the default workspace path.

        Returns:
            Path to default workspace, or None if not set
        """
        config = self.load()
        default = config.get('default_workspace')

        if default:
            path = Path(os.path.expanduser(default))
            if path.exists():
                return path

        return None

    def set_default_workspace(self, path: Path) -> None:
        """
        Set the default workspace.

        Args:
            path: Path to the workspace directory
        """
        config = self.load()
        config['default_workspace'] = str(path.resolve())
        self.save()

        # Also add to known workspaces
        self.add_known_workspace(path)

    def clear_default_workspace(self) -> None:
        """Clear the default workspace setting."""
        config = self.load()
        config['default_workspace'] = None
        self.save()

    def get_scan_locations(self) -> List[Path]:
        """
        Get list of directories to scan for workspaces.

        Returns:
            List of existing directories to scan
        """
        config = self.load()
        locations = []

        for loc in config.get('scan_locations', []):
            path = Path(os.path.expanduser(loc))
            if path.exists() and path.is_dir():
                locations.append(path)

        return locations

    def get_scan_depth(self) -> int:
        """Get the maximum depth for workspace scanning."""
        config = self.load()
        return config.get('scan_depth', 2)

    def add_known_workspace(self, path: Path, name: Optional[str] = None) -> None:
        """
        Add a workspace to the known workspaces list.

        Args:
            path: Path to the workspace
            name: Optional display name (defaults to directory name)
        """
        config = self.load()
        path = path.resolve()

        # Check if already known
        known = config.get('known_workspaces', [])
        for ws in known:
            if ws.get('path') == str(path):
                # Update last_used
                ws['last_used'] = datetime.now().isoformat()
                self.save()
                return

        # Add new workspace
        known.append({
            'path': str(path),
            'name': name or path.name,
            'last_used': datetime.now().isoformat(),
        })

        config['known_workspaces'] = known
        self.save()

    def update_last_used(self, path: Path) -> None:
        """Update the last_used timestamp for a workspace."""
        config = self.load()
        path = path.resolve()

        for ws in config.get('known_workspaces', []):
            if ws.get('path') == str(path):
                ws['last_used'] = datetime.now().isoformat()
                self.save()
                return

    def get_known_workspaces(self) -> List[Dict]:
        """
        Get list of known workspaces sorted by last_used.

        Returns:
            List of workspace dicts with path, name, last_used
        """
        config = self.load()
        known = config.get('known_workspaces', [])

        # Filter to only existing workspaces
        valid = []
        for ws in known:
            path = Path(ws.get('path', ''))
            if path.exists() and (path / '.dailyos-version').exists():
                valid.append(ws)

        # Sort by last_used (most recent first)
        valid.sort(key=lambda x: x.get('last_used', ''), reverse=True)

        return valid

    def remove_known_workspace(self, path: Path) -> None:
        """Remove a workspace from the known list."""
        config = self.load()
        path = path.resolve()

        known = config.get('known_workspaces', [])
        config['known_workspaces'] = [
            ws for ws in known if ws.get('path') != str(path)
        ]
        self.save()


class WorkspaceScanner:
    """
    Discovers workspaces by scanning for .dailyos-version marker files.

    Workspaces are identified by the presence of a .dailyos-version file
    in their root directory.
    """

    def __init__(self, config: Optional[WorkspaceConfig] = None):
        self.config = config or WorkspaceConfig()

    def scan_all(self) -> List[Path]:
        """
        Scan all configured locations for workspaces.

        Returns:
            List of workspace paths found
        """
        workspaces = []
        depth = self.config.get_scan_depth()

        for location in self.config.get_scan_locations():
            found = self.scan_directory(location, depth=depth)
            workspaces.extend(found)

        # Deduplicate by resolved path
        seen = set()
        unique = []
        for ws in workspaces:
            resolved = str(ws.resolve())
            if resolved not in seen:
                seen.add(resolved)
                unique.append(ws)

        return unique

    def scan_directory(self, path: Path, depth: int = 2) -> List[Path]:
        """
        Scan a directory for workspaces up to specified depth.

        Args:
            path: Directory to scan
            depth: Maximum depth to scan (0 = check path only)

        Returns:
            List of workspace paths found
        """
        workspaces = []

        if not path.exists() or not path.is_dir():
            return workspaces

        # Check if this directory is a workspace
        if self.is_valid_workspace(path)[0]:
            workspaces.append(path)
            return workspaces  # Don't scan subdirectories of a workspace

        # If we've reached max depth, stop
        if depth <= 0:
            return workspaces

        # Scan subdirectories
        try:
            for child in path.iterdir():
                if child.is_dir() and child.name not in SKIP_DIRECTORIES:
                    # Skip hidden directories (except .dailyos itself)
                    if child.name.startswith('.') and child.name != '.dailyos':
                        continue

                    found = self.scan_directory(child, depth=depth - 1)
                    workspaces.extend(found)
        except PermissionError:
            pass  # Skip directories we can't read

        return workspaces

    def is_valid_workspace(self, path: Path) -> Tuple[bool, Optional[str]]:
        """
        Check if a path is a valid DailyOS workspace.

        A valid workspace has:
        - A .dailyos-version file

        Args:
            path: Path to check

        Returns:
            Tuple of (is_valid, version_string or None)
        """
        version_file = path / '.dailyos-version'

        if not version_file.exists():
            return False, None

        try:
            version = version_file.read_text().strip()
            return True, version
        except OSError:
            return False, None


class WorkspaceResolver:
    """
    Main resolution engine for workspace detection.

    Implements a priority cascade:
    1. Explicit -w flag
    2. Current working directory
    3. Stored default workspace in config
    4. Auto-detect (scan for workspaces)

    Returns the resolved workspace and the method used to find it.
    """

    # Resolution methods
    METHOD_EXPLICIT = "explicit"
    METHOD_CWD = "cwd"
    METHOD_CONFIG = "config"
    METHOD_AUTO_SINGLE = "auto-single"
    METHOD_AUTO_SELECTED = "auto-selected"
    METHOD_NONE = "none"

    def __init__(self, config: Optional[WorkspaceConfig] = None):
        self.config = config or WorkspaceConfig()
        self.scanner = WorkspaceScanner(self.config)

    def resolve(
        self,
        explicit: Optional[Path] = None,
        allow_interactive: bool = True
    ) -> Tuple[Optional[Path], str]:
        """
        Resolve the workspace to use.

        Args:
            explicit: Explicitly provided workspace path (-w flag)
            allow_interactive: Whether to allow interactive prompts

        Returns:
            Tuple of (workspace_path, resolution_method)
            - workspace_path: Path to workspace, or None if not found
            - resolution_method: One of the METHOD_* constants
        """
        # 1. Check explicit workspace
        if explicit:
            explicit = Path(os.path.expanduser(str(explicit))).resolve()
            is_valid, _ = self.scanner.is_valid_workspace(explicit)
            if is_valid:
                return explicit, self.METHOD_EXPLICIT
            # Explicit path given but not valid - still return it with explicit method
            # Let caller decide whether to error
            if explicit.exists():
                return explicit, self.METHOD_EXPLICIT

        # 2. Check current working directory
        cwd = Path.cwd()
        is_valid, _ = self.scanner.is_valid_workspace(cwd)
        if is_valid:
            return cwd, self.METHOD_CWD

        # 3. Check stored default
        default = self.config.get_default_workspace()
        if default:
            is_valid, _ = self.scanner.is_valid_workspace(default)
            if is_valid:
                return default, self.METHOD_CONFIG

        # 4. Auto-detect by scanning
        workspaces = self.scanner.scan_all()

        # Also check known workspaces in case they're not in scan locations
        for ws in self.config.get_known_workspaces():
            path = Path(ws['path'])
            if path not in workspaces:
                workspaces.append(path)

        if len(workspaces) == 0:
            return None, self.METHOD_NONE

        if len(workspaces) == 1:
            return workspaces[0], self.METHOD_AUTO_SINGLE

        # Multiple workspaces found - need selection
        # Return the list in a way the caller can present for selection
        # For now, return first one but indicate selection needed
        return workspaces[0], self.METHOD_AUTO_SELECTED

    def get_available_workspaces(self) -> List[Dict]:
        """
        Get all available workspaces with metadata.

        Returns:
            List of workspace info dicts with:
            - path: Path to workspace
            - name: Display name
            - version: DailyOS version
            - last_used: Last used timestamp (if known)
        """
        workspaces = []
        scanner = self.scanner

        # Get from scanning
        for path in scanner.scan_all():
            is_valid, version = scanner.is_valid_workspace(path)
            if is_valid:
                workspaces.append({
                    'path': path,
                    'name': path.name,
                    'version': version,
                    'last_used': None,
                })

        # Merge with known workspaces for last_used info
        known = {ws['path']: ws for ws in self.config.get_known_workspaces()}
        for ws in workspaces:
            path_str = str(ws['path'].resolve())
            if path_str in known:
                ws['last_used'] = known[path_str].get('last_used')

        # Sort by last_used (most recent first), then by name
        def sort_key(ws):
            last_used = ws.get('last_used') or ''
            return (not bool(last_used), -ord(last_used[0]) if last_used else 0, ws['name'].lower())

        workspaces.sort(key=lambda ws: (
            not bool(ws.get('last_used')),
            -(datetime.fromisoformat(ws['last_used']).timestamp() if ws.get('last_used') else 0),
            ws['name'].lower()
        ))

        return workspaces

    def validate_workspace(self, path: Path) -> Tuple[bool, str]:
        """
        Validate a workspace path and return status.

        Args:
            path: Path to validate

        Returns:
            Tuple of (is_valid, status_message)
        """
        if not path.exists():
            return False, f"Path does not exist: {path}"

        if not path.is_dir():
            return False, f"Not a directory: {path}"

        is_valid, version = self.scanner.is_valid_workspace(path)

        if is_valid:
            return True, f"Valid workspace (v{version})"

        return False, "Not a DailyOS workspace (missing .dailyos-version)"


def format_relative_time(iso_timestamp: Optional[str]) -> str:
    """
    Format an ISO timestamp as a relative time string.

    Args:
        iso_timestamp: ISO format timestamp

    Returns:
        Human-readable relative time (e.g., "today", "yesterday", "3 days ago")
    """
    if not iso_timestamp:
        return "never"

    try:
        dt = datetime.fromisoformat(iso_timestamp)
        now = datetime.now()
        diff = now - dt

        if diff.days == 0:
            return "today"
        elif diff.days == 1:
            return "yesterday"
        elif diff.days < 7:
            return f"{diff.days} days ago"
        elif diff.days < 30:
            weeks = diff.days // 7
            return f"{weeks} week{'s' if weeks > 1 else ''} ago"
        elif diff.days < 365:
            months = diff.days // 30
            return f"{months} month{'s' if months > 1 else ''} ago"
        else:
            years = diff.days // 365
            return f"{years} year{'s' if years > 1 else ''} ago"
    except (ValueError, TypeError):
        return "unknown"


def get_scan_summary() -> List[str]:
    """
    Get a summary of locations that would be scanned.

    Returns:
        List of location descriptions for error messages
    """
    config = WorkspaceConfig()
    summary = []

    # Current directory
    summary.append(f"Current directory: {Path.cwd()}")

    # Configured scan locations
    for loc in config.get_scan_locations():
        summary.append(f"~/{loc.relative_to(Path.home())} (scanned)")

    # Check for common locations that don't exist
    for loc in DEFAULT_SCAN_LOCATIONS:
        if not loc.exists():
            try:
                rel = loc.relative_to(Path.home())
                summary.append(f"~/{rel} (not found)")
            except ValueError:
                summary.append(f"{loc} (not found)")

    return summary
