#!/usr/bin/env python3
"""
DailyOS CLI - Version and workspace management.

Usage:
    dailyos version          Show version info
    dailyos status           Check for updates
    dailyos update           Update to latest version
    dailyos doctor           Check workspace health
    dailyos repair           Repair broken installation
    dailyos eject <name>     Eject skill/command for customization
    dailyos reset <name>     Reset ejected skill to symlink
"""

import argparse
import json
import shutil
import sys
from pathlib import Path

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from version import (
    CORE_PATH,
    get_core_version, get_workspace_version, get_ejected_skills,
    set_workspace_version, add_ejected_skill, remove_ejected_skill,
    check_for_updates, skip_version, git_pull_core, compare_versions,
    is_symlink_intact, get_workspace_status
)
from ui.prompts import (
    confirm, prompt_choice, print_section, show_update_prompt,
    show_doctor_results
)
from ui.colors import Colors, success, error, warning, info, dim, bold


def cmd_version(args) -> int:
    """Show version information."""
    workspace = Path(args.workspace).resolve()

    core_v = get_core_version()
    workspace_v = get_workspace_version(workspace)

    print(f"\n{bold('DailyOS Version Information')}\n")
    print(f"  Core version:      v{core_v}")
    print(f"  Workspace version: v{workspace_v}")
    print(f"  Core location:     {CORE_PATH}")
    print(f"  Workspace:         {workspace}")

    ejected = get_ejected_skills(workspace)
    if ejected:
        print(f"\n  Ejected skills: {', '.join(ejected)}")

    # Check if update available
    if compare_versions(core_v, workspace_v) > 0:
        print(f"\n  {warning('Update available!')} Run 'dailyos update' to upgrade.")

    print()
    return 0


def cmd_status(args) -> int:
    """Check for updates and show workspace status."""
    workspace = Path(args.workspace).resolve()

    print(f"\n{bold('DailyOS Status')}\n")

    update_info = check_for_updates(workspace)

    if update_info:
        print(f"  {info('Update available:')} v{update_info['current']} -> v{update_info['available']}")

        if update_info.get('changelog'):
            print(f"\n  {bold('Changes:')}")
            for entry in update_info['changelog'][:5]:
                print(f"    - {entry}")

        print(f"\n  Run 'dailyos update' to update.")
    else:
        print(f"  {success('Up to date')} (v{get_workspace_version(workspace)})")

    # Show workspace health summary
    status = get_workspace_status(workspace)
    if status['problems']:
        problem_count = len(status['problems'])
        print(f"\n  {warning(f'{problem_count} issue(s) detected:')}")
        for problem in status['problems']:
            print(f"    - {problem}")
        print(f"\n  Run 'dailyos doctor' for details.")

    print()
    return 0


def cmd_update(args) -> int:
    """Update core and sync workspace."""
    workspace = Path(args.workspace).resolve()

    print(f"\n{bold('DailyOS Update')}\n")

    # Check for updates first
    update_info = check_for_updates(workspace)
    if not update_info:
        print(f"  {success('Already up to date')} (v{get_workspace_version(workspace)})")
        return 0

    # Show what's changing
    print(f"  Updating: v{update_info['current']} -> v{update_info['available']}")

    if update_info.get('changelog'):
        print(f"\n  {bold('Changes:')}")
        for entry in update_info['changelog'][:5]:
            print(f"    - {entry}")

    if update_info.get('ejected'):
        print(f"\n  {warning('Ejected skills (will not update):')}")
        for skill in update_info['ejected']:
            print(f"    - {skill}")

    print()

    # Confirm
    if not args.yes and not confirm("Proceed with update?"):
        print("  Update cancelled.")
        return 0

    # Pull latest
    print("  Pulling latest from remote...", end='', flush=True)
    success_pull, output = git_pull_core()

    if not success_pull:
        print(f" {error('failed')}")
        print(f"  {error(output)}")
        return 1

    if "Already up to date" in output:
        print(f" {success('already up to date')}")
    else:
        print(f" {success('done')}")

    # Update version marker
    new_version = get_core_version()
    set_workspace_version(workspace, new_version)

    print(f"\n  {success(f'Workspace synced to v{new_version}')}")
    print(f"\n  Symlinked components (_tools, _ui, commands, skills)")
    print(f"  will use the new version automatically.")

    # Run repair to ensure symlinks are correct
    print(f"\n  Verifying symlinks...")
    _repair_symlinks(workspace, quiet=True)

    print()
    return 0


