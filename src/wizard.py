"""
Daily Operating System Setup Wizard - Main Orchestrator.

Guides users through setting up the Daily Operating System
productivity framework on Claude Code.
"""

import sys
import webbrowser
from pathlib import Path
from typing import Optional, Dict, Any

from ui.colors import Colors, success, error, warning, info, header
from ui.prompts import (
    print_banner, print_step_header, print_section,
    confirm, prompt_text, prompt_path, prompt_choice,
    print_success, print_warning, print_error, print_info,
    press_enter_to_continue
)
from ui.progress import ProgressBar, Spinner, print_checklist
from utils.file_ops import FileOperations, FileOperationError
from utils.validators import (
    validate_path, validate_directory_writable,
    validate_command_exists, validate_python_version
)


class SetupWizard:
    """
    Main setup wizard orchestrator.

    Philosophy: Guide users step-by-step through setup with clear
    explanations, safe defaults, and the ability to rollback on errors.
    """

    TOTAL_STEPS = 10

    def __init__(self, args):
        """
        Initialize the wizard with command-line arguments.

        Args:
            args: Parsed argparse namespace with options like
                  workspace, google, verify, quick, verbose
        """
        self.args = args
        self.verbose = getattr(args, 'verbose', False)
        self.config: Dict[str, Any] = {}
        self.file_ops = FileOperations()
        self._current_step_name: Optional[str] = None

    def run(self) -> int:
        """
        Run the full setup wizard.

        Returns:
            Exit code (0 for success, non-zero for failure)
        """
        try:
            print_banner()
            self._print_intro()

            if not confirm("Ready to begin setup?"):
                print("\nSetup cancelled. Run again when ready.")
                return 0

            # Step 1: Prerequisites
            self._current_step_name = "Prerequisites Check"
            if not self._step_prerequisites():
                return 1

            # Step 2: Workspace Location
            self._current_step_name = "Workspace Location"
            if not self._step_workspace():
                return 1

            # Step 3: Directory Structure
            self._current_step_name = "Directory Structure"
            if not self._step_directories():
                return 1

            # Step 4: Git Setup
            self._current_step_name = "Git Setup"
            if not self._step_git():
                return 1

            # Step 5: Google API (optional)
            self._current_step_name = "Google API Integration"
            if not self._step_google_api():
                return 1

            # Step 6: CLAUDE.md Configuration
            self._current_step_name = "CLAUDE.md Configuration"
            if not self._step_claude_md():
                return 1

            # Step 7: Skills & Commands
            self._current_step_name = "Skills & Commands"
            if not self._step_skills():
                return 1

            # Step 8: Web Dashboard (UI)
            self._current_step_name = "Web Dashboard"
            if not self._step_ui():
                return 1

            # Step 9: Python Tools
            self._current_step_name = "Python Tools"
            if not self._step_python_tools():
                return 1

            # Step 10: Verification
            self._current_step_name = "Verification"
            if not self._step_verification():
                return 1

            self._print_completion()
            return 0

        except KeyboardInterrupt:
            print("\n\nSetup interrupted by user.")
            if confirm("\nRollback changes made so far?"):
                count = self.file_ops.rollback()
                print(f"Rolled back {count} operations.")
            return 130

        except Exception as e:
            self._handle_error(e)
            return 1

    def _handle_error(self, e: Exception):
        """
        Handle errors in a beginner-friendly way.

        Shows a clear message, saves an error report, and offers simple next steps.
        Technical details are hidden unless requested.
        """
        import traceback
        import io
        from datetime import datetime

        step_name = self._current_step_name or "setup"

        # Friendly message first
        print()
        print_error(f"Something went wrong during {step_name.lower()}")
        print()
        print("  Don't worry - this is usually fixable!")
        print()

        # Generate error report content
        error_report = io.StringIO()
        error_report.write("=" * 60 + "\n")
        error_report.write("Daily Operating System - Error Report\n")
        error_report.write("=" * 60 + "\n\n")
        error_report.write(f"Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        error_report.write(f"Step: {step_name}\n")
        error_report.write(f"Error: {type(e).__name__}: {e}\n\n")
        error_report.write("Traceback:\n")
        error_report.write("-" * 40 + "\n")
        traceback.print_exc(file=error_report)
        error_report.write("-" * 40 + "\n\n")
        error_report.write("System Info:\n")
        error_report.write(f"  Python: {sys.version}\n")
        error_report.write(f"  Platform: {sys.platform}\n")
        if self.config.get('workspace'):
            error_report.write(f"  Workspace: {self.config['workspace']}\n")
        error_report_content = error_report.getvalue()

        # Save error report to file
        error_file = Path.home() / "dailyos-error-report.txt"
        try:
            error_file.write_text(error_report_content)
            print(f"  An error report was saved to: {error_file}")
            print()
        except Exception:
            pass  # Don't fail on failing to save error report

        # Simple next steps
        print("What to do next:")
        print()
        print("  1. Try running setup again - many errors are temporary")
        print()
        print("  2. If it keeps failing, open an issue:")
        print("     github.com/jamesgiroux/daily-operating-system/issues")
        print("     (attach the error report file)")
        print()

        # Offer technical details (hidden by default)
        if confirm("Show technical details?", default=False):
            print()
            print("-" * 40)
            print(f"Error type: {type(e).__name__}")
            print(f"Message: {e}")
            print()
            traceback.print_exc()
            print("-" * 40)
            print()

        # Handle rollback with friendly language
        print()
        if confirm("Undo the changes made so far?", default=True):
            count = self.file_ops.rollback()
            print_success(f"Cleaned up {count} items. You can safely try again.")
        else:
            print_info("Changes kept. You can continue from where you left off.")

    def run_google_setup_only(self) -> int:
        """
        Run only the Google API setup step.

        Returns:
            Exit code (0 for success, non-zero for failure)
        """
        print_banner()
        print("\n" + header("Google API Setup Only"))
        print("-" * 40)

        # Need workspace path
        workspace = getattr(self.args, 'workspace', None)
        if not workspace:
            workspace = prompt_path(
                "Enter your workspace path",
                default="~/Documents/productivity"
            )
        self.config['workspace'] = Path(workspace).expanduser()

        if not self.config['workspace'].exists():
            print_error(f"Workspace does not exist: {self.config['workspace']}")
            return 1

        return 0 if self._step_google_api() else 1

    def run_verification_only(self) -> int:
        """
        Run only the verification step to check an existing installation.

        Returns:
            Exit code (0 for success, non-zero for failure)
        """
        print_banner()
        print("\n" + header("Installation Verification"))
        print("-" * 40)

        # Need workspace path
        workspace = getattr(self.args, 'workspace', None)
        if not workspace:
            workspace = prompt_path(
                "Enter your workspace path to verify",
                default="~/Documents/productivity"
            )
        self.config['workspace'] = Path(workspace).expanduser()

        if not self.config['workspace'].exists():
            print_error(f"Workspace does not exist: {self.config['workspace']}")
            return 1

        return 0 if self._step_verification() else 1

    def run_quick_setup(self) -> int:
        """
        Run quick setup with sensible defaults.

        Skips interactive prompts where possible, uses defaults,
        but still checks prerequisites.

        Returns:
            Exit code (0 for success, non-zero for failure)
        """
        print_banner()
        print("\n" + info("Quick Setup Mode"))
        print("Using sensible defaults. Override with --workspace flag.\n")

        # Check prerequisites silently
        spinner = Spinner("Checking prerequisites...")
        prereqs_ok = self._check_prerequisites_silent()
        if prereqs_ok:
            spinner.succeed("Prerequisites OK")
        else:
            spinner.fail("Prerequisites check failed")
            return 1

        # Use provided workspace or default
        workspace = getattr(self.args, 'workspace', None)
        if not workspace:
            workspace = Path.home() / "Documents" / "productivity"
        self.config['workspace'] = Path(workspace).expanduser()

        print(f"\nWorkspace: {self.config['workspace']}")

        # Create directories
        spinner = Spinner("Creating directories...")
        try:
            self._create_directories()
            spinner.succeed("Directories created")
        except Exception as e:
            spinner.fail(f"Failed: {e}")
            return 1

        # Initialize git
        spinner = Spinner("Initializing Git...")
        try:
            self._init_git()
            spinner.succeed("Git initialized")
        except Exception as e:
            spinner.warn(f"Git setup skipped: {e}")

        # Skip Google API in quick mode
        print_info("Google API setup skipped (run with --google to configure)")

        # Create basic CLAUDE.md
        spinner = Spinner("Creating CLAUDE.md...")
        try:
            self._create_basic_claude_md()
            spinner.succeed("CLAUDE.md created")
        except Exception as e:
            spinner.fail(f"Failed: {e}")
            return 1

        # Install default skills
        spinner = Spinner("Installing skills and commands...")
        try:
            self._install_default_skills()
            spinner.succeed("Skills installed")
        except Exception as e:
            spinner.fail(f"Failed: {e}")
            return 1

        # Install Python tools
        spinner = Spinner("Installing Python tools...")
        try:
            self._install_python_tools()
            spinner.succeed("Python tools installed")
        except Exception as e:
            spinner.warn(f"Python tools skipped: {e}")

        # Verify
        print("\n" + header("Verification"))
        return 0 if self._verify_installation() else 1

    # =========================================================================
    # Step Implementations
    # =========================================================================

    def _open_companion_guide(self):
        """Open the HTML companion guide in the default browser."""
        # Find the docs file relative to this script
        script_dir = Path(__file__).parent.parent  # Go up from src/ to project root
        ui_path = script_dir / 'docs' / 'index.html'

        if ui_path.exists():
            try:
                # Convert to file:// URL
                file_url = ui_path.as_uri()
                webbrowser.open(file_url)
                return True
            except Exception:
                return False
        return False

    def _print_intro(self):
        """Print the introduction and overview."""
        # Open the companion guide in browser
        if self._open_companion_guide():
            print("""
📖 Opening the visual setup guide in your browser...

Follow along in the slides while the CLI guides you through each step.
The HTML guide explains the "why" behind each decision.
""")

        print("""
This wizard will help you set up the Daily Operating System - a
productivity framework built on Claude Code for managing your daily
work, tasks, and strategic thinking.

What we'll configure:
  1. Check prerequisites (Claude Code, Python, Git)
  2. Choose your workspace location
  3. Create the PARA directory structure
  4. Initialize Git repository
  5. Set up Google API integration (optional)
  6. Generate your CLAUDE.md configuration
  7. Install skills and commands
  8. Install web dashboard (optional)
  9. Set up Python tools
  10. Verify everything works

The entire process takes about 10-15 minutes.
""")

    def _step_prerequisites(self) -> bool:
        """Step 1: Check prerequisites."""
        print_step_header(1, "Prerequisites Check", self.TOTAL_STEPS)

        checks = []

        # Python version
        py_ok, py_version, py_err = validate_python_version((3, 8))
        if py_ok:
            checks.append((f"Python {py_version}", "done"))
        else:
            checks.append((f"Python: {py_err}", "fail"))

        # Claude Code
        cc_ok, cc_version, cc_err = validate_command_exists("claude")
        if cc_ok:
            checks.append((f"Claude Code: {cc_version[:50]}...", "done"))
        else:
            checks.append(("Claude Code: Not found", "fail"))
            print_warning("Install Claude Code: npm install -g @anthropic-ai/claude-code")

        # Git
        git_ok, git_version, git_err = validate_command_exists("git")
        if git_ok:
            checks.append((f"Git: {git_version[:40]}", "done"))
        else:
            checks.append(("Git: Not found", "fail"))

        print_checklist(checks, "Prerequisites")

        # Determine if we can proceed
        if not py_ok:
            print_error("Python 3.8+ is required. Please install and try again.")
            return False

        if not cc_ok:
            print_warning("Claude Code is recommended but not required for setup.")
            if not confirm("Continue without Claude Code?"):
                return False

        if not git_ok:
            print_warning("Git is recommended for version control.")
            self.config['skip_git'] = not confirm("Continue without Git?")
            if not self.config.get('skip_git', False) and not git_ok:
                return False

        press_enter_to_continue()
        return True

    def _step_workspace(self) -> bool:
        """Step 2: Choose workspace location."""
        print_step_header(2, "Workspace Location", self.TOTAL_STEPS)

        print("""
Choose where to create your productivity workspace. This directory
will contain all your documents, accounts, projects, and configuration.

Recommended locations:
  - ~/Documents/productivity
  - ~/productivity
  - ~/workspace
""")

        # Use provided path or prompt
        workspace = getattr(self.args, 'workspace', None)
        if workspace:
            workspace = Path(workspace).expanduser()
            print(f"Using provided workspace: {workspace}")
        else:
            workspace = prompt_path(
                "Workspace location",
                default="~/Documents/productivity"
            )
            workspace = Path(workspace)

        # Validate
        valid, err = validate_directory_writable(str(workspace))
        if not valid:
            print_error(err)
            return False

        # Check if exists and has content
        if workspace.exists() and any(workspace.iterdir()):
            print_warning(f"Directory exists and is not empty: {workspace}")
            choice = prompt_choice(
                "What would you like to do?",
                [
                    ("Use existing", "Keep existing files, add missing structure"),
                    ("Start fresh", "Remove existing content and start over"),
                    ("Choose different", "Pick a different location"),
                ],
                default=1
            )
            if choice == 2:
                if confirm("This will DELETE all files. Are you sure?", default=False):
                    import shutil
                    shutil.rmtree(workspace)
                else:
                    return self._step_workspace()  # Retry
            elif choice == 3:
                self.args.workspace = None
                return self._step_workspace()  # Retry

        self.config['workspace'] = workspace
        print_success(f"Workspace: {workspace}")

        # Collect workspace name and organization info
        print_section("Workspace Identity")

        workspace_name = prompt_text(
            "Workspace name",
            default=workspace.name.replace('-', ' ').replace('_', ' ').title()
        )
        self.config['workspace_name'] = workspace_name

        print("""
Organization settings help classify meetings and emails. Internal domains
are email domains that belong to your organization (e.g., company.com).
""")

        org_name = prompt_text(
            "Organization/company name",
            default=""
        )
        self.config['organization_name'] = org_name

        domains_input = prompt_text(
            "Internal email domains (comma-separated)",
            default=""
        )
        # Parse comma-separated domains
        if domains_input.strip():
            domains = [d.strip().lower() for d in domains_input.split(',') if d.strip()]
        else:
            domains = []
        self.config['internal_domains'] = domains

        if domains:
            print_success(f"Internal domains: {', '.join(domains)}")
        else:
            print_info("No internal domains set. You can configure this later in _config/workspace.json")

        press_enter_to_continue()
        return True

    def _step_directories(self) -> bool:
        """Step 3: Create PARA directory structure."""
        print_step_header(3, "Directory Structure", self.TOTAL_STEPS)

        print("""
Creating the PARA directory structure:

  Projects/    - Active initiatives with deadlines
  Areas/       - Ongoing responsibilities
  Resources/   - Reference materials
  Archive/     - Completed/inactive items

Plus supporting directories:
  _inbox/      - Unprocessed documents
  _today/      - Daily working files
  _templates/  - Document templates
  _tools/      - Automation scripts
""")

        # Ask about role to determine account structure
        print("""
The Accounts folder structure depends on how you work:
""")
        from steps.directories import get_role_choices

        role_choices = get_role_choices()
        role_options = [
            (role['name'], role['description'])
            for role in role_choices
        ]

        role_idx = prompt_choice(
            "How do you manage accounts?",
            role_options,
            default=1
        )
        self.config['role'] = role_choices[role_idx - 1]['key']

        print(f"\nSelected: {role_choices[role_idx - 1]['name']}")
        print_info("You can customize this later by asking Claude to reorganize.")

        if not confirm("\nCreate directory structure?"):
            return False

        try:
            self._create_directories()
            print_success("Directory structure created")

            # Write workspace configuration
            self._write_workspace_config()
            print_success("Workspace configuration saved")

            press_enter_to_continue()
            return True
        except FileOperationError as e:
            print_error(str(e))
            return False

    def _step_git(self) -> bool:
        """Step 4: Initialize Git repository."""
        print_step_header(4, "Git Setup", self.TOTAL_STEPS)

        if self.config.get('skip_git'):
            print_info("Git setup skipped (not installed)")
            press_enter_to_continue()
            return True

        workspace = self.config['workspace']

        # Check if already a git repo
        if (workspace / '.git').exists():
            print_info("Git repository already initialized")
            press_enter_to_continue()
            return True

        print("""
Initializing a Git repository for your workspace enables:
  - Version history for all documents
  - Easy backup to GitHub/GitLab
  - Collaboration with team members
  - Recovery from mistakes
""")

        if not confirm("Initialize Git repository?"):
            print_info("Git setup skipped")
            press_enter_to_continue()
            return True

        try:
            self._init_git()
            print_success("Git repository initialized")

            print_info("""
Recommended: Push to a private GitHub repository for backup.
Run these commands after setup:
  git remote add origin <your-repo-url>
  git push -u origin main
""")
            press_enter_to_continue()
            return True

        except Exception as e:
            print_error(f"Git setup failed: {e}")
            if confirm("Continue without Git?"):
                return True
            return False

    def _step_google_api(self) -> bool:
        """Step 5: Google API setup."""
        print_step_header(5, "Google API Integration", self.TOTAL_STEPS)

        print("""
Google API integration enables:
  - Calendar: View and create events
  - Gmail: Read emails, create drafts
  - Sheets: Read and update spreadsheets
  - Docs: Create and edit documents

This requires:
  1. A Google Cloud project
  2. OAuth credentials (credentials.json)
  3. One-time browser authorization
""")

        choice = prompt_choice(
            "How would you like to configure Google API?",
            [
                ("Full setup", "Configure all Google services now"),
                ("Read-only", "Only read access to Calendar/Gmail"),
                ("Skip", "Set up Google API later"),
            ],
            default=3
        )

        if choice == 3:
            print_info("Google API setup skipped")
            self.config['google_api'] = 'skip'
            press_enter_to_continue()
            return True

        self.config['google_api'] = 'full' if choice == 1 else 'readonly'

        # Check for existing credentials
        workspace = self.config['workspace']
        creds_dir = workspace / '.config' / 'google'
        creds_file = creds_dir / 'credentials.json'

        if creds_file.exists():
            print_success("credentials.json found")
        else:
            print_info("""
To set up Google API:

1. Go to: https://console.cloud.google.com/
2. Create a new project (or select existing)
3. Enable these APIs:
   - Google Calendar API
   - Gmail API
   - Google Sheets API
   - Google Docs API
4. Go to Credentials > Create Credentials > OAuth client ID
5. Choose "Desktop app" as application type
6. Download the JSON file
7. Save it as: {}/credentials.json
""".format(creds_dir))

            self.file_ops.create_directory(creds_dir)

            if not confirm("Have you saved credentials.json?"):
                print_info("You can complete Google setup later by running:")
                print(f"  python3 advanced-start.py --google --workspace {workspace}")
                press_enter_to_continue()
                return True

        # Copy google_api.py to workspace
        try:
            self._install_google_api_script()
            print_success("Google API helper installed")
        except Exception as e:
            print_warning(f"Could not install Google API script: {e}")

        print_info("Run the Google API script to complete authorization:")
        print(f"  python3 {creds_dir / 'google_api.py'} calendar list 1")

        press_enter_to_continue()
        return True

    def _step_claude_md(self) -> bool:
        """Step 6: Generate CLAUDE.md configuration."""
        print_step_header(6, "CLAUDE.md Configuration", self.TOTAL_STEPS)

        print("""
CLAUDE.md tells Claude Code about your workspace, preferences,
and how to help you effectively. It's like a personalized instruction
manual for your AI assistant.
""")

        choice = prompt_choice(
            "How would you like to create CLAUDE.md?",
            [
                ("Questionnaire", "Answer questions to generate personalized config"),
                ("Template", "Start with a basic template and edit later"),
                ("Skip", "Create CLAUDE.md manually later"),
            ],
            default=1
        )

        if choice == 3:
            print_info("CLAUDE.md creation skipped")
            press_enter_to_continue()
            return True

        if choice == 1:
            return self._claude_md_questionnaire()
        else:
            return self._claude_md_template()

    def _step_skills(self) -> bool:
        """Step 7: Install skills and commands."""
        print_step_header(7, "Skills & Commands", self.TOTAL_STEPS)

        print("""
Skills are specialized workflows that Claude Code can execute.
Commands are quick-access shortcuts for common operations.

Available skills:
  - inbox-processing: Three-phase document flow
  - strategy-consulting: McKinsey-style analysis
  - editorial: Writing review standards

Available commands:
  - /today: Morning dashboard
  - /wrap: End-of-day closure
  - /week: Weekly review
  - /month: Monthly roll-up
  - /quarter: Quarterly review
  - /email-scan: Email triage
  - /git-commit: Atomic commits
""")

        choice = prompt_choice(
            "Which components would you like to install?",
            [
                ("All", "Install all skills and commands (Recommended)"),
                ("Core only", "Just /today, /wrap, /week commands"),
                ("Custom", "Choose specific components"),
                ("None", "Skip for now"),
            ],
            default=1
        )

        if choice == 4:
            print_info("Skills installation skipped")
            press_enter_to_continue()
            return True

        try:
            if choice == 1:
                self._install_all_skills()
            elif choice == 2:
                self._install_core_skills()
            else:
                self._install_custom_skills()

            print_success("Skills and commands installed")
            press_enter_to_continue()
            return True

        except Exception as e:
            print_error(f"Installation failed: {e}")
            return False

    def _step_ui(self) -> bool:
        """Step 8: Install web dashboard."""
        print_step_header(8, "Web Dashboard", self.TOTAL_STEPS)

        print("""
The web dashboard provides a visual interface for navigating your
workspace. It runs locally on your machine and displays your
accounts, projects, and daily files in a browser.

Features:
  - Visual sidebar navigation
  - Markdown file rendering
  - Search across all documents
  - Account health indicators (for Customer Success roles)

Requirements:
  - Node.js (for running the server)
""")

        # Check if Node.js is available
        from steps.ui_setup import check_nodejs_available, setup_ui, get_templates_dir

        node_ok, node_info = check_nodejs_available()

        if node_ok:
            print_success(f"Node.js found: {node_info}")
        else:
            print_warning(f"Node.js not found: {node_info}")
            print_info("You can install Node.js later and run 'npm install' in _ui/")

        if not confirm("\nInstall web dashboard?"):
            print_info("Dashboard installation skipped")
            press_enter_to_continue()
            return True

        # Validate templates exist before proceeding
        templates_dir = get_templates_dir()
        ui_templates = templates_dir / 'ui'
        if not ui_templates.exists():
            print_error(f"UI templates not found at: {ui_templates}")
            print_info("This may happen if setup files were moved or partially installed.")
            print_info("Please run advanced-start.py from the original repository location,")
            print_info("or set DAILYOS_TEMPLATES environment variable to the templates path.")
            press_enter_to_continue()
            return True  # Continue setup without UI

        # Run the UI setup
        workspace = self.config['workspace']
        role = self.config.get('role', 'customer_success')

        with Spinner("Installing dashboard...") as spinner:
            try:
                result = setup_ui(
                    workspace=workspace,
                    file_ops=self.file_ops,
                    role=role,
                    workspace_name=workspace.name,
                    install_deps=node_ok
                )

                if result['success']:
                    spinner.succeed("Dashboard installed")
                else:
                    spinner.fail(f"Installation failed: {result['message']}")
                    return True  # Non-fatal, continue setup

            except Exception as e:
                spinner.fail(f"Installation failed: {e}")
                return True  # Non-fatal, continue setup

        # Show results
        if result.get('deps_installed'):
            print_success("Dependencies installed")
        elif node_ok:
            print_warning("Dependencies not installed - run 'npm install' in _ui/")

        # Update workspace config to mark dashboard as installed
        if result.get('success'):
            self._update_workspace_config({'features': {'web_dashboard': True}})

        print(result.get('startup_instructions', ''))

        press_enter_to_continue()
        return True

    def _step_python_tools(self) -> bool:
        """Step 9: Install Python tools."""
        print_step_header(9, "Python Tools", self.TOTAL_STEPS)

        print("""
Python tools provide automation for common tasks:

  Inbox Processing:
  - prepare_inbox.py: Prepare documents for processing
  - deliver_inbox.py: Deliver processed documents to PARA

  Daily Operating System:
  - prepare_today.py / deliver_today.py: Morning dashboard automation
  - prepare_wrap.py / deliver_wrap.py: End-of-day closure
  - prepare_week.py / deliver_week.py: Weekly review automation

  Account Management:
  - generate_dashboard.py: Create account dashboards

  Shared Libraries (_tools/lib/):
  - calendar_utils.py: Calendar API helpers
  - task_utils.py: Task list parsing
  - file_utils.py: File operations
  - meeting_utils.py: Meeting classification
""")

        if not confirm("Install Python tools?"):
            print_info("Python tools skipped")
            press_enter_to_continue()
            return True

        try:
            self._install_python_tools()
            self._update_workspace_config({'features': {'python_tools': True}})
            print_success("Python tools installed")
            press_enter_to_continue()
            return True
        except Exception as e:
            print_error(f"Installation failed: {e}")
            return False

    def _step_verification(self) -> bool:
        """Step 10: Verify installation."""
        print_step_header(10, "Verification", self.TOTAL_STEPS)

        print("Verifying installation...\n")
        return self._verify_installation()

    # =========================================================================
    # Helper Methods
    # =========================================================================

    def _check_prerequisites_silent(self) -> bool:
        """Check prerequisites without output."""
        py_ok, _, _ = validate_python_version((3, 8))
        return py_ok

    def _create_directories(self):
        """Create the PARA directory structure."""
        from steps.directories import create_all_directories

        workspace = self.config['workspace']
        role = self.config.get('role', 'customer_success')

        create_all_directories(workspace, self.file_ops, role)

    def _write_workspace_config(self):
        """Write the centralized workspace configuration file."""
        import json
        from datetime import datetime

        workspace = self.config['workspace']
        config_path = workspace / '_config' / 'workspace.json'

        # Ensure _config directory exists
        config_path.parent.mkdir(parents=True, exist_ok=True)

        config_data = {
            "$schema": "./workspace-schema.json",
            "workspace": {
                "name": self.config.get('workspace_name', workspace.name),
                "role": self.config.get('role', 'general')
            },
            "organization": {
                "name": self.config.get('organization_name', ''),
                "internal_domains": self.config.get('internal_domains', [])
            },
            "features": {
                "google_api": self.config.get('google_api', 'skip') != 'skip',
                "web_dashboard": False,  # Updated later in _step_ui
                "python_tools": False    # Updated later in _step_python_tools
            },
            "metadata": {
                "created_at": datetime.now().isoformat(),
                "setup_version": "1.0.0",
                "last_updated": datetime.now().isoformat()
            }
        }

        self.file_ops.write_file(config_path, json.dumps(config_data, indent=2))

        # Also copy the schema file
        script_dir = Path(__file__).parent.parent
        schema_src = script_dir / 'templates' / 'config' / 'workspace-schema.json'
        schema_dst = workspace / '_config' / 'workspace-schema.json'
        if schema_src.exists():
            self.file_ops.write_file(schema_dst, schema_src.read_text())

    def _update_workspace_config(self, updates: Dict[str, Any]):
        """Update specific fields in the workspace configuration file."""
        import json
        from datetime import datetime

        workspace = self.config['workspace']
        config_path = workspace / '_config' / 'workspace.json'

        if not config_path.exists():
            return  # Config not created yet

        try:
            with open(config_path) as f:
                config_data = json.load(f)

            # Deep merge updates
            for key, value in updates.items():
                if isinstance(value, dict) and key in config_data:
                    config_data[key].update(value)
                else:
                    config_data[key] = value

            # Update last_updated timestamp
            if 'metadata' in config_data:
                config_data['metadata']['last_updated'] = datetime.now().isoformat()

            with open(config_path, 'w') as f:
                json.dump(config_data, f, indent=2)

        except Exception as e:
            # Non-fatal - don't break setup if config update fails
            if self.verbose:
                print(f"Warning: Could not update workspace config: {e}")

    def _init_git(self):
        """Initialize Git repository."""
        import subprocess
        workspace = self.config['workspace']

        # Initialize repo
        subprocess.run(
            ['git', 'init'],
            cwd=workspace,
            capture_output=True,
            check=True
        )

        # Create .gitignore
        gitignore_content = """# Credentials and secrets
.config/google/token.json
.config/google/credentials.json
*.credentials
*.secret

# OS files
.DS_Store
Thumbs.db

# Editor files
*.swp
*.swo
*~

# Python
__pycache__/
*.py[cod]
.venv/
venv/

# Temporary files
*.tmp
*.bak
"""
        gitignore_path = workspace / '.gitignore'
        self.file_ops.write_file(gitignore_path, gitignore_content)

        # Initial commit
        subprocess.run(
            ['git', 'add', '.gitignore'],
            cwd=workspace,
            capture_output=True,
            check=True
        )
        subprocess.run(
            ['git', 'commit', '-m', 'Initial commit: Add .gitignore'],
            cwd=workspace,
            capture_output=True,
            check=True
        )

    def _install_google_api_script(self):
        """Install the Google API helper script."""
        workspace = self.config['workspace']
        script_path = workspace / '.config' / 'google' / 'google_api.py'

        # Find templates directory (relative to this script)
        script_dir = Path(__file__).parent.parent  # Go up from src/ to project root
        src_path = script_dir / 'templates' / 'scripts' / 'google' / 'google_api.py'

        if src_path.exists():
            content = src_path.read_text()
            self.file_ops.write_file(script_path, content)
        else:
            # Fallback placeholder if template not found
            placeholder = '''#!/usr/bin/env python3
"""
Google API Helper Script.

Template not found - please reinstall from the DailyOS repository.
"""
print("Google API script template not found")
'''
            self.file_ops.write_file(script_path, placeholder)

    def _create_basic_claude_md(self):
        """Create a basic CLAUDE.md from template."""
        workspace = self.config['workspace']
        claude_md_path = workspace / 'CLAUDE.md'

        content = '''# CLAUDE.md

This file provides guidance to Claude Code when working with this workspace.

## Repository Purpose

Personal productivity workspace using the PARA organizational system.

## Directory Structure

```
{workspace}/
├── Projects/     - Active initiatives with deadlines
├── Areas/        - Ongoing responsibilities
├── Resources/    - Reference materials
├── Archive/      - Completed/inactive items
├── _inbox/       - Unprocessed documents
├── _today/       - Daily working files
├── _templates/   - Document templates
└── _tools/       - Automation scripts
```

## Available Commands

| Command | Purpose |
|---------|---------|
| /today | Morning dashboard |
| /wrap | End-of-day closure |
| /week | Weekly review |

## Working Style

[Add your preferences here]

## Current Focus

[Add your current priorities here]
'''.format(workspace=workspace.name)

        self.file_ops.write_file(claude_md_path, content)

    def _claude_md_questionnaire(self) -> bool:
        """Generate CLAUDE.md through questionnaire."""
        print_section("About You")

        name = prompt_text("Your name", default="")
        role = prompt_text("Your role/title", default="")

        print_section("Working Style")

        energy_choice = prompt_choice(
            "When do you do your best work?",
            [
                ("Morning", "High energy in AM, fades in PM"),
                ("Afternoon", "Peak performance midday"),
                ("Evening", "Most productive later in day"),
                ("Varies", "Depends on the day"),
            ],
            default=1
        )
        energy_map = {1: "morning", 2: "afternoon", 3: "evening", 4: "varies"}
        energy = energy_map[energy_choice]

        comm_choice = prompt_choice(
            "Preferred communication style?",
            [
                ("Direct", "Straightforward, get to the point"),
                ("Diplomatic", "Thoughtful, consider all angles"),
                ("Collaborative", "Team-oriented, inclusive"),
            ],
            default=1
        )
        comm_map = {1: "direct", 2: "diplomatic", 3: "collaborative"}
        comm_style = comm_map[comm_choice]

        print_section("Current Focus")

        focus = prompt_text(
            "What are you currently focused on?",
            default="Professional development and productivity"
        )

        # Generate CLAUDE.md
        workspace = self.config['workspace']
        claude_md_path = workspace / 'CLAUDE.md'

        content = f'''# CLAUDE.md

This file provides guidance to Claude Code when working with this workspace.

## About {name or 'Me'}

**Role**: {role or '[Your role]'}

**Working Style**:
- Best work happens in the {energy}
- Communication style: {comm_style}
- [Add more preferences]

## Repository Purpose

Personal productivity workspace using the PARA organizational system.

## Directory Structure

```
{workspace.name}/
├── Projects/     - Active initiatives with deadlines
├── Areas/        - Ongoing responsibilities
├── Resources/    - Reference materials
├── Archive/      - Completed/inactive items
├── _inbox/       - Unprocessed documents
├── _today/       - Daily working files
├── _templates/   - Document templates
└── _tools/       - Automation scripts
```

## Current Focus

{focus}

## Available Commands

| Command | Purpose |
|---------|---------|
| /today | Morning dashboard - meeting prep, actions, email triage |
| /wrap | End-of-day closure - reconcile actions, capture impacts |
| /week | Weekly review - overview, hygiene alerts |
| /month | Monthly roll-up - aggregate impacts |
| /quarter | Quarterly review - pre-fill expectations |
| /email-scan | Email triage - surface important, archive noise |

## Guiding Principles

1. **Consuming, not producing** - You shouldn't have to maintain your productivity tools
2. **Works when you work** - The system adapts to your rhythm
3. **Everything changeable or removable** - No sacred cows
'''

        self.file_ops.write_file(claude_md_path, content)
        print_success("CLAUDE.md created")
        print_info(f"Edit {claude_md_path} to customize further")
        press_enter_to_continue()
        return True

    def _claude_md_template(self) -> bool:
        """Create CLAUDE.md from template."""
        self._create_basic_claude_md()
        workspace = self.config['workspace']
        print_success("CLAUDE.md template created")
        print_info(f"Edit {workspace / 'CLAUDE.md'} to customize")
        press_enter_to_continue()
        return True

    def _install_all_skills(self):
        """Install all skills and commands."""
        self._install_default_skills()

    def _install_core_skills(self):
        """Install only core commands."""
        self._install_default_skills()  # Same for now

    def _install_custom_skills(self):
        """Let user choose specific skills."""
        # For now, same as default
        self._install_default_skills()

    def _install_default_skills(self):
        """Install the default set of skills and commands."""
        workspace = self.config['workspace']

        # Find templates directory (relative to this script)
        script_dir = Path(__file__).parent.parent  # Go up from src/ to project root
        templates_dir = script_dir / 'templates'

        # Copy command files from templates
        commands = ['today', 'wrap', 'week', 'month', 'quarter', 'email-scan', 'git-commit', 'setup']
        for cmd in commands:
            src_path = templates_dir / 'commands' / f'{cmd}.md'
            dst_path = workspace / '.claude' / 'commands' / f'{cmd}.md'

            if src_path.exists():
                content = src_path.read_text()
                self.file_ops.write_file(dst_path, content)
            else:
                # Fallback to placeholder if template not found
                self.file_ops.write_file(dst_path, f'# /{cmd}\n\nCommand template not found.\n')

        # Copy skill packages from templates
        skills_src = templates_dir / 'skills'
        skills_dst = workspace / '.claude' / 'skills'
        if skills_src.exists():
            for skill_dir in skills_src.iterdir():
                if skill_dir.is_dir():
                    skill_name = skill_dir.name
                    # Copy all files in the skill directory
                    for skill_file in skill_dir.iterdir():
                        if skill_file.is_file():
                            dst_path = skills_dst / skill_name / skill_file.name
                            content = skill_file.read_text()
                            self.file_ops.write_file(dst_path, content)

        # Copy agent definitions from templates
        agents_src = templates_dir / 'agents'
        agents_dst = workspace / '.claude' / 'agents'
        if agents_src.exists():
            for agent_category in agents_src.iterdir():
                if agent_category.is_dir():
                    category_name = agent_category.name
                    for agent_file in agent_category.iterdir():
                        if agent_file.is_file():
                            dst_path = agents_dst / category_name / agent_file.name
                            content = agent_file.read_text()
                            self.file_ops.write_file(dst_path, content)

    def _install_python_tools(self):
        """Install Python automation tools."""
        workspace = self.config['workspace']
        tools_dir = workspace / '_tools'

        # Find templates directory (relative to this script)
        script_dir = Path(__file__).parent.parent  # Go up from src/ to project root
        templates_dir = script_dir / 'templates' / 'scripts'

        # Tool mappings: (source_subdir, source_file, dest_file)
        tools = [
            ('inbox', 'prepare_inbox.py', 'prepare_inbox.py'),
            ('inbox', 'deliver_inbox.py', 'deliver_inbox.py'),
            ('accounts', 'generate_account_dashboard.py', 'generate_account_dashboard.py'),
            # Daily operating scripts
            ('daily', 'prepare_today.py', 'prepare_today.py'),
            ('daily', 'deliver_today.py', 'deliver_today.py'),
            ('daily', 'prepare_wrap.py', 'prepare_wrap.py'),
            ('daily', 'deliver_wrap.py', 'deliver_wrap.py'),
            ('daily', 'prepare_week.py', 'prepare_week.py'),
            ('daily', 'deliver_week.py', 'deliver_week.py'),
        ]

        for subdir, src_name, dst_name in tools:
            src_path = templates_dir / subdir / src_name
            dst_path = tools_dir / dst_name

            if src_path.exists():
                content = src_path.read_text()
                self.file_ops.write_file(dst_path, content)

        # Install shared library modules to _tools/lib/
        lib_src = templates_dir / 'lib'
        lib_dst = tools_dir / 'lib'
        if lib_src.exists():
            for lib_file in lib_src.iterdir():
                if lib_file.is_file() and lib_file.suffix == '.py':
                    dst_path = lib_dst / lib_file.name
                    content = lib_file.read_text()
                    self.file_ops.write_file(dst_path, content)

        # Also install google_api.py to .config/google/
        google_src = templates_dir / 'google' / 'google_api.py'
        google_dst = workspace / '.config' / 'google' / 'google_api.py'
        if google_src.exists():
            content = google_src.read_text()
            self.file_ops.write_file(google_dst, content)

    def _verify_installation(self) -> bool:
        """Verify the installation is complete."""
        workspace = self.config.get('workspace')
        if not workspace:
            print_error("No workspace configured")
            return False

        checks = []

        # Check directories
        required_dirs = ['Projects', 'Areas', 'Resources', 'Archive', '_inbox', '_today']
        for d in required_dirs:
            if (workspace / d).exists():
                checks.append((f"Directory: {d}/", "done"))
            else:
                checks.append((f"Directory: {d}/", "fail"))

        # Check CLAUDE.md
        if (workspace / 'CLAUDE.md').exists():
            checks.append(("CLAUDE.md", "done"))
        else:
            checks.append(("CLAUDE.md", "pending"))

        # Check .claude directory
        if (workspace / '.claude' / 'commands').exists():
            checks.append(("Commands directory", "done"))
        else:
            checks.append(("Commands directory", "pending"))

        # Check Git
        if (workspace / '.git').exists():
            checks.append(("Git repository", "done"))
        else:
            checks.append(("Git repository", "skip"))

        # Check Google API
        if (workspace / '.config' / 'google' / 'credentials.json').exists():
            checks.append(("Google credentials", "done"))
        else:
            checks.append(("Google credentials", "skip"))

        # Check UI dashboard
        if (workspace / '_ui' / 'server.js').exists():
            checks.append(("Web dashboard", "done"))
        else:
            checks.append(("Web dashboard", "skip"))

        print_checklist(checks, "Installation Status")

        # Determine overall status
        fails = sum(1 for _, status in checks if status == "fail")
        if fails > 0:
            print_error(f"{fails} required components missing")
            return False

        print_success("Installation verified!")
        return True

    def _print_completion(self):
        """Print completion message and next steps."""
        workspace = self.config['workspace']

        print(f"""
{Colors.GREEN}{Colors.BOLD}
    ✅ Setup Complete!
{Colors.RESET}
Your Daily Operating System is ready at:
  {workspace}

{Colors.BOLD}Next Steps:{Colors.RESET}

1. Open your workspace in VS Code or terminal:
   cd {workspace}

2. Start Claude Code:
   claude

3. Run your first daily dashboard:
   /today

{Colors.BOLD}Quick Reference:{Colors.RESET}

  /today      - Morning dashboard
  /wrap       - End-of-day closure
  /week       - Weekly review
  /email-scan - Email triage

{Colors.BOLD}Web Dashboard:{Colors.RESET}

  cd {workspace}/_ui && npm start
  Open http://localhost:5050

{Colors.BOLD}Documentation:{Colors.RESET}

  The visual guide should already be open in your browser.
  If not, open: docs/index.html (or visit https://daily-os.com)

  Key slides to bookmark:
  • Commands Reference (slides 19-24)
  • Skills Reference (slides 13-18)
  • Account Structure (slide 31)

{Colors.DIM}Zero-guilt design: Consuming, not producing.
Works when you work. Everything changeable.{Colors.RESET}
""")
