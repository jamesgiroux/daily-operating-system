"""
Setup step implementations for web wizard.

Each step wraps existing step module functions with JSON I/O.
"""

import os
import shutil
from pathlib import Path
from typing import Any, Dict

from setup.base import SetupStep


class WelcomeStep(SetupStep):
    """Welcome/intro step - no action, just validates session."""

    step_id = "welcome"
    step_name = "Welcome"

    def execute(self) -> Dict[str, Any]:
        return {
            "success": True,
            "result": {"message": "Welcome to Daily Operating System setup!"},
        }


class PrerequisitesStep(SetupStep):
    """Check system prerequisites."""

    step_id = "prerequisites"
    step_name = "Prerequisites Check"

    def execute(self) -> Dict[str, Any]:
        from steps.prerequisites import check_all_prerequisites

        self.progress("Checking system prerequisites...", 10)

        all_met, results = check_all_prerequisites()

        # Convert to JSON-friendly format
        checks = []
        for name, status, message in results:
            self.progress(f"Checking {name}...", 10 + len(checks) * 30)
            checks.append({
                "name": name,
                "status": status,  # 'ok', 'warn', 'fail'
                "message": message,
            })

        self.progress("Prerequisites check complete", 100)

        return {
            "success": True,  # Don't fail on warnings
            "result": {
                "allRequiredMet": all_met,
                "checks": checks,
            },
        }


class WorkspaceStep(SetupStep):
    """Validate and set workspace path."""

    step_id = "workspace"
    step_name = "Workspace Location"

    def execute(self) -> Dict[str, Any]:
        path_str = self.config.get("workspacePath", "")

        if not path_str:
            return {
                "success": False,
                "error": "No workspace path provided",
            }

        self.progress("Validating workspace path...", 25)

        # Expand and resolve path
        workspace = Path(path_str).expanduser().resolve()

        # Check if parent exists (we'll create the workspace)
        if not workspace.parent.exists():
            return {
                "success": False,
                "error": f"Parent directory does not exist: {workspace.parent}",
            }

        self.progress("Checking write permissions...", 50)

        # Check if we can write to parent
        try:
            test_file = workspace.parent / ".dailyos-test"
            test_file.touch()
            test_file.unlink()
        except (PermissionError, OSError) as e:
            return {
                "success": False,
                "error": f"Cannot write to directory: {e}",
            }

        self.progress("Creating workspace directory...", 75)

        # Create workspace if it doesn't exist
        created = False
        if not workspace.exists():
            workspace.mkdir(parents=True)
            created = True

        self.progress("Workspace ready", 100)

        return {
            "success": True,
            "result": {
                "workspacePath": str(workspace),
                "created": created,
                "exists": workspace.exists(),
            },
            "rollbackData": {
                "path": str(workspace),
                "created": created,
            },
        }

    def rollback(self, rollback_data: Dict[str, Any]) -> Dict[str, Any]:
        path_str = rollback_data.get("path")
        created = rollback_data.get("created", False)

        if created and path_str:
            workspace = Path(path_str)
            if workspace.exists() and not any(workspace.iterdir()):
                workspace.rmdir()
                return {"success": True, "message": f"Removed {path_str}"}

        return {"success": True, "message": "Nothing to rollback"}


class RoleStep(SetupStep):
    """Set user role for directory structure."""

    step_id = "role"
    step_name = "Choose Role"

    VALID_ROLES = [
        "customer_success",
        "sales",
        "project_management",
        "product_management",
        "marketing",
        "engineering",
        "consulting",
        "general",
    ]

    def execute(self) -> Dict[str, Any]:
        role = self.config.get("role", "")

        if not role:
            return {
                "success": False,
                "error": "No role selected",
            }

        if role not in self.VALID_ROLES:
            return {
                "success": False,
                "error": f"Invalid role: {role}. Valid options: {', '.join(self.VALID_ROLES)}",
            }

        self.progress(f"Role set to: {role}", 100)

        return {
            "success": True,
            "result": {
                "role": role,
            },
        }


class DirectoriesStep(SetupStep):
    """Create directory structure based on role."""

    step_id = "directories"
    step_name = "Directory Structure"

    def execute(self) -> Dict[str, Any]:
        from steps.directories import create_all_directories, get_directory_descriptions
        from utils.file_ops import FileOperations

        error = self.validate_config(["workspacePath", "role"])
        if error:
            return {"success": False, "error": error}

        self.progress("Planning directory structure...", 10)

        workspace = self.workspace
        role = self.config.get("role", "general")

        # Get descriptions for UI
        descriptions = get_directory_descriptions()

        self.progress("Creating directories...", 30)

        # Create file operations tracker
        file_ops = FileOperations()

        # Create directories (this function handles role-specific structure)
        try:
            created = create_all_directories(workspace, file_ops, role)
        except Exception as e:
            return {"success": False, "error": str(e)}

        self.progress("Directory structure created", 100)

        return {
            "success": True,
            "result": {
                "created": created,  # Already strings from the function
                "descriptions": descriptions,
            },
            "rollbackData": {
                "workspacePath": str(workspace),
                "created": [str(workspace / p) for p in created],
            },
        }

    def rollback(self, rollback_data: Dict[str, Any]) -> Dict[str, Any]:
        created = rollback_data.get("created", [])

        # Remove created directories in reverse order (deepest first)
        removed = []
        for path_str in sorted(created, reverse=True):
            path = Path(path_str)
            if path.exists() and path.is_dir():
                try:
                    if not any(path.iterdir()):
                        path.rmdir()
                        removed.append(path_str)
                except OSError:
                    pass  # Skip if not empty or error

        return {
            "success": True,
            "result": {"removed": removed},
        }


