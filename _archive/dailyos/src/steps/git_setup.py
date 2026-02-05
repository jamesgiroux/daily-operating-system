"""
Step 4: Git Repository Setup.

Initializes a Git repository with appropriate .gitignore.
"""

import subprocess
from pathlib import Path
from typing import Tuple, Optional


# Default .gitignore content
GITIGNORE_CONTENT = """# Credentials and secrets
.config/google/token.json
.config/google/credentials.json
*.credentials
*.secret
.env
.env.local
.env.*.local

# OS files
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# Editor files
*.swp
*.swo
*~
.idea/
.vscode/
*.sublime-project
*.sublime-workspace

# Python
__pycache__/
*.py[cod]
*$py.class
.venv/
venv/
env/
.Python
*.egg-info/
dist/
build/

# Node (if any)
node_modules/

# Temporary files
*.tmp
*.bak
*.log

# Backup files created by setup wizard
*.bak.*
"""


def is_git_repo(path: Path) -> bool:
    """Check if path is inside a Git repository."""
    return (path / '.git').exists()


def init_git_repo(workspace: Path) -> Tuple[bool, Optional[str]]:
    """
    Initialize a new Git repository.

    Args:
        workspace: Path to initialize as Git repo

    Returns:
        Tuple of (success, error_message)
    """
    try:
        result = subprocess.run(
            ['git', 'init'],
            cwd=workspace,
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode != 0:
            return False, result.stderr.strip()
        return True, None
    except FileNotFoundError:
        return False, "Git is not installed"
    except subprocess.TimeoutExpired:
        return False, "Git init timed out"
    except Exception as e:
        return False, str(e)


def create_gitignore(workspace: Path, file_ops) -> bool:
    """
    Create .gitignore file.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance

    Returns:
        True if created successfully
    """
    gitignore_path = workspace / '.gitignore'
    file_ops.write_file(gitignore_path, GITIGNORE_CONTENT)
    return True


def create_initial_commit(workspace: Path) -> Tuple[bool, Optional[str]]:
    """
    Create the initial commit with .gitignore.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (success, error_message)
    """
    try:
        # Stage .gitignore
        result = subprocess.run(
            ['git', 'add', '.gitignore'],
            cwd=workspace,
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode != 0:
            return False, f"git add failed: {result.stderr.strip()}"

        # Create commit
        result = subprocess.run(
            ['git', 'commit', '-m', 'Initial commit: Add .gitignore'],
            cwd=workspace,
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode != 0:
            return False, f"git commit failed: {result.stderr.strip()}"

        return True, None

    except subprocess.TimeoutExpired:
        return False, "Git operation timed out"
    except Exception as e:
        return False, str(e)


def get_git_remote_instructions(workspace: Path) -> str:
    """
    Get instructions for setting up a remote repository.

    Args:
        workspace: Root workspace path

    Returns:
        Formatted instruction string
    """
    return f"""
To back up your workspace to GitHub:

1. Create a new private repository on GitHub
   (Do NOT initialize with README or .gitignore)

2. Add the remote and push:

   cd {workspace}
   git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPO.git
   git branch -M main
   git push -u origin main

3. For SSH authentication (recommended):

   git remote set-url origin git@github.com:YOUR_USERNAME/YOUR_REPO.git

Your workspace will now be backed up to GitHub!
"""


def check_git_status(workspace: Path) -> dict:
    """
    Check the status of the Git repository.

    Args:
        workspace: Root workspace path

    Returns:
        Dictionary with status information
    """
    status = {
        'is_repo': False,
        'has_commits': False,
        'has_remote': False,
        'branch': None,
        'remote_url': None,
    }

    if not (workspace / '.git').exists():
        return status

    status['is_repo'] = True

    try:
        # Check for commits
        result = subprocess.run(
            ['git', 'rev-parse', 'HEAD'],
            cwd=workspace,
            capture_output=True,
            text=True,
            timeout=10
        )
        status['has_commits'] = result.returncode == 0

        # Get current branch
        result = subprocess.run(
            ['git', 'branch', '--show-current'],
            cwd=workspace,
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode == 0:
            status['branch'] = result.stdout.strip()

        # Check for remote
        result = subprocess.run(
            ['git', 'remote', 'get-url', 'origin'],
            cwd=workspace,
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode == 0:
            status['has_remote'] = True
            status['remote_url'] = result.stdout.strip()

    except Exception:
        pass

    return status