def cmd_doctor(args) -> int:
    """Check workspace health and offer repairs."""
    workspace = Path(args.workspace).resolve()
    problems = []

    results = {
        'workspace': str(workspace),
        'core': [],
        'workspace_checks': [],
        'commands': [],
        'skills': [],
        'problems': [],
    }

    # Check core
    if (CORE_PATH / '.git').exists():
        results['core'].append({'name': 'Git repo', 'ok': True})
    else:
        results['core'].append({'name': 'Git repo', 'ok': False, 'message': 'Not a git repo'})
        problems.append('core_not_git')

    results['core'].append({'name': f'Version: v{get_core_version()}', 'ok': True})

    # Check workspace symlinks
    for name in ['_tools', '_ui']:
        path = workspace / name
        if path.is_symlink() and path.resolve().exists():
            results['workspace_checks'].append({'name': name, 'ok': True})
        elif path.is_symlink():
            results['workspace_checks'].append({'name': name, 'ok': False, 'message': 'Broken symlink'})
            problems.append(f'broken_{name}')
        elif path.exists():
            results['workspace_checks'].append({'name': name, 'ok': False, 'message': 'Exists but not symlinked'})
            problems.append(f'not_symlinked_{name}')
        else:
            results['workspace_checks'].append({'name': name, 'ok': False, 'message': 'Missing'})
            problems.append(f'missing_{name}')

    # Check commands
    cmd_dir = workspace / '.claude' / 'commands'
    ejected = get_ejected_skills(workspace)

    for cmd in ['today', 'week', 'wrap', 'month', 'quarter', 'email-scan']:
        cmd_path = cmd_dir / f'{cmd}.md'
        cmd_name = f'{cmd}.md'

        if cmd_path.is_symlink() and cmd_path.resolve().exists():
            results['commands'].append({'name': cmd_name, 'status': 'symlinked'})
        elif cmd_path.exists() and cmd in ejected:
            results['commands'].append({'name': cmd_name, 'status': 'ejected'})
        elif cmd_path.exists():
            results['commands'].append({'name': cmd_name, 'status': 'file (not tracked)'})
        else:
            results['commands'].append({'name': cmd_name, 'status': 'missing'})
            problems.append(f'missing_cmd_{cmd}')

    # Check skills
    skills_dir = workspace / '.claude' / 'skills'
    for skill in ['inbox-processing', 'daily-csm', 'vip-editorial', 'strategy-consulting']:
        skill_path = skills_dir / skill

        if skill_path.is_symlink() and skill_path.resolve().exists():
            results['skills'].append({'name': skill, 'status': 'symlinked'})
        elif skill_path.exists() and skill in ejected:
            results['skills'].append({'name': skill, 'status': 'ejected'})
        elif skill_path.is_dir():
            results['skills'].append({'name': skill, 'status': 'directory (not tracked)'})
        elif skill_path.exists():
            results['skills'].append({'name': skill, 'status': 'file (unexpected)'})
        else:
            results['skills'].append({'name': skill, 'status': 'missing'})

    results['problems'] = problems

    # Display results
    show_doctor_results(results)

    # Offer repair
    if problems:
        print()
        if confirm("Run repair to fix automatically?"):
            return cmd_repair(args)

    print()
    return 0


