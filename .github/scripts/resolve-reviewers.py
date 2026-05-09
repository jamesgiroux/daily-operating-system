#!/usr/bin/env python3
"""Resolve which L2 reviewer prompts to invoke for a given PR.

Reads the changed-files list from stdin (one path per line), matches against
matrix.yml, and prints a comma-separated list of reviewer names to stdout.

`matrix.yml` is read from the path passed as the first argument — the workflow
passes the BASE-ref version (not the PR head) to prevent self-modifying PRs.

Always-include reviewers (declared with `always: true` in matrix.yml) are
emitted regardless of changed-files matches.

Usage (in CI):
    git show "origin/${BASE_BRANCH}:.github/reviewer-prompts/matrix.yml" > /tmp/matrix.yml
    gh api --paginate "repos/${REPO}/pulls/${PR}/files" --jq '.[].filename' \\
      | python3 .github/scripts/resolve-reviewers.py /tmp/matrix.yml
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml


# Hard allowlist of valid reviewer names. The matrix-resolver MUST NOT emit
# names outside this set, even if the matrix.yml has been edited to include
# others — defense-in-depth against matrix tampering even after path-fence.
ALLOWED_REVIEWERS = {
    "code-reviewer",
    "architect-reviewer",
    "security-auditor",
    "performance-engineer",
    "accessibility-tester",
}


def glob_to_regex(pattern: str) -> re.Pattern[str]:
    """Convert a glob with `**` recursive support to a compiled regex.

    `**/` matches zero or more path components.
    `**` alone matches any sequence (including `/`).
    `*` matches any sequence excluding `/`.
    `?` matches one char excluding `/`.
    """
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


def resolve(changed: list[str], matrix: dict) -> list[str]:
    """Return ordered, deduplicated reviewer names for the given changed files."""
    picked: list[str] = []

    for entry in matrix.get("reviewers", []):
        name = entry.get("reviewer")
        if not name:
            continue
        if name not in ALLOWED_REVIEWERS:
            # Matrix tampering defense — silently skip unknown reviewer names.
            print(f"warning: matrix entry '{name}' not in ALLOWED_REVIEWERS; skipping",
                  file=sys.stderr)
            continue

        if entry.get("always") is True:
            picked.append(name)
            continue

        for g in entry.get("when_changed", []):
            rx = glob_to_regex(g)
            if any(rx.match(c) for c in changed):
                picked.append(name)
                break

    # Dedupe while preserving order.
    seen: set[str] = set()
    unique: list[str] = []
    for r in picked:
        if r not in seen:
            seen.add(r)
            unique.append(r)
    return unique


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: resolve-reviewers.py <matrix.yml-path>", file=sys.stderr)
        return 2

    matrix_path = Path(argv[1])
    if not matrix_path.is_file():
        print(f"error: matrix file not found: {matrix_path}", file=sys.stderr)
        return 2

    try:
        matrix = yaml.safe_load(matrix_path.read_text())
    except yaml.YAMLError as e:
        print(f"error: matrix.yml parse failed: {e}", file=sys.stderr)
        return 2

    if not isinstance(matrix, dict) or not isinstance(matrix.get("reviewers"), list):
        print("error: matrix.yml has no 'reviewers' list", file=sys.stderr)
        return 2

    changed = [line.strip() for line in sys.stdin if line.strip()]
    if not changed:
        print("warning: no changed files on stdin; emitting always-true reviewers only",
              file=sys.stderr)

    reviewers = resolve(changed, matrix)
    if not reviewers:
        # Fail-closed: empty reviewer list is suspicious. The matrix should
        # have at least one always-true reviewer (code-reviewer + architect).
        print("error: resolver returned empty reviewer list", file=sys.stderr)
        return 1

    print(",".join(reviewers))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
