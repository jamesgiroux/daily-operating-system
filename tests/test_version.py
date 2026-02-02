"""
Tests for DailyOS version management system.

Test scenarios:
1. Version detection and comparison
2. Core initialization
3. Symlink creation and verification
4. Update checking (with daily rate limiting)
5. Eject/reset functionality
6. Doctor and repair commands
"""

import pytest
import sys
import os
import json
from pathlib import Path
from datetime import datetime, timedelta
from unittest.mock import Mock, patch, MagicMock

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))


@pytest.fixture
def temp_core(tmp_path):
    """Create a temporary core directory (~/.dailyos equivalent)."""
    core = tmp_path / "core"
    core.mkdir()

    # Create VERSION file
    (core / "VERSION").write_text("0.4.0\n")

    # Create CHANGELOG.md
    (core / "CHANGELOG.md").write_text("""# Changelog

## [0.4.0] - 2026-02-02
### Added
- Version management system
- Symlink-based installation

## [0.3.0] - 2026-01-20
### Added
- Initial release
""")

    # Create commands directory
    commands = core / "commands"
    commands.mkdir()
    (commands / "today.md").write_text("# /today\n\nMock today command.\n")
    (commands / "wrap.md").write_text("# /wrap\n\nMock wrap command.\n")
    (commands / "week.md").write_text("# /week\n\nMock week command.\n")

    # Create skills directory
    skills = core / "skills"
    skills.mkdir()
    inbox = skills / "inbox"
    inbox.mkdir()
    (inbox / "SKILL.md").write_text("# Inbox Processing\n\nMock skill.\n")

    # Create agents directory
    agents = core / "agents"
    agents.mkdir()
    (agents / "file-organizer.md").write_text("# File Organizer\n\nMock agent.\n")

    # Create src directory with version module
    src = core / "src"
    src.mkdir()

    return core


@pytest.fixture
def temp_workspace(tmp_path):
    """Create a temporary workspace directory."""
    workspace = tmp_path / "workspace"
    workspace.mkdir()

    # Create basic structure
    (workspace / ".claude" / "commands").mkdir(parents=True)
    (workspace / ".claude" / "skills").mkdir(parents=True)
    (workspace / ".claude" / "agents").mkdir(parents=True)

    return workspace


@pytest.fixture
def temp_repo(tmp_path):
    """Create a temporary repository directory (source)."""
    repo = tmp_path / "repo"
    repo.mkdir()

    # Create VERSION file
    (repo / "VERSION").write_text("0.4.0\n")

    # Create commands directory
    commands = repo / "commands"
    commands.mkdir()
    (commands / "today.md").write_text("# /today\n\nRepository today command.\n")

    # Create skills directory
    skills = repo / "skills"
    skills.mkdir()
    inbox = skills / "inbox"
    inbox.mkdir()
    (inbox / "SKILL.md").write_text("# Inbox Processing\n\nRepository skill.\n")

    return repo


class TestVersionDetection:
    """Test version reading and comparison."""

    def test_get_core_version(self, temp_core):
        """Should read version from core VERSION file."""
        from version import get_core_version

        with patch('version.CORE_PATH', temp_core):
            version = get_core_version()
            assert version == "0.4.0"

    def test_get_core_version_missing(self, tmp_path):
        """Should return 0.0.0 if VERSION file missing."""
        from version import get_core_version

        empty_core = tmp_path / "empty_core"
        empty_core.mkdir()

        with patch('version.CORE_PATH', empty_core):
            version = get_core_version()
            assert version == "0.0.0"

    def test_get_workspace_version(self, temp_workspace):
        """Should read version from workspace .dailyos-version file."""
        from version import get_workspace_version

        # Create version file
        (temp_workspace / ".dailyos-version").write_text("0.3.0\n")

        version = get_workspace_version(temp_workspace)
        assert version == "0.3.0"

    def test_get_workspace_version_missing(self, temp_workspace):
        """Should return 0.0.0 if .dailyos-version missing."""
        from version import get_workspace_version

        version = get_workspace_version(temp_workspace)
        assert version == "0.0.0"

    def test_set_workspace_version(self, temp_workspace):
        """Should write version to workspace .dailyos-version file."""
        from version import set_workspace_version, get_workspace_version

        set_workspace_version(temp_workspace, "0.4.0")

        version = get_workspace_version(temp_workspace)
        assert version == "0.4.0"

    def test_compare_versions(self):
        """Should correctly compare version strings."""
        from version import compare_versions

        assert compare_versions("0.4.0", "0.3.0") > 0  # 0.4.0 > 0.3.0
        assert compare_versions("0.3.0", "0.4.0") < 0  # 0.3.0 < 0.4.0
        assert compare_versions("0.4.0", "0.4.0") == 0  # Equal
        assert compare_versions("1.0.0", "0.99.99") > 0  # Major version matters
        assert compare_versions("0.10.0", "0.9.0") > 0  # 10 > 9