def _repair_symlinks(workspace: Path, quiet: bool = False) -> int:
    """
    Internal function to repair symlinks.

    Args:
        workspace: Workspace path
        quiet: If True, suppress output

    Returns:
        Number of repairs made
    """
    repairs = 0

    # Repair _tools and _ui symlinks
    for name in ['_tools', '_ui']:
        workspace_path = workspace / name
        core_path = CORE_PATH / name

        if not core_path.exists():
            if not quiet:
                print(f"  {warning(f'Core {name} not found, skipping')}")
            continue

        needs_repair = False
        if workspace_path.is_symlink():
            if not workspace_path.resolve().exists():
                needs_repair = True
        elif workspace_path.exists():
            needs_repair = True
        else:
            needs_repair = True

        if needs_repair:
            # Backup existing if it's a directory
            if workspace_path.is_symlink():
                workspace_path.unlink()
            elif workspace_path.exists():
                backup = workspace_path.with_suffix('.backup')
                if backup.exists():
                    shutil.rmtree(backup)
                shutil.move(str(workspace_path), str(backup))
                if not quiet:
                    print(f"  Backed up {name} to {name}.backup")

            workspace_path.symlink_to(core_path)
            repairs += 1
            if not quiet:
                print(f"  {success(f'Repaired {name} symlink')}")

    # Repair commands
    cmd_dir = workspace / '.claude' / 'commands'
    cmd_dir.mkdir(parents=True, exist_ok=True)
    ejected = get_ejected_skills(workspace)

    for cmd in ['today', 'week', 'wrap', 'month', 'quarter', 'email-scan']:
        if cmd in ejected:
            continue  # Don't repair ejected commands

        cmd_path = cmd_dir / f'{cmd}.md'
        core_cmd = CORE_PATH / 'commands' / f'{cmd}.md'

        if not core_cmd.exists():
            continue

        if not cmd_path.is_symlink() or not cmd_path.resolve().exists():
            if cmd_path.exists() or cmd_path.is_symlink():
                cmd_path.unlink()
            cmd_path.symlink_to(core_cmd)
            repairs += 1
            if not quiet:
                print(f"  {success(f'Repaired {cmd}.md symlink')}")

    return repairs


def cmd_repair(args) -> int:
    """Repair broken symlinks and missing files."""
    workspace = Path(args.workspace).resolve()

    print(f"\n{bold('DailyOS Repair')}\n")

    repairs = _repair_symlinks(workspace, quiet=False)

    # Update version marker
    set_workspace_version(workspace, get_core_version())

    if repairs > 0:
        print(f"\n  {success(f'Repair complete ({repairs} fixes)')}")
    else:
        print(f"\n  {success('No repairs needed')}")

    print()
    return 0


def cmd_eject(args) -> int:
    """Eject a skill/command for customization."""
    workspace = Path(args.workspace).resolve()
    name = args.name

    print(f"\n{bold(f'Eject: {name}')}\n")

    # Find the file
    workspace_path = None
    core_path = None

    # Check commands first
    cmd_workspace = workspace / '.claude' / 'commands' / f'{name}.md'
    cmd_core = CORE_PATH / 'commands' / f'{name}.md'

    if cmd_workspace.exists() or cmd_core.exists():
        workspace_path = cmd_workspace
        core_path = cmd_core
        item_type = 'command'

    # Check skills
    if not workspace_path:
        skill_workspace = workspace / '.claude' / 'skills' / name
        skill_core = CORE_PATH / 'skills' / name

        if skill_workspace.exists() or skill_core.exists():
            workspace_path = skill_workspace
            core_path = skill_core
            item_type = 'skill'

    if not workspace_path:
        print(f"  {error(f'Not found: {name}')}")
        print(f"  Available commands: today, week, wrap, month, quarter, email-scan")
        print(f"  Available skills: inbox-processing, daily-csm, vip-editorial")
        return 1

    if not workspace_path.is_symlink():
        if workspace_path.exists():
            print(f"  {warning(f'{name} is already ejected (not a symlink)')}")
            return 0
        else:
            print(f"  {error(f'{name} does not exist in workspace')}")
            return 1

    if not core_path.exists():
        print(f"  {error(f'{name} not found in core')}")
        return 1

    # Warn user
    print(f"  This will:")
    print(f"    1. Copy {name} from core to your workspace")
    print(f"    2. Remove the symlink")
    print(f"    3. You will OWN this file and can customize it")
    print()
    print(f"  {warning('You will not receive automatic updates for this ' + item_type + '.')}")
    print(f"  To restore: dailyos reset {name}")
    print()

    if not confirm("Proceed?", default=False):
        print("  Cancelled.")
        return 0

    # Do the eject
    workspace_path.unlink()  # Remove symlink

    if core_path.is_dir():
        shutil.copytree(core_path, workspace_path)
    else:
        shutil.copy2(core_path, workspace_path)

    # Track ejected
    add_ejected_skill(workspace, name)

    print(f"\n  {success(f'Ejected {name}')}")
    print(f"  File location: {workspace_path}")
    print()
    return 0


