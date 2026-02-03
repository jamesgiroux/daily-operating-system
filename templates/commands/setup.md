# /setup - Workspace Configuration

Interactive workspace configuration and verification for Daily Operating System.

## When to Use

- **First run**: After running setup wizard, use `/setup` to personalize
- **Reconfigure**: When you want to adjust folder structure, enable/disable skills, or update CLAUDE.md
- **Verify**: When something isn't working, use `/setup --verify` to check configuration
- **After updates**: After pulling new DailyOS features, verify everything still works

## Execution Modes

```
/setup              # Full interactive configuration
/setup --verify     # Quick verification check (read-only)
/setup --role       # Reconfigure folder structure for different role
/setup --skills     # Review and configure available skills
/setup --claude     # Update CLAUDE.md context file
```

## Step 1: Detect Workspace

First, verify we're in a valid DailyOS workspace:

```python
from pathlib import Path
import os

workspace = Path.cwd()
issues = []
info = []

# Check for key indicators of DailyOS workspace
required_markers = [
    ('CLAUDE.md', 'Context file'),
    ('_inbox', 'Inbox directory'),
    ('_today', 'Today directory'),
]

optional_markers = [
    ('.config/google/google_api.py', 'Google API script'),
    ('.claude/commands', 'Custom commands'),
    ('.claude/skills', 'Skills directory'),
]

for marker, name in required_markers:
    if not (workspace / marker).exists():
        issues.append(f"Missing: {name} ({marker})")

for marker, name in optional_markers:
    if (workspace / marker).exists():
        info.append(f"✓ {name}")
    else:
        info.append(f"○ {name} (optional)")
```

**If issues found:**
- Offer to initialize missing components
- If CLAUDE.md missing, jump to Step 4
- If directories missing, jump to Step 3

**If no issues:**
- Display current configuration summary
- Ask what user wants to configure

## Step 2: Configuration Menu

Present interactive menu using AskUserQuestion:

```
What would you like to configure?

Options:
1. Verify installation (check everything is working)
2. Update folder structure (change your role/workflow type)
3. Review skills (enable, disable, or learn about skills)
4. Update CLAUDE.md (change your personal context)
5. Check integrations (Google API, git, etc.)
6. Show current status (display full configuration)
```

## Step 3: Folder Structure Configuration

### Detect Current Role

```python
# Examine existing structure to determine current role
current_structure = 'unknown'

if (workspace / 'Accounts').exists():
    subdirs = [d.name for d in (workspace / 'Accounts').iterdir() if d.is_dir()]

    if 'Active' in subdirs and 'Qualified' in subdirs:
        current_structure = 'sales'
    elif 'Active' in subdirs and 'Inactive' in subdirs:
        current_structure = 'mid_market'
    elif 'Current' in subdirs and 'Previous' in subdirs:
        current_structure = 'tactical_custom'
    elif any((workspace / 'Accounts' / d / '00-Index.md').exists() for d in subdirs):
        current_structure = 'key_accounts'
```

### Role Selection

Present role options using AskUserQuestion:

```
Your work style determines folder structure. Choose what fits best:

**Customer/Account-Focused:**

1. **Key Accounts** (TAMs, Relationship Managers, Account Owners)
   - Dedicated folder per account with full 12-folder structure
   - Best for: 10-50 accounts you know deeply

2. **Sales** (AEs, BDRs, Sales Engineers)
   - Stage-based: Active / Qualified / Disqualified / Future
   - Best for: Pipeline management and prospecting

3. **Mid-Market** (High-volume account management)
   - Engagement-based: Active / Inactive / Watchlist
   - Best for: 100+ accounts with rotation

4. **Tactical/Custom** (On-demand, rotation-based)
   - Assignment-based: Current / Previous / Pool
   - Best for: Project-based or temporary assignments

**Project/Product-Focused:**

5. **Project Management** (PMs, Program Managers)
   - Projects by lifecycle: Active / Planning / Completed
   - Includes Stakeholders folder for cross-project relationships
   - Best for: Delivering defined outcomes with milestones

6. **Product Management** (Product Managers)
   - Features by stage: Discovery / In-Progress / Shipped
   - Includes Research folder for user research
   - Best for: Discovery-to-delivery product work

7. **Engineering** (Engineers, Tech Leads)
   - Projects with ADRs (Architecture Decision Records)
   - Includes Learning and Documentation folders
   - Best for: Technical projects and continuous learning

**Function-Focused:**

8. **Marketing** (Marketing Managers, Content)
   - Campaigns by stage: Active / Planned / Completed
   - Includes Content and Research folders
   - Best for: Campaign planning and execution

9. **Consulting / Strategy** (Consultants, Analysts)
   - Engagements: Active / Completed
   - Includes Frameworks and Research folders
   - Best for: Client engagements with deliverables

**Flexible:**

10. **General Knowledge Work** (Any role)
    - Pure PARA: Projects / Areas / Resources / Archive
    - Minimal structure, maximum flexibility
    - Best for: When none of the above fit
```