class GitStep(SetupStep):
    """Initialize git repository."""

    step_id = "git"
    step_name = "Git Setup"

    def execute(self) -> Dict[str, Any]:
        from steps.git_setup import is_git_repo, init_git_repo, create_gitignore
        from utils.file_ops import FileOperations

        error = self.validate_config(["workspacePath"])
        if error:
            return {"success": False, "error": error}

        skip = self.config.get("skipGit", False)
        if skip:
            return {
                "success": True,
                "result": {"skipped": True, "message": "Git setup skipped"},
            }

        workspace = self.workspace

        self.progress("Checking git status...", 20)

        already_repo = is_git_repo(workspace)

        if already_repo:
            return {
                "success": True,
                "result": {"alreadyRepo": True, "message": "Already a git repository"},
            }

        self.progress("Initializing git repository...", 50)

        try:
            init_git_repo(workspace)
        except Exception as e:
            return {"success": False, "error": f"Failed to init git: {e}"}

        self.progress("Creating .gitignore...", 75)

        try:
            file_ops = FileOperations()
            create_gitignore(workspace, file_ops)
        except Exception as e:
            return {"success": False, "error": f"Failed to create .gitignore: {e}"}

        self.progress("Git repository ready", 100)

        return {
            "success": True,
            "result": {
                "initialized": True,
                "gitignoreCreated": True,
            },
            "rollbackData": {
                "workspacePath": str(workspace),
            },
        }

    def rollback(self, rollback_data: Dict[str, Any]) -> Dict[str, Any]:
        workspace = Path(rollback_data.get("workspacePath", ""))
        git_dir = workspace / ".git"

        if git_dir.exists():
            shutil.rmtree(git_dir)
            return {"success": True, "message": "Removed .git directory"}

        return {"success": True, "message": "Nothing to rollback"}


class GoogleApiStep(SetupStep):
    """Set up Google API integration."""

    step_id = "google"
    step_name = "Google API"

    def execute(self) -> Dict[str, Any]:
        from steps.google_api import (
            check_credentials_exist,
            check_token_exists,
            get_api_features,
            install_google_api_script,
        )
        from utils.file_ops import FileOperations

        error = self.validate_config(["workspacePath"])
        if error:
            return {"success": False, "error": error}

        mode = self.config.get("googleApiMode", "skip")

        if mode == "skip":
            return {
                "success": True,
                "result": {"skipped": True, "message": "Google API setup skipped"},
            }

        workspace = self.workspace

        self.progress("Checking for existing credentials...", 20)

        has_credentials, _ = check_credentials_exist(workspace)
        has_token, _ = check_token_exists(workspace)

        self.progress("Installing Google API script...", 50)

        try:
            file_ops = FileOperations()
            install_google_api_script(workspace, file_ops)
        except Exception as e:
            return {"success": False, "error": f"Failed to install API script: {e}"}

        self.progress("Google API ready", 100)

        features = get_api_features()

        return {
            "success": True,
            "result": {
                "mode": mode,
                "hasCredentials": has_credentials,
                "hasToken": has_token,
                "needsAuth": not has_token,
                "features": features,
            },
            "rollbackData": {
                "workspacePath": str(workspace),
            },
        }


class ClaudeMdStep(SetupStep):
    """Generate CLAUDE.md configuration file."""

    step_id = "claudemd"
    step_name = "CLAUDE.md"

    def execute(self) -> Dict[str, Any]:
        from steps.claude_md import generate_claude_md, generate_basic_template, create_claude_md
        from utils.file_ops import FileOperations

        error = self.validate_config(["workspacePath"])
        if error:
            return {"success": False, "error": error}

        mode = self.config.get("claudeMdMode", "template")
        answers = self.config.get("claudeMdAnswers", {})
        workspace = self.workspace

        self.progress("Generating CLAUDE.md...", 30)

        # Check if file already exists
        claude_md_path = workspace / "CLAUDE.md"
        already_exists = claude_md_path.exists()

        if already_exists and not self.config.get("overwriteClaudeMd", False):
            return {
                "success": True,
                "result": {
                    "alreadyExists": True,
                    "path": str(claude_md_path),
                },
            }

        self.progress("Writing configuration file...", 70)

        try:
            # Generate content based on mode
            if mode == "questionnaire" and answers:
                # Pass workspace and answers dict to generate_claude_md
                content = generate_claude_md(workspace, answers)
            else:
                # Template mode just needs workspace
                content = generate_basic_template(workspace)

            # Create file with FileOperations for tracking
            file_ops = FileOperations()
            create_claude_md(workspace, content, file_ops)
        except Exception as e:
            return {"success": False, "error": f"Failed to create CLAUDE.md: {e}"}

        self.progress("CLAUDE.md created", 100)

        return {
            "success": True,
            "result": {
                "created": True,
                "path": str(claude_md_path),
                "mode": mode,
            },
            "rollbackData": {
                "path": str(claude_md_path),
                "alreadyExisted": already_exists,
            },
        }

    def rollback(self, rollback_data: Dict[str, Any]) -> Dict[str, Any]:
        path = Path(rollback_data.get("path", ""))
        already_existed = rollback_data.get("alreadyExisted", False)

        if not already_existed and path.exists():
            path.unlink()
            return {"success": True, "message": "Removed CLAUDE.md"}

        return {"success": True, "message": "Nothing to rollback"}


