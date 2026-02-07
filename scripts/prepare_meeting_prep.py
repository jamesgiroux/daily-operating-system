#!/usr/bin/env python3
"""Phase 1: Single Meeting Preparation for DailyOS Daybreak.

Generates prep context for ONE meeting. Called by:
  - Calendar polling (when a new meeting is detected)
  - Manual refresh (user clicks "Refresh prep" for a meeting)

Reads meeting JSON from stdin or a file argument.
Writes a single-meeting directive to _today/data/prep-directives/{meeting_id}.json.

Phase 7a of ADR-0030 (composable workflow operations).

Usage:
    echo '{"id":"evt-1","title":"Acme QBR",...}' | python3 prepare_meeting_prep.py [workspace]
    python3 prepare_meeting_prep.py [workspace] meeting.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from ops.config import (
    resolve_workspace,
    load_config,
    get_user_domain,
    write_json,
    _info,
    _warn,
)
from ops.calendar_fetch import classify_meeting
from ops.meeting_prep import gather_meeting_context
from ops.id_gen import make_meeting_id


def _read_meeting_input() -> dict[str, Any]:
    """Read meeting JSON from stdin or file argument.

    If a second positional arg exists and is a file, reads from it.
    Otherwise reads from stdin.
    """
    # Check for file argument (argv[2] since argv[1] is workspace)
    if len(sys.argv) > 2:
        meeting_path = Path(sys.argv[2])
        if meeting_path.exists():
            try:
                return json.loads(meeting_path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError) as exc:
                _warn(f"Failed to read meeting file {meeting_path}: {exc}")
                sys.exit(1)

    # Fall back to stdin
    if sys.stdin.isatty():
        _warn("No meeting data. Pipe JSON to stdin or pass a file path.")
        sys.exit(1)

    try:
        return json.loads(sys.stdin.read())
    except json.JSONDecodeError as exc:
        _warn(f"Invalid JSON on stdin: {exc}")
        sys.exit(1)


def main() -> int:
    """Generate prep for a single meeting."""
    workspace = resolve_workspace()
    meeting_raw = _read_meeting_input()

    config = load_config()
    user_domain = get_user_domain(config)

    _info(f"Preparing: {meeting_raw.get('title', meeting_raw.get('summary', '(untitled)'))}")

    # Classify the meeting
    classified = classify_meeting(meeting_raw, user_domain)
    meeting_type = classified.get("type", "unknown")
    meeting_id = make_meeting_id(meeting_raw, meeting_type)

    _info(f"  Type: {meeting_type}, ID: {meeting_id}")

    # Skip personal/all_hands — no prep needed
    if meeting_type in ("personal", "all_hands"):
        _info("  Skipped (no prep needed for this meeting type)")
        return 0

    # Gather rich context
    ctx = gather_meeting_context(classified, workspace)
    ctx_dict = ctx.to_dict()

    # Build single-meeting directive
    directive: dict[str, Any] = {
        "command": "meeting_prep",
        "meeting_id": meeting_id,
        "meeting": ctx_dict,
        "ai_tasks": [
            {
                "type": "generate_meeting_prep",
                "event_id": classified.get("id", ""),
                "meeting_type": meeting_type,
                "priority": "high" if meeting_type in ("customer", "qbr") else "medium",
            },
        ],
    }

    # Write to prep-directives/
    output_dir = workspace / "_today" / "data" / "prep-directives"
    output_path = output_dir / f"{meeting_id}.json"
    write_json(output_path, directive)

    _info(f"  Directive: {output_path}")

    refs_count = len(ctx_dict.get("refs", {}))
    captures_count = len(ctx_dict.get("recent_captures", []))
    actions_count = len(ctx_dict.get("open_actions", []))
    _info(f"  Refs: {refs_count}, Captures: {captures_count}, Actions: {actions_count}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