### Apply Role Structure

If changing role:

1. **Backup current structure** (if accounts exist):
   ```python
   # Create backup before restructuring
   import shutil
   from datetime import datetime

   backup_dir = workspace / 'Archive' / f'account-backup-{datetime.now().strftime("%Y%m%d")}'
   if (workspace / 'Accounts').exists():
       shutil.copytree(workspace / 'Accounts', backup_dir)
       print(f"Backed up current Accounts to {backup_dir}")
   ```

2. **Create new structure**:
   - For each role, create the appropriate top-level directories
   - Generate README.md with role-specific guidance

3. **Offer migration help**:
   - If accounts exist in old structure, offer to move them
   - Map old locations to new locations based on role

## Step 4: CLAUDE.md Configuration

The CLAUDE.md file tells Claude about you and your work.

### Check Existing CLAUDE.md

```python
claude_md = workspace / 'CLAUDE.md'
if claude_md.exists():
    content = claude_md.read_text()
    # Analyze what's configured
    has_about = '## About' in content or '## Profile' in content
    has_accounts = 'Accounts' in content or 'customers' in content.lower()
    has_workflow = 'workflow' in content.lower() or 'operating system' in content.lower()
```

### Interactive Profile Builder

Use AskUserQuestion to gather information:

**Question 1: Your Role**
```
What's your primary role?

1. Customer Success / Account Management
2. Sales / Business Development
3. Project Management
4. Marketing / Content
5. Consulting / Strategy
6. Engineering / Technical
7. Other (describe)
```

**Question 2: Work Style**
```
How do you work best?

1. Morning person (protect mornings for deep work)
2. Afternoon focus (meetings early, deep work later)
3. Varies by day (flexible schedule)
4. Always-on (work fits around availability)
```

**Question 3: Communication Preferences**
```
How do you prefer to communicate?

1. Slack-first (async, brief)
2. Email-detailed (longer, documented)
3. Meetings for important topics
4. Mix based on urgency
```

**Question 4: Key Accounts/Projects (if applicable)**
```
Do you manage specific accounts, clients, or projects?

1. Yes, I have a dedicated portfolio (enter list)
2. Yes, but they rotate frequently
3. No, my work is project-based
4. No, my work is functional/departmental
```

### Generate CLAUDE.md

Based on answers, generate or update CLAUDE.md:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code when working with this workspace.

## About [Name]

**Role**: [From Question 1]
**Work Style**: [From Question 2]

### Communication Preferences
[From Question 3]

### Working Hours
- Best focus time: [Derived from Question 2]
- Meeting preferences: [Derived from Question 3]

## Workspace Structure

[Generated based on role selection in Step 3]

## Current Focus

[If accounts/projects entered, list them here]

## Key Commands

| Command | Purpose |
|---------|---------|
| `/today` | Morning prep - calendar, meetings, actions |
| `/wrap` | End-of-day closure and impact capture |
| `/week` | Monday planning and review |
| `/month` | Monthly roll-up |
| `/quarter` | Quarterly evidence compilation |

## Notes

[Space for user to add custom notes and preferences]
```

## Step 5: Skills Review

### List Available Skills

```python
skills_dir = workspace / '.claude' / 'skills'
available_skills = []

if skills_dir.exists():
    for skill in skills_dir.iterdir():
        if skill.is_dir():
            skill_file = skill / 'SKILL.md'
            if skill_file.exists():
                # Parse skill metadata
                content = skill_file.read_text()
                name = skill.name
                # Extract description from first paragraph after #
                available_skills.append({
                    'name': name,
                    'path': str(skill),
                    'description': extract_description(content)
                })
```

### Display Skills Status

```
Available Skills:

✓ inbox
  Process documents through _inbox with AI enrichment

✓ strategy-consulting
  McKinsey-style strategic analysis and frameworks

✓ editorial
  Writing review and editorial workflows

Commands to configure:
- Enable a skill: "enable [skill-name]"
- Disable a skill: "disable [skill-name]"
- Learn about a skill: "describe [skill-name]"
```

### Skill Details

When user asks about a skill, show:
- Full description
- What triggers it
- What output it produces
- Dependencies (other skills, integrations)
- Example usage

## Step 6: Integration Check

### Google API

```python
google_api = workspace / '.config' / 'google' / 'google_api.py'
google_creds = workspace / '.config' / 'google' / 'credentials.json'
google_token = workspace / '.config' / 'google' / 'token.json'

