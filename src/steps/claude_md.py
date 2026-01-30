"""
Step 6: CLAUDE.md Configuration Generation.

Creates the CLAUDE.md file that configures Claude Code for the workspace.
"""

from pathlib import Path
from typing import Dict, Any, Optional
from datetime import datetime


def get_questionnaire_prompts() -> list:
    """
    Get the list of questionnaire prompts for CLAUDE.md generation.

    Returns:
        List of prompt dictionaries
    """
    return [
        {
            'key': 'name',
            'prompt': 'Your name',
            'default': '',
            'required': False,
        },
        {
            'key': 'role',
            'prompt': 'Your role/job title',
            'default': '',
            'required': False,
        },
        {
            'key': 'energy_time',
            'prompt': 'When do you do your best work?',
            'type': 'choice',
            'options': [
                ('Morning', 'High energy in AM, fades in PM'),
                ('Afternoon', 'Peak performance midday'),
                ('Evening', 'Most productive later in day'),
                ('Varies', 'Depends on the day'),
            ],
            'default': 1,
        },
        {
            'key': 'comm_style',
            'prompt': 'Preferred communication style?',
            'type': 'choice',
            'options': [
                ('Direct', 'Straightforward, get to the point'),
                ('Diplomatic', 'Thoughtful, consider all angles'),
                ('Collaborative', 'Team-oriented, inclusive'),
            ],
            'default': 1,
        },
        {
            'key': 'focus',
            'prompt': 'What are you currently focused on?',
            'default': 'Professional development and productivity',
            'required': False,
        },
    ]


def generate_claude_md(
    workspace: Path,
    config: Dict[str, Any],
    include_google: bool = True,
    include_commands: bool = True
) -> str:
    """
    Generate CLAUDE.md content from configuration.

    Args:
        workspace: Root workspace path
        config: Configuration dictionary from questionnaire
        include_google: Whether to include Google API section
        include_commands: Whether to include commands section

    Returns:
        CLAUDE.md content as string
    """
    name = config.get('name', '')
    role = config.get('role', '')
    energy = config.get('energy_time', 'morning')
    comm_style = config.get('comm_style', 'direct')
    focus = config.get('focus', 'Professional development')

    # Build the content
    sections = []

    # Header
    sections.append('# CLAUDE.md')
    sections.append('')
    sections.append('This file provides guidance to Claude Code when working with this workspace.')
    sections.append('')

    # About section
    if name or role:
        sections.append(f'## About {name or "Me"}')
        sections.append('')
        if role:
            sections.append(f'**Role**: {role}')
            sections.append('')
        sections.append('**Working Style**:')
        sections.append(f'- Best work happens in the {energy}')
        sections.append(f'- Communication style: {comm_style}')
        sections.append('- [Add more preferences]')
        sections.append('')

    # Repository Purpose
    sections.append('## Repository Purpose')
    sections.append('')
    sections.append('Personal productivity workspace using the PARA organizational system.')
    sections.append('')

    # Directory Structure
    sections.append('## Directory Structure')
    sections.append('')
    sections.append('```')
    sections.append(f'{workspace.name}/')
    sections.append('├── Projects/     - Active initiatives with deadlines')
    sections.append('├── Areas/        - Ongoing responsibilities')
    sections.append('├── Resources/    - Reference materials')
    sections.append('├── Archive/      - Completed/inactive items')
    sections.append('├── _inbox/       - Unprocessed documents')
    sections.append('├── _today/       - Daily working files')
    sections.append('├── _templates/   - Document templates')
    sections.append('└── _tools/       - Automation scripts')
    sections.append('```')
    sections.append('')

    # Current Focus
    sections.append('## Current Focus')
    sections.append('')
    sections.append(focus)
    sections.append('')

    # Commands section
    if include_commands:
        sections.append('## Available Commands')
        sections.append('')
        sections.append('| Command | Purpose |')
        sections.append('|---------|---------|')
        sections.append('| /today | Morning dashboard - meeting prep, actions, email triage |')
        sections.append('| /wrap | End-of-day closure - reconcile actions, capture impacts |')
        sections.append('| /week | Weekly review - overview, hygiene alerts |')
        sections.append('| /month | Monthly roll-up - aggregate impacts |')
        sections.append('| /quarter | Quarterly review - pre-fill expectations |')
        sections.append('| /email-scan | Email triage - surface important, archive noise |')
        sections.append('')

    # Google API section
    if include_google:
        sections.append('## Google API Integration')
        sections.append('')
        sections.append('Claude has authenticated access to Google Workspace services via `.config/google/google_api.py`.')
        sections.append('')
        sections.append('**Available Commands**:')
        sections.append('```bash')
        sections.append('# Calendar')
        sections.append('.config/google/google_api.py calendar list [days]')
        sections.append('.config/google/google_api.py calendar get <event_id>')
        sections.append('.config/google/google_api.py calendar create <title> <start> <end>')
        sections.append('')
        sections.append('# Gmail')
        sections.append('.config/google/google_api.py gmail list [max]')
        sections.append('.config/google/google_api.py gmail search <query> [max]')
        sections.append('.config/google/google_api.py gmail draft <to> <subj> <body>')
        sections.append('')
        sections.append('# Sheets')
        sections.append('.config/google/google_api.py sheets get <id> <range>')
        sections.append('')
        sections.append('# Docs')
        sections.append('.config/google/google_api.py docs get <doc_id>')
        sections.append('```')
        sections.append('')

    # Guiding Principles
    sections.append('## Guiding Principles')
    sections.append('')
    sections.append('1. **Value shows up without asking** - The system does work before you arrive')
    sections.append('2. **Skip a day, nothing breaks** - No accumulated guilt from missed days')
    sections.append('3. **Incremental improvement** - Small, compounding gains over time')
    sections.append('')

    # Generated timestamp
    sections.append('---')
    sections.append(f'*Generated by Daily Operating System Setup Wizard on {datetime.now().strftime("%Y-%m-%d")}*')

    return '\n'.join(sections)


def generate_basic_template(workspace: Path) -> str:
    """
    Generate a basic CLAUDE.md template without questionnaire.

    Args:
        workspace: Root workspace path

    Returns:
        CLAUDE.md content as string
    """
    return generate_claude_md(
        workspace,
        {},
        include_google=True,
        include_commands=True
    )


def create_claude_md(workspace: Path, content: str, file_ops) -> bool:
    """
    Create the CLAUDE.md file.

    Args:
        workspace: Root workspace path
        content: CLAUDE.md content
        file_ops: FileOperations instance

    Returns:
        True if created successfully
    """
    claude_md_path = workspace / 'CLAUDE.md'
    file_ops.write_file(claude_md_path, content)
    return True


def verify_claude_md(workspace: Path) -> Dict[str, Any]:
    """
    Verify CLAUDE.md exists and has expected sections.

    Args:
        workspace: Root workspace path

    Returns:
        Verification results dictionary
    """
    claude_md_path = workspace / 'CLAUDE.md'

    if not claude_md_path.exists():
        return {
            'exists': False,
            'path': str(claude_md_path),
            'sections': [],
        }

    content = claude_md_path.read_text()

    # Check for expected sections
    expected_sections = [
        '## Repository Purpose',
        '## Directory Structure',
        '## Available Commands',
    ]

    found_sections = [s for s in expected_sections if s in content]

    return {
        'exists': True,
        'path': str(claude_md_path),
        'sections': found_sections,
        'missing_sections': [s for s in expected_sections if s not in content],
        'size': len(content),
    }
