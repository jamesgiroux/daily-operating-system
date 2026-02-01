"""
Step 9: Installation Verification.

Verifies that the setup completed successfully and all components are working.
"""

from pathlib import Path
from typing import Dict, Any, List, Tuple


def verify_directory_structure(workspace: Path) -> Tuple[bool, List[Dict[str, Any]]]:
    """
    Verify PARA directory structure exists.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (all_present, results)
    """
    required_dirs = [
        ('Projects', 'PARA: Active initiatives'),
        ('Areas', 'PARA: Ongoing responsibilities'),
        ('Resources', 'PARA: Reference materials'),
        ('Archive', 'PARA: Completed items'),
        ('_inbox', 'Support: Unprocessed documents'),
        ('_today', 'Support: Daily working files'),
        ('_templates', 'Support: Document templates'),
        ('_tools', 'Support: Automation scripts'),
    ]

    results = []
    all_present = True

    for dir_name, description in required_dirs:
        exists = (workspace / dir_name).exists()
        results.append({
            'name': dir_name,
            'description': description,
            'exists': exists,
            'status': 'ok' if exists else 'missing',
        })
        if not exists:
            all_present = False

    return all_present, results


def verify_configuration(workspace: Path) -> Tuple[bool, List[Dict[str, Any]]]:
    """
    Verify configuration files exist.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (all_present, results)
    """
    config_files = [
        ('CLAUDE.md', 'Claude Code configuration', True),
        ('.gitignore', 'Git ignore rules', False),
        ('.config/google/google_api.py', 'Google API helper', False),
        ('.config/google/credentials.json', 'Google credentials', False),
    ]

    results = []
    all_required_present = True

    for file_path, description, required in config_files:
        exists = (workspace / file_path).exists()
        results.append({
            'name': file_path,
            'description': description,
            'exists': exists,
            'required': required,
            'status': 'ok' if exists else ('missing' if required else 'optional'),
        })
        if required and not exists:
            all_required_present = False

    return all_required_present, results


def verify_commands(workspace: Path) -> Tuple[bool, List[Dict[str, Any]]]:
    """
    Verify Claude Code commands are installed.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (core_present, results)
    """
    commands = [
        ('today.md', '/today command', True),
        ('wrap.md', '/wrap command', True),
        ('week.md', '/week command', True),
        ('month.md', '/month command', False),
        ('quarter.md', '/quarter command', False),
        ('email-scan.md', '/email-scan command', False),
        ('git-commit.md', '/git-commit command', False),
    ]

    results = []
    core_present = True
    cmd_dir = workspace / '.claude' / 'commands'

    for file_name, description, core in commands:
        exists = (cmd_dir / file_name).exists()
        results.append({
            'name': file_name,
            'description': description,
            'exists': exists,
            'core': core,
            'status': 'ok' if exists else ('missing' if core else 'optional'),
        })
        if core and not exists:
            core_present = False

    return core_present, results


def verify_skills(workspace: Path) -> Tuple[bool, List[Dict[str, Any]]]:
    """
    Verify Claude Code skills are installed.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (any_installed, results)
    """
    skills = [
        ('inbox-processing', 'Document flow workflow'),
        ('strategy-consulting', 'Strategic analysis'),
        ('editorial', 'Writing review standards'),
    ]

    results = []
    any_installed = False
    skill_dir = workspace / '.claude' / 'skills'

    for skill_name, description in skills:
        skill_path = skill_dir / skill_name / 'SKILL.md'
        exists = skill_path.exists()
        results.append({
            'name': skill_name,
            'description': description,
            'exists': exists,
            'status': 'ok' if exists else 'not installed',
        })
        if exists:
            any_installed = True

    return any_installed, results


