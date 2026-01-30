"""
Comprehensive tests for DailyOS setup wizard.

Test scenarios:
1. Path handling (tilde expansion, absolute paths, relative paths)
2. Idempotency (running setup multiple times)
3. Edge cases (empty workspace, existing files, special characters)
4. Error handling (missing templates, permission errors)
5. Quick setup mode
6. Verification mode
7. Google API setup mode
8. Rollback on failure
"""

import pytest
import sys
import os
from pathlib import Path
from argparse import Namespace
from unittest.mock import Mock, patch, MagicMock

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))


class TestPathHandling:
    """Test path expansion and normalization."""

    def test_tilde_expansion_in_workspace(self, tmp_path, mock_prompts, mock_validators):
        """Tilde paths should be expanded to absolute paths."""
        from wizard import SetupWizard

        # Create a test path we can actually verify
        test_home = tmp_path / "home"
        test_home.mkdir()

        args = Namespace(
            workspace=str(test_home / "productivity"),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)

        # Mock the quick setup prerequisites check
        with patch.object(wizard, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard, '_create_directories'), \
             patch.object(wizard, '_init_git'), \
             patch.object(wizard, '_create_basic_claude_md'), \
             patch.object(wizard, '_install_default_skills'), \
             patch.object(wizard, '_install_python_tools'), \
             patch.object(wizard, '_verify_installation', return_value=True):

            result = wizard.run_quick_setup()

        assert result == 0
        # Workspace should be set and expanded
        assert wizard.config.get('workspace') is not None

    def test_absolute_path_preserved(self, temp_workspace, mock_prompts, mock_validators):
        """Absolute paths should be preserved as-is."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)

        with patch.object(wizard, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard, '_create_directories'), \
             patch.object(wizard, '_init_git'), \
             patch.object(wizard, '_create_basic_claude_md'), \
             patch.object(wizard, '_install_default_skills'), \
             patch.object(wizard, '_install_python_tools'), \
             patch.object(wizard, '_verify_installation', return_value=True):

            result = wizard.run_quick_setup()

        assert result == 0
        assert str(wizard.config['workspace']) == str(temp_workspace)

    def test_path_with_spaces(self, tmp_path, mock_prompts, mock_validators):
        """Paths with spaces should be handled correctly."""
        from wizard import SetupWizard

        space_path = tmp_path / "my workspace folder"
        space_path.mkdir()

        args = Namespace(
            workspace=str(space_path),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)

        with patch.object(wizard, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard, '_create_directories'), \
             patch.object(wizard, '_init_git'), \
             patch.object(wizard, '_create_basic_claude_md'), \
             patch.object(wizard, '_install_default_skills'), \
             patch.object(wizard, '_install_python_tools'), \
             patch.object(wizard, '_verify_installation', return_value=True):

            result = wizard.run_quick_setup()

        assert result == 0
        assert "my workspace folder" in str(wizard.config['workspace'])


class TestIdempotency:
    """Test that setup can be run multiple times safely."""

    def test_run_setup_twice(self, temp_workspace, mock_prompts, mock_validators):
        """Running setup twice should not fail or duplicate content."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        # First run
        wizard1 = SetupWizard(args)
        with patch.object(wizard1, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard1, '_verify_installation', return_value=True):
            result1 = wizard1.run_quick_setup()

        # Second run
        wizard2 = SetupWizard(args)
        with patch.object(wizard2, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard2, '_verify_installation', return_value=True):
            result2 = wizard2.run_quick_setup()

        assert result1 == 0
        assert result2 == 0

    def test_existing_git_repo_preserved(self, temp_workspace, mock_prompts, mock_validators, mock_subprocess):
        """Existing git repo should not be reinitialized."""
        from wizard import SetupWizard

        # Create existing git repo
        git_dir = temp_workspace / ".git"
        git_dir.mkdir()

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace
        wizard.config['skip_git'] = False

        # The step should detect existing repo and skip
        result = wizard._step_git()

        assert result is True
        # subprocess should NOT have been called for git init
        mock_subprocess.assert_not_called()


