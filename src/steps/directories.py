"""
Step 3: Directory Structure Creation.

Creates the PARA-based directory structure for the productivity workspace.
Supports different organizational structures for different roles.
"""

from pathlib import Path
from typing import List, Dict


# Core PARA directories
PARA_DIRECTORIES = [
    'Projects',
    'Areas',
    'Resources',
    'Archive',
]

# Role-specific account structures
ROLE_STRUCTURES = {
    'customer_success': {
        'name': 'Customer Success',
        'description': 'TAMs, RMs, CSMs, AOs: Dedicated portfolio with full context',
        'directories': [
            'Accounts',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Customer-Information',
            '02-Meetings',
            '03-Call-Transcripts',
            '04-Action-Items',
            '05-Projects',
            '06-Integrations',
            '07-Reporting',
            '08-Health-Reviews',
            '09-Incidents',
            '10-Decisions',
            '11-Commercial',
            '12-P2s',
            '_attachments',
        ],
        'readme': '''# Accounts

Your dedicated account portfolio. Each account has its own folder with
the full 12-folder structure for comprehensive relationship management.

## Structure

Create a folder for each account you own:
```
Accounts/
├── ClientA/
│   ├── 00-Index.md           # Account overview and navigation
│   ├── 01-Customer-Information/  # Stakeholder maps, org context
│   ├── 02-Meetings/          # Meeting summaries and agendas
│   ├── 03-Call-Transcripts/  # Full meeting transcripts
│   ├── 04-Action-Items/      # Task tracking and follow-ups
│   ├── 05-Projects/          # Account-specific initiatives
│   ├── 06-Integrations/      # Technical integration docs
│   ├── 07-Reporting/         # Customer intelligence
│   ├── 08-Health-Reviews/    # Account health assessments
│   ├── 09-Incidents/         # Issue tracking and post-mortems
│   ├── 10-Decisions/         # Decision records and outcomes
│   ├── 11-Commercial/        # Pricing, contracts, renewals
│   ├── 12-P2s/               # Internal posts about account
│   └── _attachments/         # Supporting files and assets
└── ClientB/
    └── ...
```

## Philosophy

Each account is a relationship to nurture over time. The full structure
ensures nothing falls through the cracks across meetings, projects, and
commercial cycles.
'''
    },
    'sales': {
        'name': 'Sales',
        'description': 'AEs, BDRs, SEs: Stage-based Active/Qualified/Future',
        'directories': [
            'Accounts/Active',
            'Accounts/Qualified',
            'Accounts/Disqualified',
            'Accounts/Future',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Contact-Information',
            '02-Meetings',
            '03-Call-Transcripts',
            '04-Action-Items',
            '05-Discovery',
            '06-Proposals',
            '_attachments',
        ],
        'readme': '''# Accounts

Organized by prospect stage for sales workflow.

## Folders

- **Active/** - Currently pursuing (in active sales cycle)
- **Qualified/** - Handed off to sales or closed-won
- **Disqualified/** - Not a fit (documented why)
- **Future/** - Nurture pipeline (timing not right)

## Structure

Each account folder within a stage:
```
Active/
└── ProspectA/
    ├── 00-Index.md           # Prospect overview
    ├── 01-Contact-Information/   # Key contacts and org chart
    ├── 02-Meetings/          # Meeting notes
    ├── 03-Call-Transcripts/  # Discovery and demo calls
    ├── 04-Action-Items/      # Follow-ups and next steps
    ├── 05-Discovery/         # Pain points, requirements
    ├── 06-Proposals/         # Quotes and proposals sent
    └── _attachments/         # Decks, documents shared
```

## Workflow

Move accounts between folders as their status changes.
When a prospect advances, drag their folder to the next stage.
'''
    },
    'project_management': {
        'name': 'Project Management',
        'description': 'PMs, Program Managers: Project-centric with stakeholders',
        'directories': [
            'Projects/Active',
            'Projects/Planning',
            'Projects/Completed',
            'Stakeholders',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Project-Information',
            '02-Meetings',
            '03-Call-Transcripts',
            '04-Action-Items',
            '05-Milestones',
            '06-Risks-Issues',
            '07-Decisions',
            '08-Status-Reports',
            '_attachments',
        ],
        'readme': '''# Projects

Organized by project lifecycle stage.

## Folders

- **Active/** - Currently executing (in delivery phase)
- **Planning/** - In planning or initiation phase
- **Completed/** - Delivered projects (reference and lessons learned)

## Structure

Each project folder:
```
Active/
└── Website-Redesign/
    ├── 00-Index.md           # Project overview, quick links
    ├── 01-Project-Information/   # Charter, scope, RACI
    ├── 02-Meetings/          # Meeting summaries
    ├── 03-Call-Transcripts/  # Full meeting transcripts
    ├── 04-Action-Items/      # Tasks and follow-ups
    ├── 05-Milestones/        # Key deliverables and dates
    ├── 06-Risks-Issues/      # Risk register, issue log
    ├── 07-Decisions/         # Decision records
    ├── 08-Status-Reports/    # Weekly/monthly status
    └── _attachments/         # Supporting documents
```

## Stakeholders Folder

Track key stakeholders across all projects:
```
Stakeholders/
├── Executive-Sponsor.md     # Preferences, communication style
├── Tech-Lead.md
└── Business-Owner.md
```
'''
    },
    'product_management': {
        'name': 'Product Management',
        'description': 'Product Managers: Feature-centric with discovery and delivery',
        'directories': [
            'Products',
            'Features/Discovery',
            'Features/In-Progress',
            'Features/Shipped',
            'Research',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Product-Information',
            '02-Meetings',
            '03-User-Research',
            '04-Requirements',
            '05-Decisions',
            '_attachments',
        ],
        'readme': '''# Products & Features

Organized around product discovery and delivery.

## Folders

- **Products/** - Product-level context and strategy
- **Features/Discovery/** - Features being researched and defined
- **Features/In-Progress/** - Features in development
- **Features/Shipped/** - Launched features (learnings, metrics)
- **Research/** - User research, competitive analysis

## Structure

Each feature folder:
```
Features/In-Progress/
└── AI-Recommendations/
    ├── 00-Index.md           # Feature overview
    ├── 01-Product-Information/   # PRD, specs
    ├── 02-Meetings/          # Design reviews, syncs
    ├── 03-User-Research/     # Interviews, testing
    ├── 04-Requirements/      # User stories, acceptance criteria
    ├── 05-Decisions/         # Technical and product decisions
    └── _attachments/         # Mockups, diagrams
```

## Workflow

Move features between folders as they progress through discovery → development → ship.
'''
    },
    'marketing': {
        'name': 'Marketing',
        'description': 'Marketing Managers: Campaign and content-centric',
        'directories': [
            'Campaigns/Active',
            'Campaigns/Planned',
            'Campaigns/Completed',
            'Content',
            'Research',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Campaign-Brief',
            '02-Meetings',
            '03-Assets',
            '04-Action-Items',
            '05-Performance',
            '_attachments',
        ],
        'readme': '''# Campaigns & Content

Organized around marketing campaigns and content production.

## Folders

- **Campaigns/Active/** - Currently running campaigns
- **Campaigns/Planned/** - Upcoming campaigns in planning
- **Campaigns/Completed/** - Past campaigns (with results)
- **Content/** - Evergreen content and brand assets
- **Research/** - Market research, competitive intelligence

## Structure

Each campaign folder:
```
Campaigns/Active/
└── Q2-Product-Launch/
    ├── 00-Index.md           # Campaign overview, KPIs
    ├── 01-Campaign-Brief/    # Strategy, messaging, audience
    ├── 02-Meetings/          # Planning meetings, reviews
    ├── 03-Assets/            # Creative assets, copy
    ├── 04-Action-Items/      # Tasks and deadlines
    ├── 05-Performance/       # Metrics and reporting
    └── _attachments/         # Designs, vendor docs
```

## Workflow

Move campaigns through Planned → Active → Completed as they progress.
'''
    },
    'engineering': {
        'name': 'Engineering',
        'description': 'Engineers, Tech Leads: Codebase and sprint-centric',
        'directories': [
            'Projects/Active',
            'Projects/Backlog',
            'Projects/Completed',
            'Documentation',
            'Learning',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Technical-Specs',
            '02-Meetings',
            '03-Notes',
            '04-Action-Items',
            '05-ADRs',
            '_attachments',
        ],
        'readme': '''# Engineering Projects

Organized around technical projects and learning.

## Folders

- **Projects/Active/** - Currently working on
- **Projects/Backlog/** - Queued for future work
- **Projects/Completed/** - Finished projects (for reference)
- **Documentation/** - System docs, runbooks, guides
- **Learning/** - Technical learning, courses, experiments

## Structure

Each project folder:
```
Projects/Active/
└── API-Refactor/
    ├── 00-Index.md           # Project overview, goals
    ├── 01-Technical-Specs/   # Design docs, diagrams
    ├── 02-Meetings/          # Sprint planning, reviews
    ├── 03-Notes/             # Working notes, research
    ├── 04-Action-Items/      # Tasks, blockers
    ├── 05-ADRs/              # Architecture Decision Records
    └── _attachments/         # Diagrams, screenshots
```

## ADRs (Architecture Decision Records)

Track technical decisions with context:
- What was decided
- Why (context and constraints)
- Consequences and trade-offs
'''
    },
    'consulting': {
        'name': 'Consulting / Strategy',
        'description': 'Consultants, Analysts: Engagement and deliverable-centric',
        'directories': [
            'Engagements/Active',
            'Engagements/Completed',
            'Frameworks',
            'Research',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Engagement-Information',
            '02-Meetings',
            '03-Call-Transcripts',
            '04-Action-Items',
            '05-Analysis',
            '06-Deliverables',
            '07-Decisions',
            '_attachments',
        ],
        'readme': '''# Consulting Engagements

Organized around client engagements and deliverables.

## Folders

- **Engagements/Active/** - Current client work
- **Engagements/Completed/** - Past engagements (reference)
- **Frameworks/** - Reusable frameworks and templates
- **Research/** - Industry research, benchmarks

## Structure

Each engagement folder:
```
Engagements/Active/
└── Acme-Digital-Strategy/
    ├── 00-Index.md           # Engagement overview
    ├── 01-Engagement-Information/  # SOW, stakeholders
    ├── 02-Meetings/          # Client meetings
    ├── 03-Call-Transcripts/  # Interview transcripts
    ├── 04-Action-Items/      # Tasks and follow-ups
    ├── 05-Analysis/          # Working analysis
    ├── 06-Deliverables/      # Final outputs
    ├── 07-Decisions/         # Client decisions
    └── _attachments/         # Data, documents
```

## Workflow

Engagements move to Completed when delivered. Keep frameworks folder updated with reusable assets.
'''
    },
    'general': {
        'name': 'General Knowledge Work',
        'description': 'Flexible structure for any knowledge worker',
        'directories': [
            'Projects',
            'Areas',
            'Resources',
            'Archive',
        ],
        'account_subdirectories': [
            '00-Index.md',
            '01-Information',
            '02-Meetings',
            '03-Notes',
            '04-Action-Items',
            '_attachments',
        ],
        'readme': '''# PARA Structure

Flexible organization for any knowledge work.

## Folders

- **Projects/** - Active initiatives with defined outcomes
- **Areas/** - Ongoing responsibilities (no end date)
- **Resources/** - Reference materials and information
- **Archive/** - Completed or inactive items

## Structure

Each project or area folder:
```
Projects/
└── Website-Redesign/
    ├── 00-Index.md           # Overview and quick links
    ├── 01-Information/       # Context and background
    ├── 02-Meetings/          # Meeting notes
    ├── 03-Notes/             # Working notes
    ├── 04-Action-Items/      # Tasks and follow-ups
    └── _attachments/         # Supporting files
```

## Philosophy

PARA is about organizing by actionability:
- **Projects** = outcomes you're actively working toward
- **Areas** = standards you're maintaining
- **Resources** = information you might need
- **Archive** = things you're done with

Move items between folders as their status changes.
'''
    },
}

