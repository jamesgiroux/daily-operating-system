"""
Dashboard utilities for auto-starting the web dashboard.

Provides functions to check dashboard status and start it in the background.
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Tuple, Dict, Any


def get_workspace_root() -> Path:
    """Get the workspace root directory."""
    # When installed, lib is at _tools/lib, so root is two levels up
    return Path(__file__).parent.parent.parent


def get_ui_dir() -> Path:
    """Get the _ui directory path."""
    return get_workspace_root() / "_ui"


def get_config_path() -> Path:
    """Get the workspace config path."""
    return get_workspace_root() / "_config" / "workspace.json"


def load_workspace_config() -> Dict[str, Any]:
    """
    Load the workspace configuration from _config/workspace.json.

    Returns:
        Configuration dictionary, empty dict if file doesn't exist
    """
    config_path = get_config_path()
    try:
        if config_path.exists():
            with open(config_path, 'r') as f:
                return json.load(f)
    except (json.JSONDecodeError, IOError):
        pass
    return {}


def is_dashboard_autostart_enabled() -> bool:
    """
    Check if web_dashboard_autostart feature flag is enabled.

    Returns:
        True if autostart is enabled, False otherwise
    """
    config = load_workspace_config()
    features = config.get('features', {})
    return features.get('web_dashboard_autostart', False)


def check_port_in_use(port: int = 5050) -> bool:
    """
    Check if a port is currently in use.

    Args:
        port: Port number to check

    Returns:
        True if port is in use, False otherwise
    """
    try:
        result = subprocess.run(
            ["lsof", "-i", f":{port}", "-t"],
            capture_output=True,
            text=True,
            timeout=5
        )
        return bool(result.stdout.strip())
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return False


def check_dashboard_available() -> Tuple[bool, str]:
    """
    Verify the dashboard is available and dependencies installed.

    Returns:
        Tuple of (is_available, message)
    """
    ui_dir = get_ui_dir()
    server_path = ui_dir / "server.js"
    node_modules = ui_dir / "node_modules"

    if not ui_dir.exists():
        return False, "_ui directory not found"

    if not server_path.exists():
        return False, "server.js not found in _ui"

    if not node_modules.exists():
        return False, "node_modules not installed (run npm install in _ui)"

    # Check if node is available
    try:
        result = subprocess.run(
            ["node", "--version"],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode != 0:
            return False, "node not available"
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False, "node not found"

    return True, "Dashboard available"


def start_dashboard_background(port: int = 5050) -> Tuple[bool, str]:
    """
    Start the dashboard server in the background.

    Spawns node server.js as a detached process that survives
    after the parent script exits.

    Args:
        port: Port to run the server on

    Returns:
        Tuple of (success, message)
    """
    # Check if already running
    if check_port_in_use(port):
        return True, f"already running on port {port}"

    # Check if dashboard is available
    available, reason = check_dashboard_available()
    if not available:
        return False, reason

    ui_dir = get_ui_dir()

    try:
        # Build environment with PORT
        env = os.environ.copy()
        env['PORT'] = str(port)

        # Start server as detached process
        # Use start_new_session to detach from parent process group
        process = subprocess.Popen(
            ["node", "server.js"],
            cwd=str(ui_dir),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
            start_new_session=True,
            env=env
        )

        # Give it a moment to start
        import time
        time.sleep(0.5)

        # Verify it started
        if check_port_in_use(port):
            return True, f"started on port {port}"
        else:
            # Check if process is still running
            if process.poll() is None:
                return True, f"starting on port {port}"
            else:
                return False, "server exited immediately"

    except FileNotFoundError:
        return False, "node not found"
    except Exception as e:
        return False, f"failed to start: {str(e)}"


def stop_dashboard(port: int = 5050) -> Tuple[bool, str]:
    """
    Stop the dashboard server if running.

    Args:
        port: Port the server is running on

    Returns:
        Tuple of (success, message)
    """
    if not check_port_in_use(port):
        return True, "not running"

    try:
        # Get PID(s) using the port
        result = subprocess.run(
            ["lsof", "-i", f":{port}", "-t"],
            capture_output=True,
            text=True,
            timeout=5
        )

        if result.stdout.strip():
            pids = result.stdout.strip().split('\n')
            for pid in pids:
                subprocess.run(["kill", pid], timeout=5)

            return True, f"stopped {len(pids)} process(es)"

        return True, "not running"

    except Exception as e:
        return False, f"failed to stop: {str(e)}"


def get_dashboard_url(port: int = 5050) -> str:
    """
    Get the dashboard URL.

    Args:
        port: Port the server is running on

    Returns:
        Dashboard URL string
    """
    return f"http://localhost:{port}"