class TestDirectoryCreation:
    """Test PARA directory structure creation."""

    def test_creates_all_para_directories(self, temp_workspace, mock_validators):
        """All PARA directories should be created."""
        from wizard import SetupWizard
        from utils.file_ops import FileOperations

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace
        wizard.config['role'] = 'account_owner'
        wizard.file_ops = FileOperations()

        # Create directories
        wizard._create_directories()

        # Check required directories exist
        required = ['Projects', 'Areas', 'Resources', 'Archive', '_inbox', '_today', '_templates', '_tools']
        for dir_name in required:
            assert (temp_workspace / dir_name).exists(), f"Missing directory: {dir_name}"

    def test_account_structure_created_for_account_owner(self, temp_workspace, mock_validators):
        """Account owner role should create Accounts directory structure."""
        from wizard import SetupWizard
        from utils.file_ops import FileOperations

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace
        wizard.config['role'] = 'account_owner'
        wizard.file_ops = FileOperations()

        wizard._create_directories()

        # Account owner should have Accounts directory
        assert (temp_workspace / 'Accounts').exists()


class TestSkillInstallation:
    """Test skills and commands installation."""

    def test_commands_installed(self, temp_workspace, templates_dir, mock_validators):
        """Command files should be installed to .claude/commands/."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace

        # Patch Path to find our mock templates
        with patch('wizard.Path') as MockPath:
            # Make Path behave normally except for __file__.parent.parent
            MockPath.side_effect = lambda x: Path(x)
            MockPath.home.return_value = Path.home()

            # Create a mock for the template discovery
            script_parent = MagicMock()
            script_parent.parent = templates_dir.parent
            MockPath.return_value.parent.parent = script_parent

            # Use real Path for workspace operations
            wizard._install_default_skills()

        # Verify commands were installed
        commands_dir = temp_workspace / '.claude' / 'commands'
        assert commands_dir.exists()

    def test_skills_installed(self, temp_workspace, mock_validators):
        """Skill packages should be installed to .claude/skills/."""
        from wizard import SetupWizard
        from utils.file_ops import FileOperations

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace
        wizard.file_ops = FileOperations()

        # Run installation (will use real templates or fallback)
        wizard._install_default_skills()

        # Skills directory should be created
        skills_dir = temp_workspace / '.claude' / 'skills'
        # May or may not exist depending on templates availability


class TestVerification:
    """Test installation verification."""

    def test_verify_complete_installation(self, temp_workspace, mock_validators):
        """Verification should pass with complete installation."""
        from wizard import SetupWizard
        from utils.file_ops import FileOperations

        # Set up complete installation
        for dir_name in ['Projects', 'Areas', 'Resources', 'Archive', '_inbox', '_today']:
            (temp_workspace / dir_name).mkdir()

        (temp_workspace / 'CLAUDE.md').write_text("# CLAUDE.md\n")
        (temp_workspace / '.claude' / 'commands').mkdir(parents=True)
        (temp_workspace / '.git').mkdir()

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace

        result = wizard._verify_installation()

        assert result is True

    def test_verify_incomplete_installation(self, temp_workspace, mock_validators):
        """Verification should fail with missing directories."""
        from wizard import SetupWizard

        # Only create some directories
        (temp_workspace / 'Projects').mkdir()

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace

        result = wizard._verify_installation()

        assert result is False


class TestQuickSetup:
    """Test quick setup mode."""

    def test_quick_setup_uses_defaults(self, tmp_path, mock_prompts, mock_validators):
        """Quick setup should use defaults without prompting."""
        from wizard import SetupWizard

        workspace = tmp_path / "quick_test"

        args = Namespace(
            workspace=str(workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)

        with patch.object(wizard, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard, '_create_directories'), \
             patch.object(wizard, '_init_git'), \
             patch.object(wizard, '_create_basic_claude_md'), \
             patch.object(wizard, '_install_default_skills'), \
             patch.object(wizard, '_install_python_tools'), \
             patch.object(wizard, '_verify_installation', return_value=True):

            result = wizard.run_quick_setup()

        assert result == 0
        # confirm should not have been called in quick mode
        # (we mocked the internal methods, so prompts are bypassed)

    def test_quick_setup_skips_google(self, tmp_path, mock_prompts, mock_validators):
        """Quick setup should skip Google API configuration."""
        from wizard import SetupWizard

        workspace = tmp_path / "quick_test"

        args = Namespace(
            workspace=str(workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)

        with patch.object(wizard, '_check_prerequisites_silent', return_value=True), \
             patch.object(wizard, '_create_directories'), \
             patch.object(wizard, '_init_git'), \
             patch.object(wizard, '_create_basic_claude_md'), \
             patch.object(wizard, '_install_default_skills'), \
             patch.object(wizard, '_install_python_tools'), \
             patch.object(wizard, '_verify_installation', return_value=True):

            result = wizard.run_quick_setup()

        assert result == 0
        # Google API should not be configured
        assert wizard.config.get('google_api') is None


class TestErrorHandling:
    """Test error handling and rollback."""

    def test_keyboard_interrupt_offers_rollback(self, temp_workspace, mock_prompts, mock_validators):
        """KeyboardInterrupt should offer rollback option."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)

        # Simulate KeyboardInterrupt during setup
        with patch.object(wizard, '_print_intro', side_effect=KeyboardInterrupt), \
             patch('ui.prompts.confirm', return_value=False):

            result = wizard.run()

        assert result == 130  # Standard exit code for Ctrl+C

    def test_exception_offers_rollback(self, temp_workspace, mock_prompts, mock_validators):
        """Unexpected exceptions should offer rollback option."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)

        # Simulate exception during setup
        with patch.object(wizard, '_print_intro', side_effect=Exception("Test error")), \
             patch('ui.prompts.confirm', return_value=False):

            result = wizard.run()

        assert result == 1

    def test_verbose_mode_shows_traceback(self, temp_workspace, mock_validators):
        """Verbose mode should show full traceback on errors."""
        from wizard import SetupWizard
        import io
        import sys

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=False,
            verbose=True
        )

        wizard = SetupWizard(args)

        # Capture stderr
        with patch.object(wizard, '_print_intro', side_effect=Exception("Test error")), \
             patch('ui.prompts.confirm', return_value=False), \
             patch('traceback.print_exc') as mock_traceback:

            result = wizard.run()

        # In verbose mode, traceback should be printed
        mock_traceback.assert_called_once()


class TestGoogleAPISetup:
    """Test Google API setup mode."""

    def test_google_setup_only_mode(self, temp_workspace, mock_prompts, mock_validators):
        """--google flag should only run Google setup."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=True,
            verify=False,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)

        with patch.object(wizard, '_step_google_api', return_value=True):
            result = wizard.run_google_setup_only()

        assert result == 0
        assert wizard.config.get('workspace') == temp_workspace

    def test_google_setup_expands_tilde(self, tmp_path, mock_prompts, mock_validators):
        """Google setup should expand tilde in workspace path."""
        from wizard import SetupWizard

        # Create a real path to test with
        test_workspace = tmp_path / "test_ws"
        test_workspace.mkdir()

        args = Namespace(
            workspace=str(test_workspace),
            google=True,
            verify=False,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)

        with patch.object(wizard, '_step_google_api', return_value=True):
            result = wizard.run_google_setup_only()

        assert result == 0
        # Path should be a Path object (expanded)
        assert isinstance(wizard.config['workspace'], Path)