# Supporting directories
SUPPORT_DIRECTORIES = [
    '_inbox',
    '_today',
    '_today/tasks',
    '_today/archive',
    '_today/90-agenda-needed',
    '_templates',
    '_tools',
    '_reference',
]

# Configuration directories
CONFIG_DIRECTORIES = [
    '.config/google',
    '.claude/commands',
    '.claude/skills',
    '.claude/agents',
]

# All directories combined
ALL_DIRECTORIES = PARA_DIRECTORIES + SUPPORT_DIRECTORIES + CONFIG_DIRECTORIES


def get_directory_descriptions() -> Dict[str, str]:
    """
    Get descriptions for each directory.

    Returns:
        Dictionary mapping directory name to description
    """
    return {
        # PARA
        'Projects': 'Active initiatives with defined outcomes and deadlines',
        'Areas': 'Ongoing responsibilities requiring maintenance',
        'Resources': 'Reference materials and information',
        'Archive': 'Completed or inactive items',

        # Support
        '_inbox': 'Unprocessed documents awaiting triage',
        '_today': 'Daily working files and meeting prep',
        '_today/tasks': 'Persistent task tracking (survives daily archive)',
        '_today/archive': 'Previous days\' files (processed by /week)',
        '_today/90-agenda-needed': 'Draft agendas for upcoming meetings',
        '_templates': 'Reusable document templates',
        '_tools': 'Python automation scripts',
        '_reference': 'Standards and guidelines',

        # Config
        '.config/google': 'Google API credentials and scripts',
        '.claude/commands': 'Claude Code command definitions',
        '.claude/skills': 'Claude Code skill packages',
        '.claude/agents': 'Claude Code agent definitions',
    }


