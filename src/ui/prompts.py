"""
Interactive prompts for the setup wizard.
"""

import sys
from typing import Optional, List, Tuple
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
