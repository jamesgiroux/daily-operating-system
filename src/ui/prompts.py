"""
Interactive prompts for the setup wizard and workspace management.
"""

import sys
from pathlib import Path
from typing import Optional, List, Tuple, Dict
from .colors import Colors, success, error, warning, info, dim, highlight, bold


def print_banner():
    """Print the welcome banner."""
    banner = f"""
{Colors.CYAN}{Colors.BOLD}
    ╔══════════════════════════════════════════════════════════════╗
    ║                                                              ║
    ║           Daily Operating System Setup Wizard                ║
    ║                                                              ║
    ║   A productivity system built on Claude Code for managing    ║
    ║   your daily work, customer relationships, and strategic     ║
    ║   thinking.                                                  ║
    ║                                                              ║
    ╚══════════════════════════════════════════════════════════════╝
{Colors.RESET}"""
    print(banner)


def print_step_header(step_number: int, title: str, total_steps: int = 9):
    """Print a step header."""
    print(f"\n{Colors.BOLD}{'═' * 63}{Colors.RESET}")
    print(f"{Colors.BOLD}Step {step_number}/{total_steps}: {title}{Colors.RESET}")
    print(f"{Colors.BOLD}{'═' * 63}{Colors.RESET}\n")


def print_section(title: str):
    """Print a section header."""
    print(f"\n{Colors.CYAN}{title}{Colors.RESET}")
    print(f"{'-' * 40}\n")


def confirm(message: str, default: bool = True) -> bool:
    """Ask for yes/no confirmation."""
    suffix = " [Y/n]: " if default else " [y/N]: "
    response = input(f"{message}{suffix}").strip().lower()

    if not response:
        return default
    return response in ("y", "yes")


def prompt_text(message: str, default: Optional[str] = None) -> str:
    """Prompt for text input."""
    if default:
        prompt_str = f"{message} [{default}]: "
    else:
        prompt_str = f"{message}: "

    response = input(prompt_str).strip()
    return response if response else (default or "")


def prompt_path(message: str, default: Optional[str] = None) -> str:
    """Prompt for a path with expansion."""
    import os
    response = prompt_text(message, default)
    return os.path.expanduser(response)


def prompt_choice(
    message: str,
    options: List[Tuple[str, str]],
    default: int = 1
) -> int:
    """
    Prompt for a numbered choice.

    Args:
        message: The prompt message
        options: List of (label, description) tuples
        default: Default option number (1-indexed)

    Returns:
        Selected option number (1-indexed)
    """
    print(f"\n{message}\n")

    for i, (label, description) in enumerate(options, 1):
        marker = f"{Colors.GREEN}*{Colors.RESET}" if i == default else " "
        print(f"  {marker} {i}. {Colors.BOLD}{label}{Colors.RESET}")
        if description:
            print(f"       {dim(description)}")
        print()

    while True:
        response = input(f"Enter choice (1-{len(options)}) [{default}]: ").strip()

        if not response:
            return default

        try:
            choice = int(response)
            if 1 <= choice <= len(options):
                return choice
            print(f"{error('Invalid choice.')} Please enter a number between 1 and {len(options)}.")
        except ValueError:
            print(f"{error('Invalid input.')} Please enter a number.")


def prompt_multi_choice(
    message: str,
    options: List[Tuple[str, str]],
    defaults: Optional[List[int]] = None
) -> List[int]:
    """
    Prompt for multiple selections.

    Args:
        message: The prompt message
        options: List of (label, description) tuples
        defaults: List of default selections (1-indexed)

    Returns:
        List of selected option numbers (1-indexed)
    """
    selected = set(defaults or [])

    print(f"\n{message}")
    print(f"{dim('(Press Enter to toggle, type done when finished)')}\n")

    while True:
        for i, (label, description) in enumerate(options, 1):
            marker = f"{Colors.GREEN}[*]{Colors.RESET}" if i in selected else "[ ]"
            print(f"  {marker} {i}. {label}")
            if description:
                print(f"       {dim(description)}")
        print()

        response = input("Toggle (1-{}) or 'done': ".format(len(options))).strip().lower()

        if response == "done" or response == "":
            break

        try:
            choice = int(response)
            if 1 <= choice <= len(options):
                if choice in selected:
                    selected.remove(choice)
                else:
                    selected.add(choice)
        except ValueError:
            print(f"{error('Invalid input.')}")

    return sorted(selected)


def press_enter_to_continue(message: str = "Press Enter to continue..."):
    """Wait for user to press Enter."""
    input(f"\n{dim(message)}")


def print_success(message: str):
    """Print a success message with checkmark."""
    print(f"{Colors.GREEN}✓{Colors.RESET} {message}")


def print_warning(message: str):
    """Print a warning message."""
    print(f"{Colors.YELLOW}⚠{Colors.RESET} {message}")


