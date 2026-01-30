"""
Pytest configuration and fixtures for DailyOS setup wizard tests.
"""

import pytest
import sys
import shutil
from pathlib import Path
from argparse import Namespace
from unittest.mock import Mock, patch


# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))


@pytest.fixture
def temp_workspace(tmp_path):
    """Create a temporary workspace directory."""
    workspace = tmp_path / "test_workspace"
    workspace.mkdir()
    return workspace


@pytest.fixture
def existing_workspace(tmp_path):
    """Create a workspace with existing structure."""
    workspace = tmp_path / "existing_workspace"
    workspace.mkdir()

    # Create some existing content
    (workspace / "Projects").mkdir()
    (workspace / "Areas").mkdir()
    (workspace / "_inbox").mkdir()
    (workspace / "existing_file.md").write_text("# Existing content\n")

    return workspace


@pytest.fixture
def mock_args():
    """Create mock argparse namespace with defaults."""
    return Namespace(
        workspace=None,
        google=False,
        verify=False,
        quick=False,
        verbose=False
    )


@pytest.fixture
def mock_args_with_workspace(temp_workspace):
    """Create mock args with workspace path."""
    return Namespace(
        workspace=str(temp_workspace),
        google=False,
        verify=False,
        quick=False,
        verbose=False
    )


@pytest.fixture
def mock_args_quick(temp_workspace):
    """Create mock args for quick setup."""
    return Namespace(
        workspace=str(temp_workspace),
        google=False,
        verify=False,
        quick=True,
        verbose=False
    )


@pytest.fixture
def mock_args_verbose(temp_workspace):
    """Create mock args with verbose flag."""
    return Namespace(
        workspace=str(temp_workspace),
        google=False,
        verify=False,
        quick=False,
        verbose=True
    )


@pytest.fixture
def mock_args_tilde():
    """Create mock args with tilde path."""
    return Namespace(
        workspace="~/test_productivity",
        google=False,
        verify=False,
        quick=False,
        verbose=False
    )


@pytest.fixture
def templates_dir(tmp_path):
    """Create mock templates directory structure."""
    templates = tmp_path / "templates"
    templates.mkdir()

    # Commands
    commands = templates / "commands"
    commands.mkdir()
    for cmd in ['today', 'wrap', 'week', 'month', 'quarter', 'email-scan', 'git-commit', 'setup']:
        (commands / f"{cmd}.md").write_text(f"# /{cmd}\n\nMock command content.\n")

    # Skills
    skills = templates / "skills"
    skills.mkdir()
    for skill in ['inbox-processing', 'strategy-consulting', 'editorial']:
        skill_dir = skills / skill
        skill_dir.mkdir()
        (skill_dir / "SKILL.md").write_text(f"# {skill}\n\nMock skill content.\n")

    # Agents
    agents = templates / "agents"
    agents.mkdir()
    agent_categories = ['csm', 'strategy', 'editorial']
    for category in agent_categories:
        cat_dir = agents / category
        cat_dir.mkdir()
        (cat_dir / "agent.md").write_text(f"# {category} agent\n\nMock agent.\n")

    # Scripts
    scripts = templates / "scripts"
    scripts.mkdir()

    inbox_scripts = scripts / "inbox"
    inbox_scripts.mkdir()
    (inbox_scripts / "prepare_inbox.py").write_text("# Mock prepare inbox\n")
    (inbox_scripts / "deliver_inbox.py").write_text("# Mock deliver inbox\n")

    accounts_scripts = scripts / "accounts"
    accounts_scripts.mkdir()
    (accounts_scripts / "generate_account_dashboard.py").write_text("# Mock dashboard\n")

    google_scripts = scripts / "google"
    google_scripts.mkdir()
    (google_scripts / "google_api.py").write_text("# Mock google api\n")

    return templates


@pytest.fixture
def mock_validators():
    """Mock all validator functions to return success."""
    with patch('utils.validators.validate_python_version') as mock_py, \
         patch('utils.validators.validate_command_exists') as mock_cmd, \
         patch('utils.validators.validate_path') as mock_path, \
         patch('utils.validators.validate_directory_writable') as mock_dir:

        mock_py.return_value = (True, "3.11.0", None)
        mock_cmd.return_value = (True, "claude 1.0.0", None)
        mock_path.return_value = (True, None)
        mock_dir.return_value = (True, None)

        yield {
            'python': mock_py,
            'command': mock_cmd,
            'path': mock_path,
            'directory': mock_dir
        }


@pytest.fixture
def mock_prompts():
    """Mock all interactive prompt functions."""
    with patch('ui.prompts.confirm') as mock_confirm, \
         patch('ui.prompts.prompt_text') as mock_text, \
         patch('ui.prompts.prompt_path') as mock_path, \
         patch('ui.prompts.prompt_choice') as mock_choice, \
         patch('ui.prompts.press_enter_to_continue') as mock_enter, \
         patch('ui.prompts.print_banner') as mock_banner:

        mock_confirm.return_value = True
        mock_text.return_value = "Test User"
        mock_path.return_value = "/tmp/test"
        mock_choice.return_value = 1
        mock_enter.return_value = None
        mock_banner.return_value = None

        yield {
            'confirm': mock_confirm,
            'text': mock_text,
            'path': mock_path,
            'choice': mock_choice,
            'enter': mock_enter,
            'banner': mock_banner
        }


@pytest.fixture
def mock_subprocess():
    """Mock subprocess for git commands."""
    with patch('subprocess.run') as mock_run:
        mock_run.return_value = Mock(returncode=0, stdout=b'', stderr=b'')
        yield mock_run


@pytest.fixture
def mock_webbrowser():
    """Mock webbrowser.open."""
    with patch('webbrowser.open') as mock_open:
        mock_open.return_value = True
        yield mock_open


class MockFileOperations:
    """Mock FileOperations class for testing."""

    def __init__(self):
        self.operations = []
        self.written_files = {}
        self.created_dirs = set()

    def write_file(self, path, content):
        """Mock write file."""
        path = Path(path)
        self.operations.append(('write', str(path), content))
        self.written_files[str(path)] = content
        # Actually create the file for tests that need it
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def create_directory(self, path):
        """Mock create directory."""
        path = Path(path)
        self.operations.append(('mkdir', str(path)))
        self.created_dirs.add(str(path))
        path.mkdir(parents=True, exist_ok=True)

    def rollback(self):
        """Mock rollback."""
        count = len(self.operations)
        self.operations = []
        return count


@pytest.fixture
def mock_file_ops():
    """Create mock file operations."""
    return MockFileOperations()