class TestUpdateChecking:
    """Test update detection and rate limiting."""

    def test_check_for_updates_available(self, temp_core, temp_workspace):
        """Should detect when update is available."""
        from version import check_for_updates

        # Set workspace to older version
        (temp_workspace / ".dailyos-version").write_text("0.3.0\n")

        with patch('version.CORE_PATH', temp_core):
            update_info = check_for_updates(temp_workspace)

            assert update_info is not None
            assert update_info['current'] == "0.3.0"
            assert update_info['available'] == "0.4.0"

    def test_check_for_updates_not_available(self, temp_core, temp_workspace):
        """Should return None when up to date."""
        from version import check_for_updates

        # Set workspace to same version as core
        (temp_workspace / ".dailyos-version").write_text("0.4.0\n")

        with patch('version.CORE_PATH', temp_core):
            update_info = check_for_updates(temp_workspace)
            assert update_info is None

    def test_should_check_today_first_time(self, temp_workspace):
        """Should check for updates if never checked before."""
        from version import should_check_today

        # No .dailyos-last-check file exists
        result = should_check_today(temp_workspace)
        assert result is True

    def test_should_check_today_already_checked(self, temp_workspace):
        """Should not check if already checked today."""
        from version import should_check_today, record_check

        # Record a check for today
        record_check(temp_workspace)

        result = should_check_today(temp_workspace)
        assert result is False

    def test_should_check_today_checked_yesterday(self, temp_workspace):
        """Should check if last check was yesterday."""
        from version import should_check_today

        # Create check file with yesterday's date
        yesterday = datetime.now() - timedelta(days=1)
        check_file = temp_workspace / ".dailyos-last-check"
        check_file.write_text(yesterday.isoformat())

        result = should_check_today(temp_workspace)
        assert result is True

    def test_record_check(self, temp_workspace):
        """Should record check timestamp."""
        from version import record_check, should_check_today

        # Before recording, should check
        assert should_check_today(temp_workspace) is True

        # Record check
        record_check(temp_workspace)

        # After recording, should not check
        assert should_check_today(temp_workspace) is False


class TestCoreInitialization:
    """Test initialization of ~/.dailyos core directory."""

    def test_initialize_core_fresh(self, temp_repo, tmp_path):
        """Should create core from repository."""
        from version import initialize_core

        core_path = tmp_path / "core"

        with patch('version.CORE_PATH', core_path):
            success, message = initialize_core(temp_repo)

            assert success is True
            assert core_path.exists()
            assert (core_path / "VERSION").exists()
            assert (core_path / "VERSION").read_text().strip() == "0.4.0"

    def test_initialize_core_already_exists(self, temp_repo, temp_core):
        """Should skip if core already exists with same version."""
        from version import initialize_core

        with patch('version.CORE_PATH', temp_core):
            success, message = initialize_core(temp_repo)

            # Should succeed but indicate already exists
            assert success is True
            assert "already" in message.lower() or "up to date" in message.lower()