def get_role_choices() -> List[Dict[str, str]]:
    """
    Get the list of role choices for the user.

    Returns:
        List of dictionaries with 'key', 'name', and 'description'
    """
    return [
        {
            'key': role_key,
            'name': role_info['name'],
            'description': role_info['description'],
        }
        for role_key, role_info in ROLE_STRUCTURES.items()
    ]


def create_all_directories(workspace: Path, file_ops, role: str = 'customer_success') -> List[str]:
    """
    Create all directories in the workspace.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance for tracking
        role: User role (affects account structure)

    Returns:
        List of created directory paths (relative to workspace)
    """
    created = []

    # Create PARA directories
    for dir_path in PARA_DIRECTORIES:
        full_path = workspace / dir_path
        if not full_path.exists():
            file_ops.create_directory(full_path)
            created.append(dir_path)

    # Create role-specific account directories
    role_info = ROLE_STRUCTURES.get(role, ROLE_STRUCTURES['customer_success'])
    for dir_path in role_info.get('directories', []):
        full_path = workspace / dir_path
        if not full_path.exists():
            file_ops.create_directory(full_path)
            created.append(dir_path)

    # Create Accounts README with role-specific content
    accounts_readme = workspace / 'Accounts' / 'README.md'
    if not accounts_readme.exists():
        accounts_readme.parent.mkdir(parents=True, exist_ok=True)
        with open(accounts_readme, 'w') as f:
            f.write(role_info.get('readme', '# Accounts\n'))

    # Create support directories
    for dir_path in SUPPORT_DIRECTORIES:
        full_path = workspace / dir_path
        if not full_path.exists():
            file_ops.create_directory(full_path)
            created.append(dir_path)

    # Create config directories
    for dir_path in CONFIG_DIRECTORIES:
        full_path = workspace / dir_path
        if not full_path.exists():
            file_ops.create_directory(full_path)
            created.append(dir_path)

    return created


