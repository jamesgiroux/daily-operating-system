#!/usr/bin/env python3
"""Validate the PR body's `security_auditor_invoked` field against the changed
files, using `matrix.yml`'s `security-auditor.when_changed` as the single
source of truth for trigger paths.

This replaces validate-pr-template.sh — the bash version maintained its own
trigger-path list which drifted from matrix.yml. Single source of truth here.

Usage (in CI):
    git show "origin/${BASE_BRANCH}:.github/reviewer-prompts/matrix.yml" > /tmp/matrix.yml
    gh api --paginate "repos/${REPO}/pulls/${PR}/files" --jq '.[].filename' > /tmp/changed.txt
    python3 .github/scripts/validate-pr-template.py /tmp/pr-body.txt /tmp/matrix.yml < /tmp/changed.txt

Behavior:
    - If `security_auditor_invoked: true` is present (top-level, not in fenced
      code block) → pass, exit 0.
    - If `security_auditor_invoked: false` AND no security-trigger paths match
      against the changed-files list → pass, exit 0.
    - If `security_auditor_invoked: false` AND any security-trigger path matches
      → fail, exit 1, with explicit message (path-prefix overrides claim per
      Amendment 3).
    - If field is missing, parseable as neither true nor false, or appears
      multiple times at top-level → fail, exit 1.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml


def strip_fenced_blocks(body: str) -> str:
    """Remove triple-backtick fenced code blocks from the body.

    Used to ensure we don't false-positive on fenced examples that mention
    the field. Matches both ```lang\n...\n``` and bare ```\n...\n```.
    """
    out = []
    in_fence = False
    for line in body.splitlines():
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if not in_fence:
            out.append(line)
    return "\n".join(out)


def extract_field(body: str) -> str | None:
    """Extract `security_auditor_invoked: true|false` value from cleaned body.

    Returns the lowercased value (`"true"` or `"false"`), or None if missing
    or malformed. Fails (returns sentinel) if multiple matches at top-level.
    """
    cleaned = strip_fenced_blocks(body)

    # Match the field with optional backticks: `security_auditor_invoked` or plain.
    pattern = re.compile(
        r"`?security_auditor_invoked`?\s*:\s*(true|false)\b",
        re.IGNORECASE,
    )
    matches = pattern.findall(cleaned)
    if not matches:
        return None
    if len(matches) > 1 and len(set(m.lower() for m in matches)) > 1:
        # Multiple distinct values mentioned at top-level — author confused
        # or attempting to bypass.
        return "ambiguous"
    return matches[0].lower()


def glob_to_regex(pattern: str) -> re.Pattern[str]:
    """Convert a glob with `**` recursive support to a compiled regex.

    Mirrors resolve-reviewers.py's glob_to_regex so the matcher behavior is
    consistent across the gate.
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


def load_security_triggers(matrix_path: Path) -> list[str]:
    """Read matrix.yml and return the security-auditor entry's `when_changed`."""
    matrix = yaml.safe_load(matrix_path.read_text())
    for entry in matrix.get("reviewers", []):
        if entry.get("reviewer") == "security-auditor":
            return entry.get("when_changed", []) or []
    return []


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: validate-pr-template.py <pr-body-file> <matrix.yml-path>",
              file=sys.stderr)
        return 2

    body_path = Path(argv[1])
    matrix_path = Path(argv[2])

    if not body_path.is_file():
        print(f"error: pr-body file not found: {body_path}", file=sys.stderr)
        return 2
    if not matrix_path.is_file():
        print(f"error: matrix file not found: {matrix_path}", file=sys.stderr)
        return 2

    body = body_path.read_text()
    field = extract_field(body)

    if field is None:
        print(
            "🔒 PR-template validation: `security_auditor_invoked` field missing.\n"
            "\n"
            "The §4 Security section of the PR template requires:\n"
            "    security_auditor_invoked: true | false\n"
            "\n"
            "If false, cite an exemption ID in the rationale slot.\n"
            "Add the field to the PR body and sync the PR.",
            file=sys.stderr,
        )
        return 1

    if field == "ambiguous":
        print(
            "🔒 PR-template validation: `security_auditor_invoked` appears with\n"
            "multiple distinct values in the PR body. Use one value (true OR false).",
            file=sys.stderr,
        )
        return 1

    if field == "true":
        print("validate-pr-template: security_auditor_invoked=true; OK.")
        return 0

    # field == "false" — check if any trigger paths match the changed files.
    changed = [line.strip() for line in sys.stdin if line.strip()]
    triggers = load_security_triggers(matrix_path)
    if not triggers:
        print(
            "warning: matrix.yml's security-auditor entry has no when_changed list",
            file=sys.stderr,
        )
        return 0  # Nothing to enforce against; trust the author's claim.

    compiled = [glob_to_regex(t) for t in triggers]
    triggered: list[str] = []
    for path in changed:
        for rx in compiled:
            if rx.match(path):
                triggered.append(path)
                break

    if not triggered:
        print("validate-pr-template: security_auditor_invoked=false and no trigger paths matched; OK.")
        return 0

    print(
        "🔒 PR-template validation FAILED.\n"
        "\n"
        "You set `security_auditor_invoked: false`, but the PR touches paths\n"
        "that mandate security review per Amendment 3:\n"
        "\n"
        + "\n".join(f"   - {p}" for p in triggered) + "\n"
        "\n"
        "Either set `security_auditor_invoked: true` (recommended), or — if you\n"
        "genuinely believe an exemption applies — escalate L6 with rationale.\n"
        "\n"
        "Path-prefix detection overrides the author's claim. By design.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