def print_error(message: str):
    """Print an error message."""
    print(f"{Colors.RED}✗{Colors.RESET} {message}")


def print_info(message: str):
    """Print an info message."""
    print(f"{Colors.CYAN}ℹ{Colors.RESET} {message}")


def print_bullet(message: str, indent: int = 2):
    """Print a bullet point."""
    spaces = " " * indent
    print(f"{spaces}• {message}")


def show_update_prompt(update_info: dict) -> int:
    """
    Show the version update prompt.

    Args:
        update_info: Dict with 'current', 'available', 'changelog', 'ejected' keys

    Returns:
        User choice (1=update, 2=remind tomorrow, 3=skip version, 4=show changelog)
    """
    print(f"\n{Colors.CYAN}{'=' * 63}{Colors.RESET}")
    print(f"  {Colors.BOLD}DailyOS Update Available{Colors.RESET}")
    print(f"{Colors.CYAN}{'=' * 63}{Colors.RESET}\n")

    print(f"  Current: v{update_info['current']} -> Available: v{update_info['available']}\n")

    # Show changelog summary
    changelog = update_info.get('changelog', [])
    if changelog:
        print(f"  {Colors.BOLD}What's New:{Colors.RESET}")
        for entry in changelog[:5]:  # Max 5 items
            print(f"    {dim('-')} {entry}")
        print()

    # Warn about ejected skills
    ejected = update_info.get('ejected', [])
    if ejected:
        print(f"  {warning('Your customized skills will not auto-update:')}")
        for skill in ejected:
            print(f"       - {skill}")
        print()

    # Safety message
    print(f"  {dim('Your data (Accounts/, Projects/) is never touched.')}\n")

    options = [
        ("Update now", "Pull latest and sync workspace"),
        ("Remind me tomorrow", "Continue with current version"),
        ("Skip this version", "Don't ask again until next release"),
        ("Show full changelog", "View detailed changes"),
    ]

    return prompt_choice("What would you like to do?", options, default=1)


def show_doctor_results(results: dict) -> None:
    """Display doctor check results."""
    print(f"\n{bold('DailyOS Health Check')}")
    print("=" * 40)

    # Core status
    print_section("Core (~/.dailyos):")
    for check in results.get('core', []):
        status = success('ok') if check['ok'] else error('FAIL')
        print(f"    {check['name']}: {status}")
        if not check['ok'] and check.get('message'):
            print(f"      {dim(check['message'])}")

    # Workspace status
    print_section(f"Workspace ({results.get('workspace', '.')}):")
    for check in results.get('workspace_checks', []):
        status = success('ok') if check['ok'] else error('FAIL')
        print(f"    {check['name']}: {status}")
        if not check['ok'] and check.get('message'):
            print(f"      {dim(check['message'])}")

    # Commands
    print_section("Commands:")
    for cmd in results.get('commands', []):
        if cmd['status'] == 'symlinked':
            status = success('symlinked')
        elif cmd['status'] == 'ejected':
            status = f"{Colors.YELLOW}ejected{Colors.RESET}"
        elif cmd['status'] == 'missing':
            status = error('MISSING')
        else:
            status = warning(cmd['status'])
        print(f"    {cmd['name']}: {status}")

    # Skills
    print_section("Skills:")
    for skill in results.get('skills', []):
        if skill['status'] == 'symlinked':
            status = success('symlinked')
        elif skill['status'] == 'ejected':
            status = f"{Colors.YELLOW}ejected{Colors.RESET}"
        elif skill['status'] == 'missing':
            status = error('MISSING')
        else:
            status = warning(skill['status'])
        print(f"    {skill['name']}: {status}")

    # Summary
    problems = results.get('problems', [])
    if problems:
        print(f"\n{warning(f'Problems found: {len(problems)}')}")
    else:
        print(f"\n{success('Everything looks healthy')}")


# ============================================================================
# Workspace Detection Prompts
# ============================================================================

