#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

python3 - <<'PY'
from __future__ import annotations

from pathlib import Path
import re

ROOT = Path.cwd()
SERVICE_ROOT = ROOT / "src-tauri" / "src" / "services"

KIND_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    (
        "D",
        re.compile(
            r"(?:(?:\b(?:db|tx)\.|\bcrate::services::[A-Za-z0-9_:]+::|^\s*\.)(?:"
            r"accept_|add_|append_|archive|bulk_create|clear_|complete_|confirm_|"
            r"correct_|create|delete|dismiss_|ensure_|insert|link_|mark_|merge_|"
            r"persist_|enqueue_|queue_|record_|reject_|remove_|reopen_|replace_|reset_|"
            r"resolve_|restore_|retry_|save_|set_|snooze_|submit_|sync_|toggle_pin|"
            r"tombstone_|touch_|unlink_|unarchive|unsuppress|update|upsert|write_"
            r")[A-Za-z0-9_]*\s*\(|"
            r"\b(?:accept_|add_|append_|archive|bulk_create|clear_|complete_|confirm_|"
            r"correct_|create|delete|dismiss_|ensure_|insert|link_|mark_|merge_|"
            r"persist_|enqueue_|queue_|record_|reject_|remove_|reopen_|replace_|reset_|"
            r"resolve_|restore_|retry_|save_|set_|snooze_|submit_|sync_|toggle_pin|"
            r"tombstone_|touch_|unlink_|unarchive|unsuppress|update|upsert|write_"
            r")[A-Za-z0-9_]*\s*\(\s*(?:db|tx)\b|"
            r"\b(?:submit_[A-Za-z0-9_]*|upsert_report|save_report_content|"
            r"write_audit_entry|create_or_update_config)\s*\()"
        ),
    ),
    ("SQL", re.compile(r"\.(?:execute|execute_batch)\s*\(")),
    ("TX", re.compile(r"\.(?:transaction|with_transaction)\s*\(")),
    (
        "SIG",
        re.compile(
            r"(?:emit_signal|emit_and_propagate|emit_propagate_and_evaluate|"
            r"emit_signal_and_propagate|emit_signal_propagate_and_evaluate|\.emit\s*\()"
        ),
    ),
    (
        "FS",
        re.compile(
            r"(?:std::fs::|fs::)(?:write|create_dir_all|remove_file|remove_dir_all|"
            r"remove_dir|rename|set_permissions)\s*\("
        ),
    ),
    (
        "BG",
        re.compile(
            r"(?:\benqueue_[A-Za-z0-9_]*\s*\(|\.enqueue\s*\(|remove_by_entity_id\s*\(|"
            r"invalidate_and_requeue|schedule_recompute\s*\(|notify_one\s*\()"
        ),
    ),
    (
        "EXT",
        re.compile(
            r"(?:crate::google_api|google_api::|\bgmail::|reqwest::Client|"
            r"\.post\s*\(|\.send\s*\(\)\.await|run_report_generation|"
            r"run_parallel_swot_generation|prefetch_glean|run_bob_generation|"
            r"state\.integrations\.linear|\bLinear[A-Za-z0-9_]*\b|"
            r"\bslack\b|\bsalesforce\b|\bglean\b|\bpty\b)"
        ),
    ),
    ("C", re.compile(r"(?:\bUtc::now\s*\(|chrono::Utc::now\s*\(|rand::thread_rng\s*\(|thread_rng\s*\(|rand::rng\s*\()")),
]

FUNCTION_RE = re.compile(
    r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:async[ \t]+)?fn[ \t]+"
    r"([A-Za-z_][A-Za-z0-9_]*)[ \t]*(?:<[^>{};]*>)?[ \t]*\("
)


def line_starts(text: str) -> list[int]:
    starts = [0]
    for match in re.finditer(r"\n", text):
        starts.append(match.end())
    return starts


def line_for_index(starts: list[int], index: int) -> int:
    lo, hi = 0, len(starts)
    while lo + 1 < hi:
        mid = (lo + hi) // 2
        if starts[mid] <= index:
            lo = mid
        else:
            hi = mid
    return lo + 1


def first_code_brace(text: str, start: int) -> int:
    semicolon = text.find(";", start)
    brace = text.find("{", start)
    if brace == -1 or (semicolon != -1 and semicolon < brace):
        return -1
    return brace


