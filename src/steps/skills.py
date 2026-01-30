"""
Step 7: Skills and Commands Installation.

Installs Claude Code skills, commands, and agents into the workspace.
"""

from pathlib import Path
from typing import List, Dict, Any, Optional


# Available commands with descriptions
AVAILABLE_COMMANDS = {
    'today': {
        'name': '/today',
        'description': 'Morning dashboard - meeting prep, actions, email triage, look-ahead agendas',
        'category': 'core',
        'dependencies': ['google_api'],
    },
    'wrap': {
        'name': '/wrap',
        'description': 'End-of-day closure - reconcile actions, capture impacts, archive daily files',
        'category': 'core',
        'dependencies': [],
    },
    'week': {
        'name': '/week',
        'description': 'Monday weekly review - overview, hygiene alerts, pre-populate impact template',
        'category': 'core',
        'dependencies': ['google_api'],
    },
    'month': {
        'name': '/month',
        'description': 'Monthly roll-up - aggregate weekly impacts into monthly report',
        'category': 'extended',
        'dependencies': [],
    },
    'quarter': {
        'name': '/quarter',
        'description': 'Quarterly pre-population - pre-fill expectations with evidence',
        'category': 'extended',
        'dependencies': [],
    },
    'email-scan': {
        'name': '/email-scan',
        'description': 'Email inbox triage - surface important emails, draft responses, archive noise',
        'category': 'extended',
        'dependencies': ['google_api'],
    },
    'git-commit': {
        'name': '/git-commit',
        'description': 'Atomic commit workflow - stage, commit, push with meaningful messages',
        'category': 'extended',
        'dependencies': [],
    },
}

# Available skills with descriptions
AVAILABLE_SKILLS = {
    'inbox-processing': {
        'name': 'Inbox Processing',
        'description': 'Three-phase document flow (preparation, enrichment, delivery) to PARA structure',
        'category': 'core',
        'agents': ['file-organizer', 'integration-linker'],
    },
    'strategy-consulting': {
        'name': 'Strategy Consulting',
        'description': 'McKinsey-style strategic analysis with multi-agent workflow',
        'category': 'advanced',
        'agents': [
            'problem-framer',
            'framework-strategist',
            'red-team',
            'evidence-analyst',
            'executive-storyteller',
        ],
    },
    'editorial': {
        'name': 'Editorial',
        'description': 'Writing review standards with multi-stage review process',
        'category': 'advanced',
        'agents': [
            'writer-research',
            'writer-mechanical-review',
            'writer-structural-review',
            'writer-voice-review',
            'writer-craft-review',
            'writer-authenticity-review',
            'writer-challenger',
            'writer-scrutiny',
        ],
    },
}

# Core vs extended packages
CORE_COMMANDS = ['today', 'wrap', 'week']
CORE_SKILLS = ['inbox-processing']


def get_command_list(category: Optional[str] = None) -> List[Dict[str, Any]]:
    """
    Get list of available commands.

    Args:
        category: Filter by category ('core', 'extended') or None for all

    Returns:
        List of command dictionaries
    """
    commands = []
    for key, cmd in AVAILABLE_COMMANDS.items():
        if category is None or cmd['category'] == category:
            commands.append({'key': key, **cmd})
    return commands


def get_skill_list(category: Optional[str] = None) -> List[Dict[str, Any]]:
    """
    Get list of available skills.

    Args:
        category: Filter by category ('core', 'advanced') or None for all

    Returns:
        List of skill dictionaries
    """
    skills = []
    for key, skill in AVAILABLE_SKILLS.items():
        if category is None or skill['category'] == category:
            skills.append({'key': key, **skill})
    return skills


def install_command(workspace: Path, command_key: str, file_ops) -> bool:
    """
    Install a single command.

    Args:
        workspace: Root workspace path
        command_key: Command identifier
        file_ops: FileOperations instance

    Returns:
        True if installed successfully
    """
    if command_key not in AVAILABLE_COMMANDS:
        return False

    cmd = AVAILABLE_COMMANDS[command_key]
    cmd_path = workspace / '.claude' / 'commands' / f'{command_key}.md'

    # Create placeholder - actual content loaded from templates
    content = f"""# {cmd['name']}

{cmd['description']}

## When to Use

[Usage guidance will be added during setup]

## Execution Steps

[Steps will be configured based on your workspace]

---
*Installed by Daily Operating System Setup Wizard*
"""

    file_ops.write_file(cmd_path, content)
    return True


