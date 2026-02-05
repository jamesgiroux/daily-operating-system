"""
Setup wizard step modules.

Each step module handles a specific phase of the setup process.
"""

from .prerequisites import check_all_prerequisites, get_prerequisite_install_instructions
from .directories import (
    create_all_directories,
    verify_directory_structure,
    get_directory_descriptions,
    get_directory_tree_display,
)
from .git_setup import (
    is_git_repo,
    init_git_repo,
    create_gitignore,
    create_initial_commit,
    get_git_remote_instructions,
    check_git_status,
)
from .google_api import (
    get_google_setup_instructions,
    check_credentials_exist,
    check_token_exists,
    get_api_features,
    install_google_api_script,
    verify_google_setup,
)
from .claude_md import (
    get_questionnaire_prompts,
    generate_claude_md,
    generate_basic_template,
    create_claude_md,
    verify_claude_md,
)
from .skills import (
    get_command_list,
    get_skill_list,
    install_command,
    install_skill,
    install_core_package,
    install_all_packages,
    verify_installation as verify_skills_installation,
)
from .python_tools import (
    get_tool_list,
    install_tool,
    install_all_tools,
    verify_tools_installation,
    create_requirements_txt,
)
from .verification import (
    run_full_verification,
    get_verification_summary,
)

__all__ = [
    # Prerequisites
    'check_all_prerequisites',
    'get_prerequisite_install_instructions',
    # Directories
    'create_all_directories',
    'verify_directory_structure',
    'get_directory_descriptions',
    'get_directory_tree_display',
    # Git
    'is_git_repo',
    'init_git_repo',
    'create_gitignore',
    'create_initial_commit',
    'get_git_remote_instructions',
    'check_git_status',
    # Google API
    'get_google_setup_instructions',
    'check_credentials_exist',
    'check_token_exists',
    'get_api_features',
    'install_google_api_script',
    'verify_google_setup',
    # CLAUDE.md
    'get_questionnaire_prompts',
    'generate_claude_md',
    'generate_basic_template',
    'create_claude_md',
    'verify_claude_md',
    # Skills
    'get_command_list',
    'get_skill_list',
    'install_command',
    'install_skill',
    'install_core_package',
    'install_all_packages',
    'verify_skills_installation',
    # Python tools
    'get_tool_list',
    'install_tool',
    'install_all_tools',
    'verify_tools_installation',
    'create_requirements_txt',
    # Verification
    'run_full_verification',
    'get_verification_summary',
]
