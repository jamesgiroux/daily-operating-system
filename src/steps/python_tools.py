"""
Step 8: Python Tools Installation.

Installs Python automation scripts into the _tools directory.
"""

from pathlib import Path
from typing import List, Dict, Any


# Available Python tools
AVAILABLE_TOOLS = {
    'prepare_inbox': {
        'name': 'prepare_inbox.py',
        'description': 'Phase 1 of inbox processing - prepares files and generates agent directives',
        'category': 'inbox',
    },
    'deliver_inbox': {
        'name': 'deliver_inbox.py',
        'description': 'Phase 3 of inbox processing - delivers processed files to PARA locations',
        'category': 'inbox',
    },
    'generate_dashboard': {
        'name': 'generate_dashboard.py',
        'description': 'Creates account dashboards from data sources',
        'category': 'accounts',
    },
}


def get_tool_list(category: str = None) -> List[Dict[str, Any]]:
    """
    Get list of available Python tools.

    Args:
        category: Filter by category or None for all

    Returns:
        List of tool dictionaries
    """
    tools = []
    for key, tool in AVAILABLE_TOOLS.items():
        if category is None or tool['category'] == category:
            tools.append({'key': key, **tool})
    return tools


def get_tool_content(tool_key: str) -> str:
    """
    Get the content for a Python tool script.

    Args:
        tool_key: Tool identifier

    Returns:
        Python script content
    """
    if tool_key not in AVAILABLE_TOOLS:
        return ''

    tool = AVAILABLE_TOOLS[tool_key]

    # Placeholder content - actual scripts loaded from templates
    return f'''#!/usr/bin/env python3
"""
{tool['name']} - {tool['description']}

This is a placeholder. The full implementation will be installed
when you run the complete setup wizard with template installation.

Usage:
    python3 _tools/{tool['name']} [options]

Options:
    --help      Show this help message
    --dry-run   Preview changes without making them
"""

import sys
import argparse
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(
        description="{tool['description']}"
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Preview changes without making them'
    )
    args = parser.parse_args()

    print("{tool['name']}")
    print("=" * len("{tool['name']}"))
    print()
    print("This is a placeholder script.")
    print("Run the full setup wizard to install the complete version.")
    print()
    print("Expected functionality:")
    print(f"  {tool['description']}")

    if args.dry_run:
        print()
        print("[DRY RUN] No changes made.")

    return 0


if __name__ == "__main__":
    sys.exit(main())
'''


def install_tool(workspace: Path, tool_key: str, file_ops) -> bool:
    """
    Install a single Python tool.

    Args:
        workspace: Root workspace path
        tool_key: Tool identifier
        file_ops: FileOperations instance

    Returns:
        True if installed successfully
    """
    if tool_key not in AVAILABLE_TOOLS:
        return False

    tool = AVAILABLE_TOOLS[tool_key]
    tool_path = workspace / '_tools' / tool['name']

    content = get_tool_content(tool_key)
    file_ops.write_file(tool_path, content)

    return True


def install_all_tools(workspace: Path, file_ops) -> Dict[str, bool]:
    """
    Install all Python tools.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance

    Returns:
        Dictionary of installation results
    """
    results = {}

    for tool_key in AVAILABLE_TOOLS:
        results[tool_key] = install_tool(workspace, tool_key, file_ops)

    return results


def verify_tools_installation(workspace: Path) -> Dict[str, Any]:
    """
    Verify Python tools installation.

    Args:
        workspace: Root workspace path

    Returns:
        Verification results dictionary
    """
    results = {
        'tools_dir_exists': (workspace / '_tools').exists(),
        'tools': {},
    }

    tools_dir = workspace / '_tools'
    for tool_key, tool in AVAILABLE_TOOLS.items():
        tool_path = tools_dir / tool['name']
        results['tools'][tool_key] = {
            'exists': tool_path.exists(),
            'path': str(tool_path),
        }

    return results


def create_requirements_txt(workspace: Path, file_ops) -> bool:
    """
    Create requirements.txt for Python dependencies.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance

    Returns:
        True if created successfully
    """
    content = """# Daily Operating System Python Dependencies
#
# Install with: pip install -r requirements.txt

# Google API
google-api-python-client>=2.0.0
google-auth-httplib2>=0.1.0
google-auth-oauthlib>=0.5.0

# YAML processing
pyyaml>=6.0

# Date handling
python-dateutil>=2.8.0

# Optional: Rich terminal output
# rich>=13.0.0

# Optional: Progress bars
# tqdm>=4.60.0
"""

    req_path = workspace / '_tools' / 'requirements.txt'
    file_ops.write_file(req_path, content)
    return True