def install_skill(workspace: Path, skill_key: str, file_ops) -> bool:
    """
    Install a single skill with its agents.

    Args:
        workspace: Root workspace path
        skill_key: Skill identifier
        file_ops: FileOperations instance

    Returns:
        True if installed successfully
    """
    if skill_key not in AVAILABLE_SKILLS:
        return False

    skill = AVAILABLE_SKILLS[skill_key]
    skill_dir = workspace / '.claude' / 'skills' / skill_key

    # Create skill directory
    file_ops.create_directory(skill_dir)

    # Create SKILL.md
    skill_md_content = f"""# {skill['name']}

{skill['description']}

## Overview

[Skill overview will be added during setup]

## Agents

This skill includes the following agents:
{chr(10).join(f'- {agent}' for agent in skill['agents'])}

## Usage

[Usage instructions will be configured based on your workspace]

---
*Installed by Daily Operating System Setup Wizard*
"""

    file_ops.write_file(skill_dir / 'SKILL.md', skill_md_content)

    # Install agents for this skill
    for agent in skill['agents']:
        install_agent(workspace, agent, skill_key, file_ops)

    return True


def install_agent(workspace: Path, agent_key: str, skill_key: str, file_ops) -> bool:
    """
    Install a single agent.

    Args:
        workspace: Root workspace path
        agent_key: Agent identifier
        skill_key: Parent skill identifier
        file_ops: FileOperations instance

    Returns:
        True if installed successfully
    """
    agent_dir = workspace / '.claude' / 'agents'
    file_ops.create_directory(agent_dir)

    agent_path = agent_dir / f'{agent_key}.md'

    content = f"""# {agent_key}

Agent for {skill_key} skill.

## Purpose

[Agent purpose will be configured during setup]

## When to Use

[Usage guidance will be added]

---
*Installed by Daily Operating System Setup Wizard*
"""

    file_ops.write_file(agent_path, content)
    return True


def install_core_package(workspace: Path, file_ops) -> Dict[str, bool]:
    """
    Install the core package (essential commands and skills).

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance

    Returns:
        Dictionary of installation results
    """
    results = {}

    # Install core commands
    for cmd in CORE_COMMANDS:
        results[f'command:{cmd}'] = install_command(workspace, cmd, file_ops)

    # Install core skills
    for skill in CORE_SKILLS:
        results[f'skill:{skill}'] = install_skill(workspace, skill, file_ops)

    return results


def install_all_packages(workspace: Path, file_ops) -> Dict[str, bool]:
    """
    Install all available commands and skills.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance

    Returns:
        Dictionary of installation results
    """
    results = {}

    # Install all commands
    for cmd_key in AVAILABLE_COMMANDS:
        results[f'command:{cmd_key}'] = install_command(workspace, cmd_key, file_ops)

    # Install all skills
    for skill_key in AVAILABLE_SKILLS:
        results[f'skill:{skill_key}'] = install_skill(workspace, skill_key, file_ops)

    return results


def verify_installation(workspace: Path) -> Dict[str, Any]:
    """
    Verify skills and commands installation.

    Args:
        workspace: Root workspace path

    Returns:
        Verification results dictionary
    """
    results = {
        'commands': {},
        'skills': {},
        'agents': [],
    }

    # Check commands
    cmd_dir = workspace / '.claude' / 'commands'
    for cmd_key in AVAILABLE_COMMANDS:
        cmd_path = cmd_dir / f'{cmd_key}.md'
        results['commands'][cmd_key] = cmd_path.exists()

    # Check skills
    skill_dir = workspace / '.claude' / 'skills'
    for skill_key in AVAILABLE_SKILLS:
        skill_path = skill_dir / skill_key / 'SKILL.md'
        results['skills'][skill_key] = skill_path.exists()

    # Check agents
    agent_dir = workspace / '.claude' / 'agents'
    if agent_dir.exists():
        results['agents'] = [f.stem for f in agent_dir.glob('*.md')]

    return results