def get_account_subdirectories(role: str) -> List[str]:
    """
    Get the list of subdirectories to create within each account folder.

    Different roles have different account folder structures optimized
    for their workflow.

    Args:
        role: User role key

    Returns:
        List of subdirectory names/files to create within account folders
    """
    role_info = ROLE_STRUCTURES.get(role, ROLE_STRUCTURES['customer_success'])
    return role_info.get('account_subdirectories', [])


def create_example_account(workspace: Path, file_ops, role: str, account_name: str = 'ExampleAccount') -> List[str]:
    """
    Create an example account folder with role-appropriate subdirectories.

    This helps users understand the structure and provides a template
    for their real accounts.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance for tracking
        role: User role (determines subdirectory structure)
        account_name: Name for the example account folder

    Returns:
        List of created paths (relative to workspace)
    """
    created = []
    role_info = ROLE_STRUCTURES.get(role, ROLE_STRUCTURES['customer_success'])

    # Determine the base path based on role structure
    # For customer_success: Accounts/ExampleAccount/
    # For sales: Accounts/Active/ExampleAccount/
    # For mid_market: Accounts/Active/ExampleAccount/
    # For tactical_custom: Accounts/Current/ExampleAccount/

    role_dirs = role_info.get('directories', ['Accounts'])
    if len(role_dirs) > 1:
        # Use the first directory (Active or Current) for the example
        base_dir = role_dirs[0]
    else:
        base_dir = role_dirs[0]

    account_path = workspace / base_dir / account_name

    if not account_path.exists():
        file_ops.create_directory(account_path)
        created.append(f'{base_dir}/{account_name}')

    # Create role-specific subdirectories
    subdirs = role_info.get('account_subdirectories', [])
    for subdir in subdirs:
        if subdir.endswith('.md'):
            # It's a file, create it with template content
            file_path = account_path / subdir
            if not file_path.exists():
                with open(file_path, 'w') as f:
                    if subdir == '00-Index.md':
                        f.write(f'# {account_name}\n\nAccount overview and navigation.\n')
                    else:
                        f.write(f'# {subdir.replace(".md", "").replace("00-", "")}\n\n')
                created.append(f'{base_dir}/{account_name}/{subdir}')
        else:
            # It's a directory
            dir_path = account_path / subdir
            if not dir_path.exists():
                file_ops.create_directory(dir_path)
                created.append(f'{base_dir}/{account_name}/{subdir}')

    return created


