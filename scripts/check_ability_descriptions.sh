#!/usr/bin/env bash
# Purpose: lint model- and user-facing ability descriptions for PII and internal vocabulary.
# Exit codes: 0 when clean; 1 when violations or required lint inputs are found invalid.
# How to run: ./scripts/check_ability_descriptions.sh

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
# Local-only PII blocklist (gitignored). Present on developer machines, absent
# in CI. Loaded when available for the broader real-PII coverage.
PII_BLOCKLIST_LOCAL="${ROOT_DIR}/.claude/pii-blocklist.txt"
# Committed, CI-safe PII denylist. Always available — provides the deterministic
# safety net so the lint gate never degrades to vocab-only when the local
# blocklist is absent.
PII_BLOCKLIST_COMMITTED="${ROOT_DIR}/scripts/ability_description_pii_denylist.txt"
VOCAB_BLOCKLIST="${ROOT_DIR}/scripts/ability_description_vocab_blocklist.txt"
SCAN_PATHS="${ABILITY_DESC_LINT_SCAN_PATHS:-}"

python3 - "$ROOT_DIR" "$PII_BLOCKLIST_LOCAL" "$PII_BLOCKLIST_COMMITTED" "$VOCAB_BLOCKLIST" "$SCAN_PATHS" <<'PY'
import ast
import json
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1]).resolve()
pii_blocklist_local = pathlib.Path(sys.argv[2])
pii_blocklist_committed = pathlib.Path(sys.argv[3])
vocab_blocklist = pathlib.Path(sys.argv[4])
scan_paths_override = sys.argv[5]


def die(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def load_terms(path, required):
    if not path.exists():
        if required:
            die(f"{path}: required blocklist is missing")
        return []

    terms = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        terms.append(line)
    return terms


def compile_term(term):
    parts = re.split(r"\s+", term.strip())
    body = r"\s+".join(re.escape(part) for part in parts)
    return re.compile(r"(?<![A-Za-z0-9_])" + body + r"(?![A-Za-z0-9_])", re.IGNORECASE)


def display_path(path, original):
    try:
        return str(path.resolve().relative_to(root))
    except ValueError:
        return original


def rust_string_value(literal):
    try:
        return ast.literal_eval(literal)
    except (SyntaxError, ValueError):
        return literal[1:-1]


def json_string_value(literal):
    try:
        return json.loads(literal)
    except json.JSONDecodeError:
        return literal[1:-1]


def display_description(description):
    compact = re.sub(r"\s+", " ", description).strip()
    return compact.replace("\\", "\\\\").replace('"', '\\"')


def display_term(term):
    return term.replace("\\", "\\\\").replace('"', '\\"')


raw_terms = (
    load_terms(pii_blocklist_local, required=False)
    + load_terms(pii_blocklist_committed, required=True)
    + load_terms(vocab_blocklist, required=True)
)
terms = []
seen_terms = set()
for term in raw_terms:
    key = term.casefold()
    if key in seen_terms:
        continue
    seen_terms.add(key)
    terms.append((term, compile_term(term)))


def findings_for_description(path_label, line_number, description):
    findings = []
    for term, pattern in terms:
        if pattern.search(description):
            findings.append(
                f'{path_label}:{line_number}: matched term "{display_term(term)}" '
                f'in description "{display_description(description)}"'
            )
    return findings


def find_matching_paren(text, open_index):
    depth = 0
    in_string = False
    escaped = False

    for index in range(open_index, len(text)):
        char = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue

        if char == '"':
            in_string = True
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return index

    return None


ABILITY_ATTR_RE = re.compile(r"#\s*\[\s*ability\s*\(")
RUST_DESCRIPTION_RE = re.compile(r'\bdescription\s*=\s*("(?:(?:\\.)|[^"\\])*")', re.DOTALL)


def scan_rust_file(path, path_label):
    text = path.read_text(encoding="utf-8")
    findings = []

    for attr_match in ABILITY_ATTR_RE.finditer(text):
        open_paren = attr_match.end() - 1
        close_paren = find_matching_paren(text, open_paren)
        if close_paren is None:
            continue

        attr_text = text[attr_match.start(): close_paren + 1]
        attr_offset = attr_match.start()
        for desc_match in RUST_DESCRIPTION_RE.finditer(attr_text):
            literal = desc_match.group(1)
            line_number = text.count("\n", 0, attr_offset + desc_match.start(1)) + 1
            description = rust_string_value(literal)
            findings.extend(findings_for_description(path_label, line_number, description))

    return findings


JSON_DESCRIPTION_RE = re.compile(r'"description"\s*:\s*("(?:(?:\\.)|[^"\\])*")', re.DOTALL)


def description_line_index(text):
    entries = []
    for match in JSON_DESCRIPTION_RE.finditer(text):
        literal = match.group(1)
        entries.append(
            {
                "description": json_string_value(literal),
                "line": text.count("\n", 0, match.start(1)) + 1,
                "used": False,
            }
        )
    return entries


def line_for_description(entries, description):
    for entry in entries:
        if not entry["used"] and entry["description"] == description:
            entry["used"] = True
            return entry["line"]
    return 1


def scan_json_file(path, path_label):
    text = path.read_text(encoding="utf-8")
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as error:
        die(f"{path_label}:{error.lineno}: invalid JSON: {error.msg}")

    # The MCP/inventory artifact at tools/dailyos-abilities.json uses the
    # `abilities` top-level key (mirrors AbilitySurfaceInventory shape).
    entries = payload.get("abilities", [])
    if not isinstance(entries, list):
        return []

    line_entries = description_line_index(text)
    findings = []
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        description = entry.get("description")
        if not isinstance(description, str):
            continue
        line_number = line_for_description(line_entries, description)
        findings.extend(findings_for_description(path_label, line_number, description))
    return findings


def default_scan_files():
    files = []
    for base in (
        root / "src-tauri/abilities-runtime/src",
        root / "src-tauri/src/abilities",
    ):
        if base.exists():
            files.extend(sorted(base.rglob("*.rs")))

    tools_json = root / "tools/dailyos-abilities.json"
    if tools_json.exists():
        files.append(tools_json)
    return [(path, display_path(path, str(path))) for path in files]


def override_scan_files():
    files = []
    for raw_path in scan_paths_override.split(":"):
        if not raw_path:
            continue
        path = pathlib.Path(raw_path)
        if not path.is_absolute():
            path = root / path
        if not path.exists():
            die(f"{raw_path}: scan path does not exist")
        files.append((path, display_path(path, raw_path)))
    return files


scan_files = override_scan_files() if scan_paths_override else default_scan_files()
all_findings = []
for path, path_label in scan_files:
    if path.suffix == ".json":
        all_findings.extend(scan_json_file(path, path_label))
    elif path.suffix == ".rs":
        all_findings.extend(scan_rust_file(path, path_label))

for finding in all_findings:
    print(finding)

sys.exit(1 if all_findings else 0)
PY