def prompt_workspace_selection(workspaces: List[Dict]) -> Optional[int]:
    """
    Prompt user to select from multiple workspaces.

    Args:
        workspaces: List of workspace dicts with:
            - path: Path object
            - name: Display name
            - version: Version string
            - last_used: Optional ISO timestamp

    Returns:
        Selected index (0-indexed), or None if cancelled
    """
    from workspace import format_relative_time

    print(f"\n  Found {len(workspaces)} workspace{'s' if len(workspaces) > 1 else ''}:\n")

    for i, ws in enumerate(workspaces, 1):
        path = ws.get('path')
        name = ws.get('name', path.name if isinstance(path, Path) else 'Unknown')
        last_used = format_relative_time(ws.get('last_used'))

        # Format path for display (use ~ for home)
        try:
            display_path = f"~/{path.relative_to(Path.home())}"
        except (ValueError, AttributeError):
            display_path = str(path)

        print(f"    {Colors.BOLD}{i}.{Colors.RESET} {display_path}")
        if last_used != "never":
            print(f"       {dim(f'Last used: {last_used}')}")
        print()

    while True:
        try:
            response = input(f"  Select workspace [1]: ").strip()

            if not response:
                return 0  # Default to first

            choice = int(response)
            if 1 <= choice <= len(workspaces):
                return choice - 1

            print(f"  {error('Invalid choice.')} Enter a number between 1 and {len(workspaces)}.")

        except ValueError:
            print(f"  {error('Invalid input.')} Enter a number.")
        except (KeyboardInterrupt, EOFError):
            print()
            return None


def show_no_workspace_error(scan_summary: List[str]) -> None:
    """
    Display helpful error when no workspace is found.

    Args:
        scan_summary: List of location descriptions that were checked
    """
    print(f"\n  {error('No workspace found.')}\n")

    print("  We looked in:")
    for location in scan_summary:
        print(f"    - {location}")

    print()
    print("  To create a workspace, run the setup wizard:")
    print(f"    {info('./easy-start.command')}")
    print()
    print("  Or specify a workspace directly:")
    print(f"    {info('dailyos start -w /path/to/workspace')}")
    print()


def show_invalid_workspace_error(path: Path, reason: str) -> None:
    """
    Display error when specified workspace is invalid.

    Args:
        path: The invalid path
        reason: Why it's invalid
    """
    print(f"\n  {error('Invalid workspace:')} {path}\n")
    print(f"  {reason}")
    print()
    print("  To create a new workspace, run the setup wizard:")
    print(f"    {info('./easy-start.command')}")
    print()


def confirm_save_default(workspace: Path) -> bool:
    """
    Ask user if they want to save workspace as default.

    Args:
        workspace: The workspace to save

    Returns:
        True if user wants to save, False otherwise
    """
    # Format path for display
    try:
        display_path = f"~/{workspace.relative_to(Path.home())}"
    except ValueError:
        display_path = str(workspace)

    return confirm(f"Save {display_path} as default?", default=True)


def show_workspace_found(workspace: Path, method: str) -> None:
    """
    Display message about which workspace was found and how.

    Args:
        workspace: The resolved workspace
        method: How it was found (from WorkspaceResolver.METHOD_*)
    """
    # Format path for display
    try:
        display_path = f"~/{workspace.relative_to(Path.home())}"
    except ValueError:
        display_path = str(workspace)

    method_descriptions = {
        'explicit': 'specified',
        'cwd': 'current directory',
        'config': 'default',
        'auto-single': 'auto-detected',
        'auto-selected': 'selected',
    }

    method_desc = method_descriptions.get(method, method)

    if method == 'config':
        print(f"  Using: {info(display_path)}")
    else:
        print(f"  Found workspace: {info(display_path)}")


def show_config_info(config: Dict) -> None:
    """
    Display current configuration information.

    Args:
        config: The configuration dictionary
    """
    print(f"\n{bold('DailyOS Configuration')}\n")

    # Default workspace
    default = config.get('default_workspace')
    if default:
        try:
            display_path = f"~/{Path(default).relative_to(Path.home())}"
        except ValueError:
            display_path = default
        print(f"  Default workspace: {info(display_path)}")
    else:
        print(f"  Default workspace: {dim('(not set)')}")

    # Scan locations
    print(f"\n  Scan locations:")
    for loc in config.get('scan_locations', []):
        try:
            display_path = f"~/{Path(loc).relative_to(Path.home())}"
        except ValueError:
            display_path = loc

        if Path(loc).exists():
            print(f"    - {display_path}")
        else:
            print(f"    - {display_path} {dim('(not found)')}")

    print(f"\n  Scan depth: {config.get('scan_depth', 2)}")

    # Known workspaces
    known = config.get('known_workspaces', [])
    if known:
        print(f"\n  Known workspaces:")
        from workspace import format_relative_time
        for ws in known[:5]:  # Show max 5
            try:
                display_path = f"~/{Path(ws['path']).relative_to(Path.home())}"
            except ValueError:
                display_path = ws['path']
            last_used = format_relative_time(ws.get('last_used'))
            print(f"    - {display_path} ({last_used})")
        if len(known) > 5:
            print(f"    ... and {len(known) - 5} more")

    # Preferences
    prefs = config.get('preferences', {})
    print(f"\n  Preferences:")
    print(f"    Auto-save default: {success('yes') if prefs.get('auto_save_default') else dim('no')}")
    print(f"    Prompt on multiple: {success('yes') if prefs.get('prompt_on_multiple') else dim('no')}")

    print()
