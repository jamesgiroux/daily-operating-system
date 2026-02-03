"""Server management utilities for DailyOS web UI."""

import os
import socket
import subprocess
import time
import webbrowser
from pathlib import Path
from typing import Optional, Tuple

from version import CORE_PATH


def find_ui_directory(workspace: Optional[Path] = None) -> Optional[Path]:
    """
    Find _ui directory - check workspace first, then core.

    Search order:
    1. Explicit workspace parameter (if provided)
    2. Current working directory
    3. Core installation (~/.dailyos/_ui)

    Args:
        workspace: Optional explicit workspace path to check

    Returns:
        Path to _ui directory, or None if not found
    """
    candidates = []

    # Check explicit workspace first
    if workspace:
        workspace_ui = Path(workspace) / '_ui'
        candidates.append(workspace_ui)

    # Check current directory
    cwd_ui = Path.cwd() / '_ui'
    candidates.append(cwd_ui)

    # Fall back to core
    core_ui = CORE_PATH / '_ui'
    candidates.append(core_ui)

    for ui_dir in candidates:
        if (ui_dir / 'server.js').exists():
            return ui_dir.resolve()

    return None


def is_port_in_use(port: int) -> bool:
    """
    Check if a port is in use.

    Args:
        port: Port number to check

    Returns:
        True if port is in use, False otherwise
    """
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(('localhost', port)) == 0


def is_server_responding(port: int) -> bool:
    """
    Check if the DailyOS server is actually responding.

    This verifies the server is running and healthy, not just that
    the port is open (which could be another process).

    Args:
        port: Port number to check

    Returns:
        True if server responds to health check, False otherwise
    """
    try:
        import urllib.request
        import urllib.error
        urllib.request.urlopen(f'http://localhost:{port}/api/config', timeout=2)
        return True
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError):
        return False