class TestSymlinkOperations:
    """Test symlink creation and verification."""

    def test_create_command_symlink(self, temp_core, temp_workspace):
        """Should create symlink for command."""
        # Import after setting up paths
        with patch('version.CORE_PATH', temp_core):
            # Need to reload to pick up patched CORE_PATH
            import importlib
            import steps.skills as skills_module
            skills_module.CORE_PATH = temp_core
            skills_module.VERSION_AVAILABLE = True

            from steps.skills import install_command_symlink

            result = install_command_symlink(temp_workspace, "today")

            assert result is True
            cmd_path = temp_workspace / ".claude" / "commands" / "today.md"
            assert cmd_path.is_symlink()
            assert cmd_path.resolve() == temp_core / "commands" / "today.md"

    def test_create_skill_symlink(self, temp_core, temp_workspace):
        """Should create symlink for skill directory."""
        with patch('version.CORE_PATH', temp_core):
            import steps.skills as skills_module
            skills_module.CORE_PATH = temp_core
            skills_module.VERSION_AVAILABLE = True

            from steps.skills import install_skill_symlink

            # Add inbox-processing to available skills temporarily
            skills_module.AVAILABLE_SKILLS['inbox-processing'] = {
                'name': 'Inbox Processing',
                'description': 'Test',
                'category': 'core',
                'agents': [],
            }
            skills_module.SKILL_TEMPLATE_MAP['inbox-processing'] = 'inbox'

            result = install_skill_symlink(temp_workspace, "inbox-processing")

            assert result is True
            skill_path = temp_workspace / ".claude" / "skills" / "inbox-processing"
            assert skill_path.is_symlink()

    def test_is_symlink_intact(self, temp_core, temp_workspace):
        """Should verify symlink points to correct location."""
        from version import is_symlink_intact

        # Create a valid symlink
        cmd_path = temp_workspace / ".claude" / "commands" / "today.md"
        cmd_path.parent.mkdir(parents=True, exist_ok=True)
        cmd_path.symlink_to(temp_core / "commands" / "today.md")

        with patch('version.CORE_PATH', temp_core):
            assert is_symlink_intact(temp_workspace, 'today.md', 'commands') is True

    def test_is_symlink_intact_broken(self, temp_workspace, tmp_path):
        """Should detect broken symlink."""
        from version import is_symlink_intact

        # Create a broken symlink
        cmd_path = temp_workspace / ".claude" / "commands" / "today.md"
        cmd_path.parent.mkdir(parents=True, exist_ok=True)
        nonexistent = tmp_path / "nonexistent" / "today.md"
        cmd_path.symlink_to(nonexistent)

        with patch('version.CORE_PATH', tmp_path / "core"):
            assert is_symlink_intact(temp_workspace, 'today.md', 'commands') is False


class TestEjectReset:
    """Test eject (convert symlink to copy) and reset (restore symlink)."""

    def test_eject_command(self, temp_core, temp_workspace):
        """Should convert symlink to regular file."""
        import version as version_module

        # First create a symlink
        cmd_path = temp_workspace / ".claude" / "commands" / "today.md"
        cmd_path.parent.mkdir(parents=True, exist_ok=True)
        cmd_path.symlink_to(temp_core / "commands" / "today.md")

        assert cmd_path.is_symlink()

        # Patch CORE_PATH on the module directly
        original_core = version_module.CORE_PATH
        version_module.CORE_PATH = temp_core

        try:
            success = version_module.eject_component(temp_workspace, "today.md", "commands")

            assert success is True
            assert cmd_path.exists()
            assert not cmd_path.is_symlink()  # No longer a symlink
            assert "Mock today command" in cmd_path.read_text()
        finally:
            version_module.CORE_PATH = original_core

    def test_reset_command(self, temp_core, temp_workspace):
        """Should restore symlink from regular file."""
        import version as version_module

        # Create a regular file
        cmd_path = temp_workspace / ".claude" / "commands" / "today.md"
        cmd_path.parent.mkdir(parents=True, exist_ok=True)
        cmd_path.write_text("# Custom content\n")

        assert not cmd_path.is_symlink()

        # Patch CORE_PATH on the module directly
        original_core = version_module.CORE_PATH
        version_module.CORE_PATH = temp_core

        try:
            success = version_module.reset_component(temp_workspace, "today.md", "commands")

            assert success is True
            assert cmd_path.is_symlink()
            assert cmd_path.resolve() == temp_core / "commands" / "today.md"
        finally:
            version_module.CORE_PATH = original_core


