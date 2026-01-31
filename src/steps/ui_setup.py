"""
Step: UI Dashboard Setup.

Installs the optional web dashboard for visual navigation of the workspace.
"""

import json
import shutil
import subprocess
from pathlib import Path
from typing import Dict, Any, Optional

# Role to UI config mapping
# Note: project_management uses engineering.json as they share similar
# project-centric structure (Active/Backlog/Completed). The UI adapts
# based on what directories actually exist in the workspace.
ROLE_CONFIG_MAP = {
    'customer_success': 'customer-success.json',
    'sales': 'sales.json',
    'project_management': 'engineering.json',
    'product_management': 'product-management.json',
    'marketing': 'marketing.json',
    'engineering': 'engineering.json',
    'consulting': 'consulting.json',
    'general': 'general.json',
}


def get_templates_dir() -> Path:
    """Get the templates directory path."""
    return Path(__file__).parent.parent.parent / 'templates'


def check_nodejs_available() -> tuple[bool, str]:
    """
    Check if Node.js is available.

    Returns:
        Tuple of (is_available, version_or_error)
    """
    try:
        result = subprocess.run(
            ['node', '--version'],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0:
            return True, result.stdout.strip()
        return False, "Node.js not found"
    except FileNotFoundError:
        return False, "Node.js not installed"
    except subprocess.TimeoutExpired:
        return False, "Node.js check timed out"
    except Exception as e:
        return False, str(e)


def check_npm_available() -> tuple[bool, str]:
    """
    Check if npm is available.

    Returns:
        Tuple of (is_available, version_or_error)
    """
    try:
        result = subprocess.run(
            ['npm', '--version'],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0:
            return True, result.stdout.strip()
        return False, "npm not found"
    except FileNotFoundError:
        return False, "npm not installed"
    except subprocess.TimeoutExpired:
        return False, "npm check timed out"
    except Exception as e:
        return False, str(e)


def copy_ui_to_workspace(workspace: Path, file_ops) -> bool:
    """
    Copy the UI template to the workspace.

    Args:
        workspace: Target workspace path
        file_ops: FileOperations instance for tracking

    Returns:
        True if successful
    """
    templates_dir = get_templates_dir()
    ui_src = templates_dir / 'ui'
    ui_dst = workspace / '_ui'

    if not ui_src.exists():
        raise FileNotFoundError(f"UI template not found: {ui_src}")

    # Copy the entire UI directory
    if ui_dst.exists():
        # Remove existing to ensure clean copy
        shutil.rmtree(ui_dst)

    shutil.copytree(ui_src, ui_dst)
    return True


def generate_config(workspace: Path, role: str, workspace_name: Optional[str] = None) -> bool:
    """
    Generate the UI config.json based on role.

    Args:
        workspace: Target workspace path
        role: User's role (customer_success, sales, etc.)
        workspace_name: Optional custom workspace name

    Returns:
        True if successful
    """
    templates_dir = get_templates_dir()
    ui_dst = workspace / '_ui'
    config_dst = ui_dst / 'config' / 'config.json'

    # Get the role-specific config template
    role_config_file = ROLE_CONFIG_MAP.get(role, 'customer-success.json')
    role_config_path = templates_dir / 'ui' / 'config' / 'roles' / role_config_file

    if role_config_path.exists():
        # Use role-specific config as base
        with open(role_config_path) as f:
            config = json.load(f)
    else:
        # Fall back to reading the template config
        template_config = ui_dst / 'config' / 'config.json'
        if not template_config.exists():
            raise FileNotFoundError(
                f"Neither role config ({role_config_path}) nor template config ({template_config}) found"
            )
        with open(template_config) as f:
            content = f.read()

        # Replace placeholders
        display_name = workspace_name or workspace.name
        content = content.replace('{{WORKSPACE_NAME}}', display_name)
        content = content.replace('{{ROLE}}', role.replace('_', '-'))

        config = json.loads(content)

    # Update workspace name
    if workspace_name:
        config['workspace']['name'] = workspace_name
    elif config['workspace'].get('name', '').startswith('{{'):
        config['workspace']['name'] = workspace.name

    # Ensure role is set correctly
    config['workspace']['role'] = role.replace('_', '-')

    # Write the config
    with open(config_dst, 'w') as f:
        json.dump(config, f, indent=2)

    return True


def run_npm_install(workspace: Path) -> tuple[bool, str]:
    """
    Run npm install in the _ui directory.

    Args:
        workspace: Workspace path

    Returns:
        Tuple of (success, message)
    """
    ui_dir = workspace / '_ui'

    try:
        result = subprocess.run(
            ['npm', 'install'],
            cwd=ui_dir,
            capture_output=True,
            text=True,
            timeout=120  # 2 minute timeout
        )

        if result.returncode == 0:
            return True, "Dependencies installed successfully"
        else:
            return False, result.stderr or "npm install failed"

    except subprocess.TimeoutExpired:
        return False, "npm install timed out (>2 minutes)"
    except Exception as e:
        return False, str(e)


def get_startup_instructions(workspace: Path, has_node: bool) -> str:
    """
    Get the startup instructions for the UI.

    Args:
        workspace: Workspace path
        has_node: Whether Node.js is available

    Returns:
        Instructions string
    """
    ui_dir = workspace / '_ui'

    if has_node:
        return f"""
To start the dashboard:
  cd {ui_dir}
  npm start

Then open http://localhost:5050 in your browser.
"""
    else:
        return f"""
Node.js is required to run the dashboard.

After installing Node.js:
  cd {ui_dir}
  npm install
  npm start

Then open http://localhost:5050 in your browser.
"""


def setup_ui(
    workspace: Path,
    file_ops,
    role: str = 'customer_success',
    workspace_name: Optional[str] = None,
    install_deps: bool = True
) -> Dict[str, Any]:
    """
    Main UI setup function.

    Args:
        workspace: Target workspace path
        file_ops: FileOperations instance
        role: User's role
        workspace_name: Optional custom workspace name
        install_deps: Whether to run npm install

    Returns:
        Dictionary with setup results
    """
    result = {
        'success': False,
        'ui_installed': False,
        'config_generated': False,
        'deps_installed': False,
        'node_available': False,
        'message': '',
        'startup_instructions': ''
    }

    # Check Node.js availability
    node_ok, node_version = check_nodejs_available()
    result['node_available'] = node_ok
    if node_ok:
        result['node_version'] = node_version

    try:
        # Copy UI files
        copy_ui_to_workspace(workspace, file_ops)
        result['ui_installed'] = True

        # Generate config
        generate_config(workspace, role, workspace_name)
        result['config_generated'] = True

        # Install dependencies if Node.js available and requested
        if install_deps and node_ok:
            npm_ok, npm_version = check_npm_available()
            if npm_ok:
                deps_ok, deps_msg = run_npm_install(workspace)
                result['deps_installed'] = deps_ok
                if not deps_ok:
                    result['deps_message'] = deps_msg

        result['success'] = True
        result['startup_instructions'] = get_startup_instructions(workspace, node_ok)
        result['message'] = 'UI dashboard installed successfully'

    except Exception as e:
        result['success'] = False
        result['message'] = str(e)

    return result