def get_process_using_port(port: int) -> Optional[int]:
    """
    Get the PID of the process using a port.

    Args:
        port: Port number to check

    Returns:
        PID of process using the port, or None if port is free
    """
    try:
        result = subprocess.run(
            ['lsof', '-ti', f':{port}', '-sTCP:LISTEN'],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0 and result.stdout.strip():
            # May return multiple PIDs, take the first
            pids = result.stdout.strip().split('\n')
            return int(pids[0])
    except (subprocess.TimeoutExpired, ValueError, FileNotFoundError):
        pass
    return None


def kill_process(pid: int) -> bool:
    """
    Kill a process by PID.

    Args:
        pid: Process ID to kill

    Returns:
        True if process was killed, False otherwise
    """
    try:
        subprocess.run(['kill', '-9', str(pid)], capture_output=True, timeout=5)
        return True
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


def check_node_installed() -> bool:
    """
    Check if Node.js is installed.

    Returns:
        True if node is available, False otherwise
    """
    try:
        result = subprocess.run(
            ['node', '--version'],
            capture_output=True,
            timeout=5
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


def check_npm_installed() -> bool:
    """
    Check if npm is installed.

    Returns:
        True if npm is available, False otherwise
    """
    try:
        result = subprocess.run(
            ['npm', '--version'],
            capture_output=True,
            timeout=5
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


def install_dependencies(ui_dir: Path, quiet: bool = False) -> Tuple[bool, str]:
    """
    Run npm install if node_modules is missing.

    Args:
        ui_dir: Path to the _ui directory
        quiet: If True, suppress npm output

    Returns:
        Tuple of (success, message)
    """
    node_modules = ui_dir / 'node_modules'

    if node_modules.exists():
        return True, "Dependencies already installed"

    if not (ui_dir / 'package.json').exists():
        return False, "No package.json found"

    if not check_npm_installed():
        return False, "npm not installed. Install Node.js from https://nodejs.org"

    try:
        result = subprocess.run(
            ['npm', 'install', '--silent'] if quiet else ['npm', 'install'],
            cwd=ui_dir,
            capture_output=True,
            text=True,
            timeout=120  # 2 minutes for install
        )

        if result.returncode == 0:
            return True, "Dependencies installed"
        else:
            return False, f"npm install failed: {result.stderr}"

    except subprocess.TimeoutExpired:
        return False, "npm install timed out"
    except Exception as e:
        return False, f"Failed to install dependencies: {e}"


def start_server(
    ui_dir: Path,
    port: int = 5050,
    open_browser: bool = True,
    quiet: bool = False
) -> Tuple[bool, str]:
    """
    Start the DailyOS web UI server.

    Args:
        ui_dir: Path to the _ui directory
        port: Port to run on (default 5050)
        open_browser: Whether to open browser after starting
        quiet: If True, suppress output

    Returns:
        Tuple of (success, message)
    """
    # Check if already running on this port
    if is_server_responding(port):
        url = f'http://localhost:{port}'
        if open_browser:
            webbrowser.open(url)
        return True, f"Server already running at {url}"

    # Check if port is in use by something else
    if is_port_in_use(port):
        pid = get_process_using_port(port)
        if pid:
            # Try to determine if it's a zombie Node process
            try:
                result = subprocess.run(
                    ['ps', '-p', str(pid), '-o', 'comm='],
                    capture_output=True,
                    text=True,
                    timeout=5
                )
                process_name = result.stdout.strip()
                if 'node' in process_name.lower():
                    # Kill zombie Node process
                    if not quiet:
                        print(f"  Killing stale Node process (PID {pid})...")
                    kill_process(pid)
                    time.sleep(0.5)
                else:
                    return False, f"Port {port} in use by {process_name}. Try: dailyos start --port {port + 1}"
            except Exception:
                return False, f"Port {port} in use. Try: dailyos start --port {port + 1}"

    # Check Node.js
    if not check_node_installed():
        return False, "Node.js required. Install from https://nodejs.org"

    # Install dependencies if needed
    if not quiet:
        if not (ui_dir / 'node_modules').exists():
            print("  Installing dependencies...")

    success, msg = install_dependencies(ui_dir, quiet=True)
    if not success:
        return False, msg

    # Determine workspace from ui_dir
    # If ui_dir is a symlink, resolve to find actual location
    # The workspace is typically the parent of _ui
    workspace = ui_dir.parent
    if ui_dir.is_symlink():
        # For symlinked _ui, workspace is the directory containing the symlink
        workspace = ui_dir.parent

    # Start the server
    env = os.environ.copy()
    env['PORT'] = str(port)
    env['WORKSPACE'] = str(workspace)

    try:
        # Use npm start to run the server
        subprocess.Popen(
            ['npm', 'start'],
            cwd=ui_dir,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True  # Detach from parent process
        )
    except Exception as e:
        return False, f"Failed to start server: {e}"

    # Wait for server to start (up to 10 seconds)
    url = f'http://localhost:{port}'
    for i in range(20):
        time.sleep(0.5)
        if is_server_responding(port):
            if open_browser:
                webbrowser.open(url)
            return True, f"Server running at {url}"

    return False, "Server failed to start (timeout)"


def stop_server(port: int = 5050) -> Tuple[bool, str]:
    """
    Stop the DailyOS web UI server.

    Args:
        port: Port the server is running on

    Returns:
        Tuple of (success, message)
    """
    if not is_port_in_use(port):
        return True, "Server not running"

    pid = get_process_using_port(port)
    if not pid:
        return True, "Server not running"

    if kill_process(pid):
        # Wait a moment for process to die
        time.sleep(0.5)

        # Verify it's stopped
        if not is_port_in_use(port):
            return True, "Server stopped"
        else:
            return False, "Failed to stop server (process still running)"

    return False, "Failed to stop server"


def get_server_status(port: int = 5050) -> dict:
    """
    Get the current status of the DailyOS server.

    Args:
        port: Port to check

    Returns:
        Dict with status information
    """
    status = {
        'running': False,
        'responding': False,
        'port': port,
        'pid': None,
        'url': None,
    }

    if is_port_in_use(port):
        status['running'] = True
        status['pid'] = get_process_using_port(port)

        if is_server_responding(port):
            status['responding'] = True
            status['url'] = f'http://localhost:{port}'

    return status
