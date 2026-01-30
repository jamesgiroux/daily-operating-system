"""
Step 1: Prerequisites Check.

Verifies that the system has the required dependencies:
- Python 3.8+
- Claude Code CLI
- Git (optional but recommended)
"""

from typing import Tuple, List
from ..utils.validators import (
    validate_python_version,
    validate_command_exists
)


def check_all_prerequisites() -> Tuple[bool, List[Tuple[str, str, str]]]:
    """
    Check all prerequisites for the setup wizard.

    Returns:
        Tuple of (all_required_met, results)
        where results is a list of (name, status, message) tuples
        status is one of: 'ok', 'warn', 'fail'
    """
    results = []
    all_required_met = True

    # Python version (required)
    py_ok, py_version, py_err = validate_python_version((3, 8))
    if py_ok:
        results.append(('Python', 'ok', f'Python {py_version}'))
    else:
        results.append(('Python', 'fail', py_err or 'Python 3.8+ required'))
        all_required_met = False

    # Claude Code (recommended)
    cc_ok, cc_version, cc_err = validate_command_exists('claude')
    if cc_ok:
        # Truncate version string if too long
        version_display = cc_version[:50] + '...' if len(cc_version) > 50 else cc_version
        results.append(('Claude Code', 'ok', version_display))
    else:
        results.append(('Claude Code', 'warn', 'Not found (recommended)'))

    # Git (recommended)
    git_ok, git_version, git_err = validate_command_exists('git')
    if git_ok:
        version_display = git_version[:40] if len(git_version) > 40 else git_version
        results.append(('Git', 'ok', version_display))
    else:
        results.append(('Git', 'warn', 'Not found (recommended for version control)'))

    return all_required_met, results


def get_prerequisite_install_instructions() -> dict:
    """
    Get installation instructions for missing prerequisites.

    Returns:
        Dictionary mapping prerequisite name to installation instructions
    """
    return {
        'Python': """
Python 3.8+ Installation:
  macOS:   brew install python@3.11
  Ubuntu:  sudo apt install python3.11
  Windows: Download from https://python.org/downloads/
""",
        'Claude Code': """
Claude Code Installation:
  npm install -g @anthropic-ai/claude-code

Or if you don't have npm:
  1. Install Node.js: https://nodejs.org/
  2. Then run: npm install -g @anthropic-ai/claude-code
""",
        'Git': """
Git Installation:
  macOS:   brew install git
  Ubuntu:  sudo apt install git
  Windows: Download from https://git-scm.com/downloads
"""
    }