def matching_brace(text: str, open_index: int) -> int:
    depth = 0
    i = open_index
    state = "code"
    raw_hashes = 0
    block_depth = 0
    n = len(text)
    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""

        if state == "line_comment":
            if ch == "\n":
                state = "code"
            i += 1
            continue

        if state == "block_comment":
            if ch == "/" and nxt == "*":
                block_depth += 1
                i += 2
                continue
            if ch == "*" and nxt == "/":
                block_depth -= 1
                i += 2
                if block_depth == 0:
                    state = "code"
                continue
            i += 1
            continue

        if state == "string":
            if ch == "\\":
                i += 2
                continue
            if ch == '"':
                state = "code"
            i += 1
            continue

        if state == "char":
            if ch == "\\":
                i += 2
                continue
            if ch == "'":
                state = "code"
            i += 1
            continue

        if state == "raw_string":
            if ch == '"' and text.startswith("#" * raw_hashes, i + 1):
                i += 1 + raw_hashes
                state = "code"
            i += 1
            continue

        if ch == "/" and nxt == "/":
            state = "line_comment"
            i += 2
            continue
        if ch == "/" and nxt == "*":
            state = "block_comment"
            block_depth = 1
            i += 2
            continue
        if ch == "r":
            raw = re.match(r'r(#+)"', text[i:])
            if raw:
                raw_hashes = len(raw.group(1))
                state = "raw_string"
                i += 2 + raw_hashes
                continue
            if nxt == '"':
                raw_hashes = 0
                state = "raw_string"
                i += 2
                continue
        if ch == '"':
            state = "string"
            i += 1
            continue
        if ch == "'":
            state = "char"
            i += 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise RuntimeError("unbalanced braces")


def cfg_test_spans(text: str) -> list[tuple[int, int]]:
    spans: list[tuple[int, int]] = []
    for cfg in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", text):
        mod = re.match(r"\s*mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{", text[cfg.end() :])
        if not mod:
            continue
        open_index = cfg.end() + mod.end() - 1
        try:
            close_index = matching_brace(text, open_index)
        except RuntimeError:
            continue
        spans.append((cfg.start(), close_index + 1))
    return spans


def in_any_span(index: int, spans: list[tuple[int, int]]) -> bool:
    return any(start <= index < end for start, end in spans)


def module_for(rel: Path) -> str:
    parts = list(rel.with_suffix("").parts)
    if parts and parts[-1] == "mod":
        parts = parts[:-1]
    return "::".join(parts)


def classify(body: str) -> tuple[list[str], list[str]]:
    kinds: list[str] = []
    evidence: list[str] = []
    body_lines = body.splitlines()
    for kind, pattern in KIND_PATTERNS:
        first_match = None
        for offset, line in enumerate(body_lines, start=1):
            if line.strip().startswith("//"):
                continue
            if pattern.search(line):
                first_match = (offset, line.strip())
                break
        if first_match:
            kinds.append(kind)
            evidence.append(f"{kind}@+{first_match[0]}:{first_match[1]}")
    return kinds, evidence


rows: list[tuple[str, int, str, str, list[str]]] = []

for path in sorted(SERVICE_ROOT.rglob("*.rs")):
    rel = path.relative_to(SERVICE_ROOT)
    text = path.read_text()
    starts = line_starts(text)
    test_spans = cfg_test_spans(text)
    module = module_for(rel)

    for match in FUNCTION_RE.finditer(text):
        if in_any_span(match.start(), test_spans):
            continue
        prefix = text[max(0, match.start() - 160) : match.start()]
        if "#[test]" in prefix or "#[tokio::test]" in prefix:
            continue
        open_index = first_code_brace(text, match.end())
        if open_index == -1:
            continue
        try:
            close_index = matching_brace(text, open_index)
        except RuntimeError:
            continue
        body = text[open_index : close_index + 1]
        kinds, evidence = classify(body)
        if not kinds:
            continue
        if all(kind == "C" for kind in kinds):
            continue
        function_line = line_for_index(starts, match.start())
        function_name = match.group(1)
        symbol = f"{module}::{function_name}:{function_line}"
        file_ref = f"src-tauri/src/services/{rel.as_posix()}:{function_line}"

        absolute_evidence: list[str] = []
        for item in evidence:
            kind, rest = item.split("@+", 1)
            offset, snippet = rest.split(":", 1)
            evidence_line = function_line + int(offset) - 1
            absolute_evidence.append(
                f"{kind}=src-tauri/src/services/{rel.as_posix()}:{evidence_line}:{snippet}"
            )

        rows.append((rel.as_posix(), function_line, symbol, "+".join(kinds), absolute_evidence))

print("# DOS-209 mutation audit")
print("# Generated by scripts/dos209-mutation-audit.sh")
print("# Source root: src-tauri/src/services")
print("# Method: Rust fn brace scanner plus deterministic mutation regexes; #[cfg(test)] modules excluded.")
print("# Columns: symbol | kinds | first matching evidence per kind")
for _, _, symbol, kinds, evidence in rows:
    print(f"{symbol} | {kinds} | {' ; '.join(evidence)}")
PY
