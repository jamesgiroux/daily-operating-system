"""
Input validation utilities.
"""

import os
import re
import subprocess
from pathlib import Path
from typing import Optional, Tuple


def validate_path(path_str: str) -> Tuple[bool, Optional[str]]:
    """
    Validate a filesystem path.

    Returns:
        Tuple of (is_valid, error_message)
    """
    if not path_str:
        return False, "Path cannot be empty"

    path = Path(os.path.expanduser(path_str))

    # Check for invalid characters
    if any(c in str(path) for c in ['<', '>', '|', '"', '*', '?']):
        return False, "Path contains invalid characters"

    # Check if parent exists
    if not path.parent.exists():
        return False, f"Parent directory does not exist: {path.parent}"

    return True, None


def validate_directory_writable(path_str: str) -> Tuple[bool, Optional[str]]:
    """
    Check if we can write to a directory.

    Returns:
        Tuple of (is_writable, error_message)
    """
    path = Path(os.path.expanduser(path_str))

    # If directory exists, check write permission
    if path.exists():
        if not os.access(path, os.W_OK):
            return False, f"Directory is not writable: {path}"
        return True, None

    # Check if parent is writable
    if path.parent.exists():
        if not os.access(path.parent, os.W_OK):
            return False, f"Cannot create directory (parent not writable): {path.parent}"
        return True, None

    return False, f"Parent directory does not exist: {path.parent}"


def validate_command_exists(command: str) -> Tuple[bool, Optional[str], Optional[str]]:
    """
    Check if a command exists and get its version.

    Returns:
        Tuple of (exists, version, error_message)
    """
    try:
        # Try running with --version
        result = subprocess.run(
            [command, "--version"],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0:
            version = result.stdout.strip().split('\n')[0]
            return True, version, None
    except FileNotFoundError:
        return False, None, f"Command not found: {command}"
    except subprocess.TimeoutExpired:
        return False, None, f"Command timed out: {command}"
    except Exception as e:
        return False, None, f"Error checking command {command}: {e}"

    return False, None, f"Command failed: {command}"


def validate_python_version(min_version: Tuple[int, int] = (3, 8)) -> Tuple[bool, str, Optional[str]]:
    """
    Check if Python version meets minimum requirement.

    Returns:
        Tuple of (meets_requirement, current_version, error_message)
    """
    import sys
    current = (sys.version_info.major, sys.version_info.minor)
    version_str = f"{current[0]}.{current[1]}.{sys.version_info.micro}"

    if current >= min_version:
        return True, version_str, None

    return False, version_str, f"Python {min_version[0]}.{min_version[1]}+ required, found {version_str}"


def validate_email(email: str) -> Tuple[bool, Optional[str]]:
    """
    Validate an email address format.

    Returns:
        Tuple of (is_valid, error_message)
    """
    if not email:
        return False, "Email cannot be empty"

    # Simple email regex
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    if re.match(pattern, email):
        return True, None

    return False, "Invalid email format"


def validate_url(url: str) -> Tuple[bool, Optional[str]]:
    """
    Validate a URL format.

    Returns:
        Tuple of (is_valid, error_message)
    """
    if not url:
        return False, "URL cannot be empty"

    # Simple URL regex
    pattern = r'^https?://[^\s/$.?#].[^\s]*$'
    if re.match(pattern, url):
        return True, None

    return False, "Invalid URL format"


def validate_json_file(path: Path) -> Tuple[bool, Optional[str]]:
    """
    Validate that a file is valid JSON.

    Returns:
        Tuple of (is_valid, error_message)
    """
    import json

    if not path.exists():
        return False, f"File does not exist: {path}"

    try:
        with open(path, 'r') as f:
            json.load(f)
        return True, None
    except json.JSONDecodeError as e:
        return False, f"Invalid JSON: {e}"
    except Exception as e:
        return False, f"Error reading file: {e}"