def verify_directory_structure(workspace: Path) -> Dict[str, bool]:
    """
    Verify that all expected directories exist.

    Args:
        workspace: Root workspace path

    Returns:
        Dictionary mapping directory path to exists boolean
    """
    results = {}
    for dir_path in ALL_DIRECTORIES:
        full_path = workspace / dir_path
        results[dir_path] = full_path.exists()
    return results


def get_directory_tree_display(workspace: Path) -> str:
    """
    Generate a tree-style display of the directory structure.

    Args:
        workspace: Root workspace path

    Returns:
        Formatted string showing directory tree
    """
    lines = [f'{workspace.name}/']

    descriptions = get_directory_descriptions()

    # Show PARA directories
    for i, d in enumerate(PARA_DIRECTORIES):
        prefix = '├── ' if i < len(PARA_DIRECTORIES) - 1 or SUPPORT_DIRECTORIES else '└── '
        desc = descriptions.get(d, '')
        lines.append(f'{prefix}{d}/'.ljust(20) + f'# {desc}' if desc else f'{prefix}{d}/')

    # Show support directories (excluding nested ones)
    support_top = [d for d in SUPPORT_DIRECTORIES if '/' not in d]
    for i, d in enumerate(support_top):
        is_last = i == len(support_top) - 1 and not CONFIG_DIRECTORIES
        prefix = '└── ' if is_last else '├── '
        desc = descriptions.get(d, '')
        lines.append(f'{prefix}{d}/'.ljust(20) + f'# {desc}' if desc else f'{prefix}{d}/')

        # Show nested directories
        nested = [n for n in SUPPORT_DIRECTORIES if n.startswith(d + '/')]
        for j, n in enumerate(nested):
            nested_name = n.split('/')[-1]
            nested_prefix = '│   └── ' if j == len(nested) - 1 else '│   ├── '
            if is_last:
                nested_prefix = nested_prefix.replace('│', ' ')
            desc = descriptions.get(n, '')
            lines.append(f'{nested_prefix}{nested_name}/'.ljust(20) + f'# {desc}' if desc else f'{nested_prefix}{nested_name}/')

    # Show config directories (hidden)
    lines.append('└── .config/')
    lines.append('    └── google/'.ljust(20) + '# Google API credentials')
    lines.append('└── .claude/')
    lines.append('    ├── commands/'.ljust(20) + '# Slash commands')
    lines.append('    ├── skills/'.ljust(20) + '# Skill packages')
    lines.append('    └── agents/'.ljust(20) + '# Agent definitions')

    return '\n'.join(lines)