class TestEjectedTracking:
    """Test tracking of ejected components."""

    def test_get_ejected_empty(self, temp_workspace):
        """Should return empty list if no ejected file."""
        from version import get_ejected_skills

        ejected = get_ejected_skills(temp_workspace)
        assert ejected == []

    def test_get_ejected_with_items(self, temp_workspace):
        """Should return list of ejected items."""
        from version import get_ejected_skills

        # Create ejected file
        ejected_file = temp_workspace / ".dailyos-ejected"
        ejected_file.write_text(json.dumps(["today", "week"]))

        ejected = get_ejected_skills(temp_workspace)
        assert "today" in ejected
        assert "week" in ejected

    def test_add_to_ejected(self, temp_workspace):
        """Should add item to ejected list."""
        from version import add_to_ejected, get_ejected_skills

        add_to_ejected(temp_workspace, "today")
        add_to_ejected(temp_workspace, "week")

        ejected = get_ejected_skills(temp_workspace)
        assert "today" in ejected
        assert "week" in ejected

    def test_remove_from_ejected(self, temp_workspace):
        """Should remove item from ejected list."""
        from version import add_to_ejected, remove_from_ejected, get_ejected_skills

        add_to_ejected(temp_workspace, "today")
        add_to_ejected(temp_workspace, "week")

        remove_from_ejected(temp_workspace, "today")

        ejected = get_ejected_skills(temp_workspace)
        assert "today" not in ejected
        assert "week" in ejected


class TestGitOperations:
    """Test git pull operations for core updates."""

    def test_git_pull_success(self, temp_core):
        """Should successfully pull updates."""
        from version import git_pull_core

        # Initialize git repo in core
        import subprocess
        subprocess.run(['git', 'init'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'config', 'user.email', 'test@test.com'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'config', 'user.name', 'Test'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'add', '.'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'commit', '-m', 'Initial'], cwd=temp_core, capture_output=True)

        with patch('version.CORE_PATH', temp_core):
            success, message = git_pull_core()

            # Should succeed (even if "already up to date" since no remote)
            assert success is True

    def test_git_pull_no_remote(self, temp_core):
        """Should handle missing remote gracefully."""
        from version import git_pull_core

        # Initialize git repo without remote
        import subprocess
        subprocess.run(['git', 'init'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'config', 'user.email', 'test@test.com'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'config', 'user.name', 'Test'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'add', '.'], cwd=temp_core, capture_output=True)
        subprocess.run(['git', 'commit', '-m', 'Initial'], cwd=temp_core, capture_output=True)

        with patch('version.CORE_PATH', temp_core):
            success, message = git_pull_core()

            # Should succeed for local-only installations
            assert success is True


class TestIntegration:
    """Integration tests for full workflows."""

    def test_full_update_workflow(self, temp_core, temp_workspace):
        """Test complete update check and apply workflow."""
        from version import (
            check_for_updates, should_check_today, record_check,
            set_workspace_version, get_workspace_version
        )

        # Set workspace to older version
        set_workspace_version(temp_workspace, "0.3.0")

        with patch('version.CORE_PATH', temp_core):
            # Should check (first time)
            assert should_check_today(temp_workspace) is True

            # Should find update
            update_info = check_for_updates(temp_workspace)
            assert update_info is not None
            assert update_info['available'] == "0.4.0"

            # Simulate applying update
            set_workspace_version(temp_workspace, "0.4.0")
            record_check(temp_workspace)

            # Should not check again today
            assert should_check_today(temp_workspace) is False

            # Should not find update (now current)
            update_info = check_for_updates(temp_workspace)
            assert update_info is None

    def test_symlink_to_eject_to_reset(self, temp_core, temp_workspace):
        """Test full lifecycle: install symlink, eject, then reset."""
        import version as version_module

        # Create initial symlink
        cmd_path = temp_workspace / ".claude" / "commands" / "today.md"
        cmd_path.parent.mkdir(parents=True, exist_ok=True)
        cmd_path.symlink_to(temp_core / "commands" / "today.md")

        # Patch CORE_PATH on the module directly
        original_core = version_module.CORE_PATH
        version_module.CORE_PATH = temp_core

        try:
            # Verify symlink
            assert cmd_path.is_symlink()

            # Eject
            version_module.eject_component(temp_workspace, "today.md", "commands")
            assert not cmd_path.is_symlink()
            assert "today.md" in version_module.get_ejected_skills(temp_workspace)

            # Modify the ejected file
            cmd_path.write_text("# My custom today command\n")

            # Reset
            version_module.reset_component(temp_workspace, "today.md", "commands")
            assert cmd_path.is_symlink()
            assert "today.md" not in version_module.get_ejected_skills(temp_workspace)
        finally:
            version_module.CORE_PATH = original_core