def cmd_reset(args) -> int:
    """Reset an ejected skill back to symlink."""
    workspace = Path(args.workspace).resolve()
    name = args.name

    print(f"\n{bold(f'Reset: {name}')}\n")

    ejected = get_ejected_skills(workspace)
    if name not in ejected:
        print(f"  {warning(f'{name} is not ejected')}")
        return 0

    # Find the paths
    workspace_path = None
    core_path = None

    # Check commands
    cmd_workspace = workspace / '.claude' / 'commands' / f'{name}.md'
    cmd_core = CORE_PATH / 'commands' / f'{name}.md'

    if cmd_workspace.exists() or cmd_core.exists():
        workspace_path = cmd_workspace
        core_path = cmd_core

    # Check skills
    if not workspace_path:
        skill_workspace = workspace / '.claude' / 'skills' / name
        skill_core = CORE_PATH / 'skills' / name

        if skill_workspace.exists() or skill_core.exists():
            workspace_path = skill_workspace
            core_path = skill_core

    if not core_path or not core_path.exists():
        print(f"  {error(f'{name} not found in core')}")
        return 1

    # Warn user
    print(f"  This will:")
    print(f"    1. Delete your customized version of {name}")
    print(f"    2. Re-link to the core version")
    print()
    print(f"  {warning('Your customizations will be lost!')}")
    print()

    if not confirm("Proceed?", default=False):
        print("  Cancelled.")
        return 0

    # Create backup
    if workspace_path.exists():
        backup = workspace_path.with_suffix('.backup' if workspace_path.is_file()
                                            else '') if workspace_path.is_file() \
                 else Path(str(workspace_path) + '.backup')
        if backup.exists():
            if backup.is_dir():
                shutil.rmtree(backup)
            else:
                backup.unlink()
        if workspace_path.is_dir():
            shutil.move(str(workspace_path), str(backup))
        else:
            shutil.copy2(workspace_path, backup)
            workspace_path.unlink()
        print(f"  Backed up to: {backup}")

    # Create symlink
    workspace_path.symlink_to(core_path)

    # Remove from ejected list
    remove_ejected_skill(workspace, name)

    print(f"\n  {success(f'Reset {name} to core version')}")
    print()
    return 0


def main():
    parser = argparse.ArgumentParser(
        description='DailyOS workspace management',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    dailyos version              Show version info
    dailyos status               Check for updates
    dailyos update               Update to latest version
    dailyos update -y            Update without confirmation
    dailyos doctor               Check workspace health
    dailyos repair               Fix broken symlinks
    dailyos eject today          Customize the /today command
    dailyos reset today          Restore /today to core version
        """
    )
    parser.add_argument(
        '--workspace', '-w',
        default='.',
        help='Workspace path (default: current directory)'
    )

    subparsers = parser.add_subparsers(dest='command', help='Commands')

    # Version command
    subparsers.add_parser('version', help='Show version info')

    # Status command
    subparsers.add_parser('status', help='Check for updates')

    # Update command
    update_parser = subparsers.add_parser('update', help='Update to latest version')
    update_parser.add_argument('-y', '--yes', action='store_true', help='Skip confirmation')

    # Doctor command
    subparsers.add_parser('doctor', help='Check workspace health')

    # Repair command
    subparsers.add_parser('repair', help='Repair broken installation')

    # Eject command
    eject_parser = subparsers.add_parser('eject', help='Eject skill for customization')
    eject_parser.add_argument('name', help='Skill or command name to eject')

    # Reset command
    reset_parser = subparsers.add_parser('reset', help='Reset ejected skill to symlink')
    reset_parser.add_argument('name', help='Skill or command name to reset')

    args = parser.parse_args()

    commands = {
        'version': cmd_version,
        'status': cmd_status,
        'update': cmd_update,
        'doctor': cmd_doctor,
        'repair': cmd_repair,
        'eject': cmd_eject,
        'reset': cmd_reset,
    }

    if args.command in commands:
        return commands[args.command](args)
    else:
        parser.print_help()
        return 0


if __name__ == '__main__':
    sys.exit(main())
