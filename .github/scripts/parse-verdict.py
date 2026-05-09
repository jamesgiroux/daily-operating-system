#!/usr/bin/env python3
"""Parse a reviewer's PR comment for the verdict line.

Reads the comment body from stdin. Looks for verdict tokens at top level (after
stripping fenced code blocks) in the first or last 25 lines. Exits 0 on
`approve`, 1 otherwise.

Multiple distinct normalized verdicts (whether across or within the first/last
buckets) = ambiguous = fail-closed.

Replaces the prior bash version: cleaner logic, deterministic Python regex,
no set -e / pipefail interaction with grep no-match.
"""
from __future__ import annotations

import re
import sys
import unicodedata


# `**Verdict:** <token>` or `### **Verdict:** <token>` etc.
# Allow optional heading hashes, asterisks, whitespace; capture the token rest.
VERDICT_PATTERN = re.compile(
    r"^[#*\s]*verdict[#*\s]*:[#*\s]*(.+?)[#*\s]*$",
    re.IGNORECASE,
)

SYNONYM_MAP = {
    "approved": "approve",
    "lgtm": "approve",
    "request changes": "changes-requested",
    "needs changes": "changes-requested",
}

VALID_VERDICTS = {"approve", "changes-requested", "reject"}

# Claude occasionally copies the format-example token verbatim; treat as
# non-substantive (per Amendment 2's substantive-output rules).
NON_SUBSTANTIVE_LITERALS = {
    "approve | changes-requested | reject",
    "approve|changes-requested|reject",
}


def strip_ansi(s: str) -> str:
    """Strip CSI/SGR, OSC (BEL- or ST-terminated), DCS, and charset-set escapes."""
    s = re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", s)
    s = re.sub(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)", "", s)
    s = re.sub(r"\x1bP[^\x1b]*\x1b\\", "", s)
    s = re.sub(r"\x1b[()][AB012]", "", s)
    return s


def strip_fenced(body: str) -> str:
    """Remove triple-backtick fenced code blocks (lines between ```...```)."""
    out: list[str] = []
    in_fence = False
    for line in body.splitlines():
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if not in_fence:
            out.append(line)
    return "\n".join(out)


def normalize(token: str) -> str:
    """Normalize: drop too-long lines, ANSI strip, NFKC, lowercase, trim, synonym map."""
    if len(token) > 1024:
        return ""
    token = strip_ansi(token)
    token = unicodedata.normalize("NFKC", token)
    token = token.lower().strip()
    return SYNONYM_MAP.get(token, token)


def find_candidates(lines: list[str]) -> list[str]:
    """Return raw verdict tokens from the given lines."""
    out: list[str] = []
    for ln in lines:
        m = VERDICT_PATTERN.match(ln)
        if m:
            out.append(m.group(1))
    return out


# Reviewer prompts emit findings as:
#   - **[severity] [category] — [title]**
# where severity ∈ {critical, high, medium, low}.
# Blocking severities mean the verdict cannot be `approve`.
BLOCKING_SEVERITIES = {"critical", "high", "medium"}
FINDING_PATTERN = re.compile(
    r"\[(critical|high|medium|low)\]",
    re.IGNORECASE,
)


def find_blocking_findings(body: str) -> list[tuple[str, str]]:
    """Return [(severity, line)] for any blocking-severity findings in body.

    Operates on the un-stripped body to catch findings inside structured
    sections (which markdown editors sometimes wrap in fences). Skips lines
    starting with verdict markers (those are part of the verdict itself).
    """
    blocking: list[tuple[str, str]] = []
    for line in body.splitlines():
        if VERDICT_PATTERN.match(line):
            continue
        m = FINDING_PATTERN.search(line)
        if m and m.group(1).lower() in BLOCKING_SEVERITIES:
            blocking.append((m.group(1).lower(), line.strip()))
    return blocking


def has_any_finding(body: str) -> bool:
    """True if the body contains at least one structured finding (any severity)."""
    for line in body.splitlines():
        if VERDICT_PATTERN.match(line):
            continue
        if FINDING_PATTERN.search(line):
            return True
    return False


# Reviewer prompts emit an explicit attestation when there are no findings,
# typically: "No findings. Diff is clean against the L2 <role> dimensions."
NO_FINDINGS_PATTERN = re.compile(
    r"^\s*no\s+findings\b",
    re.IGNORECASE | re.MULTILINE,
)


def has_no_findings_attestation(body: str) -> bool:
    """True if the body contains an explicit 'No findings' attestation."""
    cleaned = strip_fenced(body)
    return bool(NO_FINDINGS_PATTERN.search(cleaned))


def main() -> int:
    body = sys.stdin.read()
    cleaned = strip_fenced(body)
    lines = cleaned.splitlines()

    if not lines:
        print("parse-verdict: empty body", file=sys.stderr)
        return 1

    first_25 = lines[:25]
    last_25 = lines[-25:]

    candidates = find_candidates(first_25) + find_candidates(last_25)
    if not candidates:
        print("parse-verdict: no `Verdict:` line in first or last 25 lines (fenced blocks stripped)",
              file=sys.stderr)
        return 1

    normalized = {n for n in (normalize(c) for c in candidates) if n}
    if not normalized:
        print("parse-verdict: all verdict tokens normalized to empty (lines too long?)",
              file=sys.stderr)
        return 1
    if len(normalized) > 1:
        print(f"parse-verdict: ambiguous — multiple distinct verdicts: {sorted(normalized)}",
              file=sys.stderr)
        return 1

    verdict = next(iter(normalized))

    if verdict in NON_SUBSTANTIVE_LITERALS:
        print(f"parse-verdict: literal example token '{verdict}' is non-substantive",
              file=sys.stderr)
        return 1

    if verdict == "approve":
        # Substantive-output check (Amendment 2):
        #   1. Verdict must not be inconsistent with body — approve cannot
        #      coexist with medium/high/critical findings.
        #   2. Body must contain at least one structured finding OR an explicit
        #      "No findings" attestation. A bare `Verdict: approve` is
        #      non-substantive and fails closed.
        blocking = find_blocking_findings(body)
        if blocking:
            print(
                "parse-verdict: verdict is `approve` but body contains blocking findings",
                file=sys.stderr,
            )
            for sev, snippet in blocking[:5]:
                print(f"  - [{sev}] {snippet[:100]}", file=sys.stderr)
            print(
                "  → Reviewer must either resolve the findings OR change verdict to "
                "`changes-requested` / `reject`.",
                file=sys.stderr,
            )
            return 1

        if not has_any_finding(body) and not has_no_findings_attestation(body):
            print(
                "parse-verdict: verdict is `approve` but body has no structured findings",
                file=sys.stderr,
            )
            print(
                "  and no explicit 'No findings' attestation. Per Amendment 2's substantive-",
                file=sys.stderr,
            )
            print(
                "  output rule, the comment must contain at least one of:",
                file=sys.stderr,
            )
            print(
                "    - One or more structured findings: `- **[severity] [category] — [title]**`",
                file=sys.stderr,
            )
            print(
                "    - An explicit attestation: `No findings. Diff is clean against ...`",
                file=sys.stderr,
            )
            return 1

        print("approve")
        return 0
    if verdict in VALID_VERDICTS:
        # changes-requested or reject
        print(verdict)
        return 1
    print(f"parse-verdict: unknown / qualified verdict '{verdict}'", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