class SkillsStep(SetupStep):
    """Install skills and commands."""

    step_id = "skills"
    step_name = "Skills & Commands"

    def execute(self) -> Dict[str, Any]:
        from steps.skills import (
            get_command_list,
            get_skill_list,
            install_core_package,
            install_all_packages,
        )
        from utils.file_ops import FileOperations

        error = self.validate_config(["workspacePath"])
        if error:
            return {"success": False, "error": error}

        mode = self.config.get("skillsMode", "core")
        workspace = self.workspace

        if mode == "none":
            return {
                "success": True,
                "result": {"skipped": True, "message": "Skills installation skipped"},
            }

        self.progress("Getting available skills...", 20)

        commands = get_command_list()
        skills = get_skill_list()

        self.progress("Installing skills...", 40)

        try:
            file_ops = FileOperations()
            if mode == "all":
                installed = install_all_packages(workspace, file_ops)
            else:
                installed = install_core_package(workspace, file_ops)
        except Exception as e:
            return {"success": False, "error": f"Failed to install skills: {e}"}

        self.progress("Skills installed", 100)

        return {
            "success": True,
            "result": {
                "mode": mode,
                "installed": installed,
                "availableCommands": commands,
                "availableSkills": skills,
            },
            "rollbackData": {
                "workspacePath": str(workspace),
                "installed": installed,
            },
        }


class VerificationStep(SetupStep):
    """Verify the installation."""

    step_id = "verification"
    step_name = "Verification"

    def execute(self) -> Dict[str, Any]:
        from steps.verification import run_full_verification, get_verification_summary

        error = self.validate_config(["workspacePath"])
        if error:
            return {"success": False, "error": error}

        workspace = self.workspace

        self.progress("Running verification checks...", 20)

        try:
            results = run_full_verification(workspace)
        except Exception as e:
            return {"success": False, "error": f"Verification failed: {e}"}

        self.progress("Generating summary...", 80)

        summary = get_verification_summary(results)

        self.progress("Verification complete", 100)

        # Convert to UI-friendly format
        checks = []
        for section_name, section in results.get("sections", {}).items():
            if isinstance(section, dict) and "results" in section:
                for item in section["results"]:
                    checks.append({
                        "name": item.get("name", section_name),
                        "passed": item.get("status") == "ok",
                        "message": item.get("description", item.get("status", "")),
                    })
            elif isinstance(section, dict):
                # Simple section like git
                checks.append({
                    "name": section_name.replace("_", " ").title(),
                    "passed": section.get("status") == "ok",
                    "message": section.get("status", ""),
                })

        summary_data = results.get("summary", {})
        all_passed = summary_data.get("failed", 0) == 0

        return {
            "success": True,
            "result": {
                "allPassed": all_passed,
                "checks": checks,
                "summary": summary,
                "stats": summary_data,
            },
        }


class CompleteStep(SetupStep):
    """Mark setup as complete."""

    step_id = "complete"
    step_name = "Complete"

    def execute(self) -> Dict[str, Any]:
        workspace = self.workspace

        self.progress("Finalizing setup...", 50)

        # Could write a completion marker or log
        if workspace:
            completion_marker = workspace / ".dailyos-setup-complete"
            completion_marker.touch()

        self.progress("Setup complete!", 100)

        return {
            "success": True,
            "result": {
                "message": "Daily Operating System is ready!",
                "workspacePath": str(workspace) if workspace else None,
                "nextSteps": [
                    "Open your workspace in your preferred editor",
                    "Run 'claude' to start Claude Code",
                    "Try '/today' to see your first daily dashboard",
                ],
            },
        }


# Step registry
STEPS = {
    "welcome": WelcomeStep,
    "prerequisites": PrerequisitesStep,
    "workspace": WorkspaceStep,
    "role": RoleStep,
    "directories": DirectoriesStep,
    "git": GitStep,
    "google": GoogleApiStep,
    "claudemd": ClaudeMdStep,
    "skills": SkillsStep,
    "verification": VerificationStep,
    "complete": CompleteStep,
}
