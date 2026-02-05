"""
Base class for setup steps.

Provides common functionality for JSON input/output and progress reporting.
"""

import sys
import json
from abc import ABC, abstractmethod
from typing import Any, Dict, Optional
from pathlib import Path


def emit_progress(message: str, progress: int = 0):
    """
    Emit a progress update for SSE streaming.

    Format: PROGRESS:{"message": "...", "progress": N}
    """
    payload = json.dumps({"message": message, "progress": progress})
    print(f"PROGRESS:{payload}", file=sys.stdout, flush=True)


def emit_result(success: bool, result: Any = None, error: str = None, rollback_data: Dict = None):
    """
    Emit the final result as JSON.

    This is the last output from a step and will be parsed by Node.js.
    """
    output = {
        "success": success,
        "result": result,
        "error": error,
        "rollbackData": rollback_data,
    }
    print(json.dumps(output), file=sys.stdout, flush=True)


class SetupStep(ABC):
    """
    Base class for setup steps.

    Each step receives configuration, executes actions, and returns
    a result with optional rollback data.
    """

    step_id: str = "base"
    step_name: str = "Base Step"

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize with configuration.

        Args:
            config: Dictionary with workspace path, role, and other settings
        """
        self.config = config
        self.workspace = Path(config.get("workspacePath", "")).expanduser() if config.get("workspacePath") else None

    @abstractmethod
    def execute(self) -> Dict[str, Any]:
        """
        Execute the step.

        Returns:
            Dictionary with:
                - success: bool
                - result: Any (step-specific output)
                - rollbackData: Optional dict with data needed to undo
                - error: Optional error message if failed
        """
        pass

    def rollback(self, rollback_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Rollback the step.

        Override in subclasses that support rollback.

        Args:
            rollback_data: Data saved during execute()

        Returns:
            Dictionary with success status
        """
        return {"success": True, "message": "Nothing to rollback"}

    def progress(self, message: str, pct: int = 0):
        """Emit progress update."""
        emit_progress(message, pct)

    def validate_config(self, required_keys: list) -> Optional[str]:
        """
        Validate that required config keys are present.

        Returns error message if validation fails, None if OK.
        """
        missing = [k for k in required_keys if not self.config.get(k)]
        if missing:
            return f"Missing required config: {', '.join(missing)}"
        return None