def verify_python_tools(workspace: Path) -> Tuple[bool, List[Dict[str, Any]]]:
    """
    Verify Python tools are installed.

    Note: Python tools are optional and not installed by the web wizard.
    They are only included if manually added or via CLI setup.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (any_installed, results)
    """
    tools = [
        ('prepare_inbox.py', 'Inbox preparation'),
        ('deliver_inbox.py', 'Inbox delivery'),
        ('generate_dashboard.py', 'Dashboard generation'),
    ]

    results = []
    any_installed = False
    tools_dir = workspace / '_tools'

    for tool_name, description in tools:
        tool_path = tools_dir / tool_name
        exists = tool_path.exists()
        results.append({
            'name': tool_name,
            'description': description,
            'exists': exists,
            'required': False,  # Mark as optional
            'status': 'ok' if exists else 'optional',  # Not a failure
        })
        if exists:
            any_installed = True

    return any_installed, results


def verify_git_setup(workspace: Path) -> Dict[str, Any]:
    """
    Verify Git repository setup.

    Args:
        workspace: Root workspace path

    Returns:
        Verification results dictionary
    """
    git_dir = workspace / '.git'

    return {
        'initialized': git_dir.exists(),
        'gitignore_exists': (workspace / '.gitignore').exists(),
        'status': 'ok' if git_dir.exists() else 'not initialized',
    }


def run_full_verification(workspace: Path) -> Dict[str, Any]:
    """
    Run complete installation verification.

    Args:
        workspace: Root workspace path

    Returns:
        Complete verification results
    """
    results = {
        'workspace': str(workspace),
        'workspace_exists': workspace.exists(),
        'sections': {},
        'summary': {
            'total_checks': 0,
            'passed': 0,
            'failed': 0,
            'optional_missing': 0,
        }
    }

    if not workspace.exists():
        results['summary']['failed'] = 1
        return results

    # Directory structure
    dirs_ok, dirs_results = verify_directory_structure(workspace)
    results['sections']['directories'] = {
        'passed': dirs_ok,
        'results': dirs_results,
    }

    # Configuration
    config_ok, config_results = verify_configuration(workspace)
    results['sections']['configuration'] = {
        'passed': config_ok,
        'results': config_results,
    }

    # Commands
    cmds_ok, cmds_results = verify_commands(workspace)
    results['sections']['commands'] = {
        'passed': cmds_ok,
        'results': cmds_results,
    }

    # Skills
    skills_ok, skills_results = verify_skills(workspace)
    results['sections']['skills'] = {
        'passed': skills_ok,
        'results': skills_results,
    }

    # Python tools
    tools_ok, tools_results = verify_python_tools(workspace)
    results['sections']['python_tools'] = {
        'passed': tools_ok,
        'results': tools_results,
    }

    # Git
    results['sections']['git'] = verify_git_setup(workspace)

    # Calculate summary
    for section_name, section in results['sections'].items():
        if 'results' in section:
            for item in section['results']:
                results['summary']['total_checks'] += 1
                status = item.get('status', '')
                if status == 'ok':
                    results['summary']['passed'] += 1
                elif status in ('missing', 'not installed', 'not initialized'):
                    if item.get('required', True) or item.get('core', False):
                        results['summary']['failed'] += 1
                    else:
                        results['summary']['optional_missing'] += 1
                elif status == 'optional':
                    results['summary']['optional_missing'] += 1

    return results


def get_verification_summary(results: Dict[str, Any]) -> str:
    """
    Generate a human-readable verification summary.

    Args:
        results: Results from run_full_verification

    Returns:
        Formatted summary string
    """
    lines = []
    lines.append("Installation Verification Summary")
    lines.append("=" * 40)
    lines.append("")

    summary = results['summary']
    lines.append(f"Total checks: {summary['total_checks']}")
    lines.append(f"Passed: {summary['passed']}")
    lines.append(f"Failed: {summary['failed']}")
    lines.append(f"Optional missing: {summary['optional_missing']}")
    lines.append("")

    if summary['failed'] == 0:
        lines.append("Status: READY TO USE")
        lines.append("")
        lines.append("Your Daily Operating System is configured.")
        lines.append("Run /today to start your first daily dashboard!")
    else:
        lines.append("Status: INCOMPLETE")
        lines.append("")
        lines.append("Some required components are missing.")
        lines.append("Re-run advanced-start.py to complete installation.")

    return "\n".join(lines)
