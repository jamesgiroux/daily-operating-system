#!/usr/bin/env python3
"""
Daily Operating System - Setup Wizard

A productivity system built on Claude Code for managing your daily work,
customer relationships, and strategic thinking.

Usage:
    python3 setup.py              # Run full setup wizard
    python3 setup.py --google     # Run only Google API setup
    python3 setup.py --verify     # Verify existing installation
    python3 setup.py --help       # Show help

For more information, see docs/getting-started.md
"""

import sys
import argparse
from pathlib import Path

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent / "src"))

from wizard import SetupWizard


def main():
    parser = argparse.ArgumentParser(
        description="Daily Operating System - Setup Wizard",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    python3 setup.py                  # Full setup wizard
    python3 setup.py --google         # Google API setup only
    python3 setup.py --verify         # Verify installation
    python3 setup.py --workspace ~/Work  # Specify workspace location
        """
    )

    parser.add_argument(
        "--workspace", "-w",
        help="Workspace location (default: ~/Documents/DailyOS)"
    )
    parser.add_argument(
        "--google",
        action="store_true",
        help="Run only Google API setup"
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Verify existing installation"
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Quick setup with sensible defaults"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Verbose output"
    )

    args = parser.parse_args()

    # Create and run wizard
    wizard = SetupWizard(args)

    try:
        if args.google:
            exit_code = wizard.run_google_setup_only()
        elif args.verify:
            exit_code = wizard.run_verification_only()
        elif args.quick:
            exit_code = wizard.run_quick_setup()
        else:
            exit_code = wizard.run()

        sys.exit(exit_code)

    except KeyboardInterrupt:
        print("\n\nSetup cancelled by user.")
        sys.exit(130)
    except Exception as e:
        print(f"\nError: {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