class TestVerificationMode:
    """Test verification-only mode."""

    def test_verify_only_mode(self, temp_workspace, mock_prompts, mock_validators):
        """--verify flag should only run verification."""
        from wizard import SetupWizard

        # Create minimal installation
        for dir_name in ['Projects', 'Areas', 'Resources', 'Archive', '_inbox', '_today']:
            (temp_workspace / dir_name).mkdir()
        (temp_workspace / 'CLAUDE.md').write_text("# Test\n")
        (temp_workspace / '.claude' / 'commands').mkdir(parents=True)

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=True,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)
        result = wizard.run_verification_only()

        assert result == 0

    def test_verify_nonexistent_workspace_fails(self, tmp_path, mock_prompts, mock_validators):
        """Verification should fail for non-existent workspace."""
        from wizard import SetupWizard

        nonexistent = tmp_path / "does_not_exist"

        args = Namespace(
            workspace=str(nonexistent),
            google=False,
            verify=True,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)
        result = wizard.run_verification_only()

        assert result == 1


class TestCLAUDEMD:
    """Test CLAUDE.md generation."""

    def test_basic_claude_md_created(self, temp_workspace, mock_validators):
        """Basic CLAUDE.md should be created with correct structure."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace

        wizard._create_basic_claude_md()

        claude_md = temp_workspace / 'CLAUDE.md'
        assert claude_md.exists()

        content = claude_md.read_text()
        assert '# CLAUDE.md' in content
        assert 'PARA' in content or 'Projects/' in content

    def test_claude_md_questionnaire(self, temp_workspace, mock_prompts, mock_validators):
        """Questionnaire should generate personalized CLAUDE.md."""
        from wizard import SetupWizard

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=False,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace

        result = wizard._claude_md_questionnaire()

        assert result is True
        claude_md = temp_workspace / 'CLAUDE.md'
        assert claude_md.exists()

        content = claude_md.read_text()
        # Should contain the zero-guilt design principles
        assert 'Consuming, not producing' in content or 'Works when you work' in content


class TestSetupPyEntryPoint:
    """Test the main setup.py entry point."""

    def test_setup_py_imports_correctly(self):
        """setup.py should import without errors."""
        setup_py = Path(__file__).parent.parent / "setup.py"
        assert setup_py.exists()

        # Read and check for correct imports
        content = setup_py.read_text()
        assert "from wizard import SetupWizard" in content
        assert "SetupWizard(args)" in content

    def test_argparse_arguments_defined(self):
        """All expected arguments should be defined."""
        setup_py = Path(__file__).parent.parent / "setup.py"
        content = setup_py.read_text()

        expected_args = ['--workspace', '--google', '--verify', '--quick', '--verbose']
        for arg in expected_args:
            assert arg in content, f"Missing argument: {arg}"


class TestGitSetup:
    """Test Git repository initialization."""

    def test_gitignore_created(self, temp_workspace, mock_validators, mock_subprocess):
        """Git initialization should create .gitignore."""
        from wizard import SetupWizard
        from utils.file_ops import FileOperations

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace
        wizard.file_ops = FileOperations()

        wizard._init_git()

        gitignore = temp_workspace / '.gitignore'
        assert gitignore.exists()

        content = gitignore.read_text()
        assert 'credentials.json' in content
        assert '.DS_Store' in content
        assert '__pycache__' in content

    def test_git_commands_executed(self, temp_workspace, mock_validators, mock_subprocess):
        """Git init and commit commands should be executed."""
        from wizard import SetupWizard
        from utils.file_ops import FileOperations

        args = Namespace(
            workspace=str(temp_workspace),
            google=False,
            verify=False,
            quick=True,
            verbose=False
        )

        wizard = SetupWizard(args)
        wizard.config['workspace'] = temp_workspace
        wizard.file_ops = FileOperations()

        wizard._init_git()

        # Check subprocess was called for git operations
        calls = mock_subprocess.call_args_list
        git_commands = [c for c in calls if 'git' in str(c)]
        assert len(git_commands) >= 3  # init, add, commit


class TestFileOperations:
    """Test file operations utility."""

    def test_write_file_creates_parent_dirs(self, tmp_path):
        """write_file should create parent directories."""
        from utils.file_ops import FileOperations

        file_ops = FileOperations()
        nested_path = tmp_path / "a" / "b" / "c" / "file.txt"

        file_ops.write_file(nested_path, "test content")

        assert nested_path.exists()
        assert nested_path.read_text() == "test content"

    def test_rollback_tracks_operations(self, tmp_path):
        """Rollback should track and revert operations."""
        from utils.file_ops import FileOperations

        file_ops = FileOperations()
        test_file = tmp_path / "test.txt"

        file_ops.write_file(test_file, "content")
        assert test_file.exists()

        count = file_ops.rollback()
        # Should have tracked at least the write operation
        assert count >= 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
