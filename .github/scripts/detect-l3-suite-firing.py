#!/usr/bin/env python3
"""Detect which L3 suites (S, P, E) should fire for a given integrated diff.

Engineering-ladder.md (2026-05-23 revision) moves Suite S/P/E firing from
"all three always at L3" to "fire by need":
  - Suite S fires when the diff touches any path in matrix.yml's
    `security-auditor.when_changed` (Amendment 3 paths)
  - Suite P fires when the diff touches any path in matrix.yml's
    `performance-engineer.when_changed`
  - Suite E always fires (cheap; catches the long tail)

This script reads the matrix from a path passed as argv[1], reads the changed
files from stdin (one per line), and prints GitHub Actions outputs:

    needs-s=true|false
    needs-p=true|false

Usage (in CI):
    git diff --name-only "$BASE_SHA..$HEAD_SHA" \\
      | python3 .github/scripts/detect-l3-suite-firing.py \\
          .github/reviewer-prompts/matrix.yml \\
          >> "$GITHUB_OUTPUT"
"""
from __future__ import annotations

import os
import re
import sys
from pathlib import Path

import yaml


def glob_to_regex(pattern: str) -> re.Pattern[str]:
    """Mirror of the glob compiler in validate-pr-template.py for parity."""
    tokens: list[str] = []
    i = 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            tokens.append(r"(?:.*/)?")
            i += 3
        elif pattern.startswith("**", i):
            tokens.append(r".*")
            i += 2
        elif pattern[i] == "*":
            tokens.append(r"[^/]*")
            i += 1
        elif pattern[i] == "?":
            tokens.append(r"[^/]")
            i += 1
        else:
            tokens.append(re.escape(pattern[i]))
            i += 1
    return re.compile("^" + "".join(tokens) + "$")


def matches_any(path: str, patterns: list[re.Pattern[str]]) -> bool:
    return any(p.match(path) for p in patterns)


def load_when_changed(matrix_path: Path, reviewer: str) -> list[re.Pattern[str]]:
    """Load `when_changed` glob list for `reviewer` and compile to regex.

    Fails closed: if the reviewer entry is missing or its `when_changed` list
    is empty, raises ValueError. This gate is load-bearing for the L3 release
    review — a corrupted or trimmed matrix.yml must not silently produce
    `needs-{s,p}=false` for every diff.
    """
    with matrix_path.open() as fh:
        data = yaml.safe_load(fh)
    for entry in data.get("reviewers", []):
        if entry.get("reviewer") == reviewer:
            globs = entry.get("when_changed", []) or []
            if not globs:
                raise ValueError(
                    f"matrix.yml reviewer '{reviewer}' has empty `when_changed` — "
                    f"L3 suite firing cannot be computed safely. Refusing to fail open."
                )
            return [glob_to_regex(g) for g in globs]
    raise ValueError(
        f"matrix.yml has no reviewer entry named '{reviewer}' — "
        f"L3 suite firing cannot be computed safely. Refusing to fail open."
    )


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: detect-l3-suite-firing.py <matrix.yml>", file=sys.stderr)
        return 2

    matrix_path = Path(argv[1])
    if not matrix_path.exists():
        print(f"matrix not found: {matrix_path}", file=sys.stderr)
        return 2

    s_patterns = load_when_changed(matrix_path, "security-auditor")
    p_patterns = load_when_changed(matrix_path, "performance-engineer")

    changed = [line.strip() for line in sys.stdin if line.strip()]

    needs_s = any(matches_any(path, s_patterns) for path in changed)
    needs_p = any(matches_any(path, p_patterns) for path in changed)

    # Write to GITHUB_OUTPUT if available; otherwise stdout for local testing.
    out_lines = [
        f"needs-s={'true' if needs_s else 'false'}",
        f"needs-p={'true' if needs_p else 'false'}",
    ]
    gh_output = os.environ.get("GITHUB_OUTPUT")
    if gh_output:
        with open(gh_output, "a") as fh:
            for line in out_lines:
                fh.write(line + "\n")
    for line in out_lines:
        print(line)

    # Diagnostic to stderr.
    print(
        f"detect-l3-suite-firing: {len(changed)} changed paths; "
        f"Suite S needed={needs_s}; Suite P needed={needs_p}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
