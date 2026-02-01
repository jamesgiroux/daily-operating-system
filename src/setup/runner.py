#!/usr/bin/env python3
"""
Setup Step Runner - JSON Interface for Web Wizard.

This script is called by the Node.js server to execute setup steps.
It reads JSON from stdin and outputs JSON + progress updates to stdout.

Usage (from Node.js):
    echo '{"step": "prerequisites", "config": {...}}' | python3 runner.py

Input JSON:
    {
        "step": "stepId",
        "config": {
            "workspacePath": "~/path",
            "role": "customer_success",
            ...
        }
    }

    OR for rollback:
    {
        "rollback": true,
        "step": "stepId",
        "rollbackData": {...}
    }

Output:
    PROGRESS:{"message": "Checking Python...", "progress": 25}
    PROGRESS:{"message": "Checking Claude Code...", "progress": 50}
    {"success": true, "result": {...}, "rollbackData": {...}}
"""

import sys
import json
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

from setup.base import emit_result
from setup.steps import STEPS


def main():
    """Main entry point."""
    # Read input from stdin
    try:
        input_data = json.loads(sys.stdin.read())
    except json.JSONDecodeError as e:
        emit_result(False, error=f"Invalid JSON input: {e}")
        return 1

    # Determine if this is a rollback or execute
    is_rollback = input_data.get("rollback", False)
    step_id = input_data.get("step")
    config = input_data.get("config", {})

    if not step_id:
        emit_result(False, error="Missing 'step' in input")
        return 1

    # Get the step class
    step_class = STEPS.get(step_id)
    if not step_class:
        emit_result(False, error=f"Unknown step: {step_id}")
        return 1

    # Create step instance
    step = step_class(config)

    try:
        if is_rollback:
            rollback_data = input_data.get("rollbackData", {})
            result = step.rollback(rollback_data)
        else:
            result = step.execute()

        # Output final result
        emit_result(
            success=result.get("success", True),
            result=result.get("result"),
            error=result.get("error"),
            rollback_data=result.get("rollbackData"),
        )
        return 0 if result.get("success", True) else 1

    except Exception as e:
        import traceback
        emit_result(False, error=str(e))
        traceback.print_exc(file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