google_status = {
    'script': google_api.exists(),
    'credentials': google_creds.exists(),
    'token': google_token.exists(),
    'authenticated': False
}

if all([google_status['script'], google_status['credentials'], google_status['token']]):
    # Test authentication
    result = subprocess.run(
        ['python3', str(google_api), 'calendar', 'list', '1'],
        capture_output=True, text=True
    )
    google_status['authenticated'] = result.returncode == 0
```

**Display Google Status:**
```
Google API Integration:

Script: ✓ Installed
Credentials: ✓ Present
Authentication: ✓ Working

Available:
- Calendar (read/write)
- Gmail (read/draft/labels)
- Sheets (read/write)
- Docs (read/write)

To reconfigure: Run `python3 advanced-start.py --google`
```

### Git Configuration

```python
import subprocess

git_status = {
    'is_repo': (workspace / '.git').exists(),
    'has_remote': False,
    'branch': None
}

if git_status['is_repo']:
    branch = subprocess.run(['git', 'branch', '--show-current'],
                           capture_output=True, text=True, cwd=workspace)
    git_status['branch'] = branch.stdout.strip()

    remote = subprocess.run(['git', 'remote', '-v'],
                           capture_output=True, text=True, cwd=workspace)
    git_status['has_remote'] = bool(remote.stdout.strip())
```

**Display Git Status:**
```
Git Configuration:

Repository: ✓ Initialized
Branch: master
Remote: ✓ origin (github.com/user/workspace)

Your workspace is version-controlled.
Changes will be tracked and can be pushed to remote.
```

## Step 7: Verification Summary

After all checks, display comprehensive summary:

```
╔══════════════════════════════════════════════════════════════╗
║                 Daily Operating System Status                 ║
╠══════════════════════════════════════════════════════════════╣
║                                                               ║
║  Workspace: /Users/you/Documents/DailyOS                     ║
║  Role: Key Accounts                                          ║
║  Structure: PARA + 12-folder accounts                        ║
║                                                               ║
║  ✓ CLAUDE.md configured                                      ║
║  ✓ Directory structure valid                                 ║
║  ✓ 7 commands available                                      ║
║  ✓ 3 skills enabled                                          ║
║  ✓ Google API authenticated                                  ║
║  ✓ Git repository configured                                 ║
║                                                               ║
║  Ready to use:                                               ║
║  • /today - Start your morning                               ║
║  • /wrap - Close your day                                    ║
║  • Drop files in _inbox/ for processing                      ║
║                                                               ║
╚══════════════════════════════════════════════════════════════╝
```

## Quick Verify Mode

When run with `--verify`, skip interactive steps and just check:

```python
def quick_verify(workspace):
    checks = []

    # Critical
    checks.append(('CLAUDE.md exists', (workspace / 'CLAUDE.md').exists()))
    checks.append(('_inbox directory', (workspace / '_inbox').exists()))
    checks.append(('_today directory', (workspace / '_today').exists()))

    # Commands
    commands_dir = workspace / '.claude' / 'commands'
    if commands_dir.exists():
        commands = list(commands_dir.glob('*.md'))
        checks.append((f'{len(commands)} commands available', len(commands) > 0))

    # Skills
    skills_dir = workspace / '.claude' / 'skills'
    if skills_dir.exists():
        skills = [d for d in skills_dir.iterdir() if d.is_dir()]
        checks.append((f'{len(skills)} skills available', len(skills) > 0))

    # Google API
    google_ok = (workspace / '.config' / 'google' / 'token.json').exists()
    checks.append(('Google API configured', google_ok))

    # Git
    git_ok = (workspace / '.git').exists()
    checks.append(('Git repository', git_ok))

    return checks
```

Output:
```
Quick Verification:

✓ CLAUDE.md exists
✓ _inbox directory
✓ _today directory
✓ 7 commands available
✓ 3 skills available
✓ Google API configured
✓ Git repository

All checks passed. Workspace is ready.
```

## Error Recovery

If verification fails, offer specific remediation:

```
Issue: CLAUDE.md not found

This file tells Claude about you and your workspace.
Without it, commands won't have context about your work.

Fix options:
1. Generate new CLAUDE.md interactively (recommended)
2. Copy template and edit manually
3. Skip (commands will work but without personalization)
```

## Related Commands

- `/today` - Daily operating system
- `/wrap` - End-of-day closure
- `/week` - Weekly review
- `/inbox` - Process inbox manually
