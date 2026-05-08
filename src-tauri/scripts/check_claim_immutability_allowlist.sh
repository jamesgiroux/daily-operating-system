#!/usr/bin/env bash
#
# Claims-substrate lint: intelligence_claims UPDATE statements may mutate only
# the DOS-7 amendment D lifecycle/trust/threading allowlist owned by
# services/claims.rs. The scanner parses full multi-line SET clauses instead
# of grepping a bounded line window, so forbidden columns are caught in any SET
# position and with SQLite quoted identifier forms.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
allowlist_source="$script_dir/../src/services/claims.rs"

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

python3 - "$allowlist_source" "${roots[@]}" <<'PY'
import pathlib
import re
import sys

allowlist_source = pathlib.Path(sys.argv[1])
roots = [pathlib.Path(arg) for arg in sys.argv[2:]]

source = allowlist_source.read_text(encoding="utf-8")
match = re.search(
    r"const\s+CLAIM_UPDATE_ALLOWED_COLUMNS\s*:\s*&\[\s*&str\s*\]\s*=\s*&\[(.*?)\];",
    source,
    re.S,
)
if not match:
    print(f"Claim immutability lint: missing CLAIM_UPDATE_ALLOWED_COLUMNS in {allowlist_source}", file=sys.stderr)
    sys.exit(2)

ALLOWED = set(re.findall(r'"([a-z0-9_]+)"', match.group(1)))
if not ALLOWED:
    print(f"Claim immutability lint: empty CLAIM_UPDATE_ALLOWED_COLUMNS in {allowlist_source}", file=sys.stderr)
    sys.exit(2)


def is_ident_continue(ch: str) -> bool:
    return ch == "_" or ch.isalnum()


def is_boundary(text: str, idx: int) -> bool:
    return idx < 0 or idx >= len(text) or not is_ident_continue(text[idx])


def keyword_at(text: str, idx: int, keyword: str) -> bool:
    lower = text.lower()
    end = idx + len(keyword)
    return lower.startswith(keyword, idx) and is_boundary(text, idx - 1) and is_boundary(text, end)


def skip_ws(text: str, idx: int) -> int:
    while idx < len(text) and (text[idx].isspace() or text[idx] == "\\"):
        idx += 1
    return idx


def parse_identifier(text: str, idx: int):
    idx = skip_ws(text, idx)
    if idx >= len(text):
        return None, idx

    ch = text[idx]
    if ch in ('"', "`", "'"):
        return parse_quoted_identifier(text, idx, ch)

    if ch == "[":
        return parse_quoted_identifier(text, idx, "]")

    if ch == "_" or ch.isalpha():
        end = idx + 1
        while end < len(text) and is_ident_continue(text[end]):
            end += 1
        return text[idx:end].lower(), end

    return None, idx


def parse_quoted_identifier(text: str, idx: int, close: str):
    ident = []
    cursor = idx + 1
    while cursor < len(text):
        ch = text[cursor]
        cursor += 1
        if ch == close:
            if cursor < len(text) and text[cursor] == close:
                ident.append(ch)
                cursor += 1
                continue
            return "".join(ident).lower(), cursor
        ident.append(ch)
    return None, idx


def find_keyword(text: str, keyword: str, start: int):
    lower = text.lower()
    idx = lower.find(keyword, start)
    while idx != -1:
        if keyword_at(text, idx, keyword):
            return idx
        idx = lower.find(keyword, idx + len(keyword))
    return None


def top_level_clause_starts(text: str, idx: int) -> bool:
    return any(keyword_at(text, idx, kw) for kw in ("where", "returning", "order", "limit"))


def skip_expression(text: str, idx: int) -> int:
    depth = 0
    quote = None
    while idx < len(text):
        if quote is None and depth == 0 and top_level_clause_starts(text, idx):
            return idx

        ch = text[idx]
        if quote is not None:
            idx += 1
            if ch == quote:
                if quote == "'" and idx < len(text) and text[idx] == "'":
                    idx += 1
                else:
                    quote = None
            continue

        if ch in ("'", '"', "`"):
            quote = ch
            idx += 1
        elif ch == "[":
            quote = "]"
            idx += 1
        elif ch == "(":
            depth += 1
            idx += 1
        elif ch == ")":
            depth = max(0, depth - 1)
            idx += 1
        elif ch == "," and depth == 0:
            return idx + 1
        else:
            idx += 1
    return idx


def parse_assignment_target(text: str, idx: int):
    column, next_idx = parse_identifier(text, idx)
    if column is None:
        return None, idx

    next_idx = skip_ws(text, next_idx)
    if next_idx < len(text) and text[next_idx] == ".":
        qualified, qualified_next = parse_identifier(text, next_idx + 1)
        if qualified is not None:
            column = qualified
            next_idx = skip_ws(text, qualified_next)

    return column, next_idx


def parse_row_value_targets(text: str, idx: int):
    idx = skip_ws(text, idx)
    if idx >= len(text) or text[idx] != "(":
        return None

    cursor = idx + 1
    columns = []
    while cursor < len(text):
        cursor = skip_ws(text, cursor)
        if cursor < len(text) and text[cursor] == ")":
            if not columns:
                return None
            cursor += 1
            break

        column, next_idx = parse_assignment_target(text, cursor)
        if column is None:
            return None
        columns.append(column)

        cursor = skip_ws(text, next_idx)
        if cursor < len(text) and text[cursor] == ",":
            cursor += 1
            continue
        if cursor < len(text) and text[cursor] == ")":
            cursor += 1
            break
        return None

    cursor = skip_ws(text, cursor)
    if cursor < len(text) and text[cursor] == "=":
        return columns, cursor + 1
    return None


def parse_set_columns(text: str, idx: int):
    columns = []
    while idx < len(text):
        idx = skip_ws(text, idx)
        if idx >= len(text) or top_level_clause_starts(text, idx):
            break

        row_value = parse_row_value_targets(text, idx)
        if row_value is not None:
            row_columns, value_idx = row_value
            columns.extend(row_columns)
            idx = skip_expression(text, value_idx)
            continue

        column, next_idx = parse_assignment_target(text, idx)
        if column is None:
            idx += 1
            continue

        if next_idx >= len(text) or text[next_idx] != "=":
            idx = next_idx + 1
            continue

        columns.append(column)
        idx = skip_expression(text, next_idx + 1)

    return columns


def parse_update_columns(text: str, start: int):
    idx = skip_ws(text, start + len("update"))
    if keyword_at(text, idx, "or"):
        _, idx = parse_identifier(text, skip_ws(text, idx + len("or")))
        idx = skip_ws(text, idx)

    first, idx = parse_identifier(text, idx)
    if first is None:
        return []

    idx = skip_ws(text, idx)
    table = first
    if idx < len(text) and text[idx] == ".":
        second, second_idx = parse_identifier(text, idx + 1)
        if second is not None:
            table = second
            idx = second_idx

    if table != "intelligence_claims":
        return []

    set_idx = find_keyword(text, "set", idx)
    if set_idx is None:
        return []
    return parse_set_columns(text, set_idx + len("set"))


def strip_rust_comments(text: str) -> str:
    out = []
    i = 0
    n = len(text)
    state = "code"
    raw_end = None
    block_depth = 0

    while i < n:
        if state == "code":
            if text.startswith("//", i):
                newline = text.find("\n", i)
                if newline == -1:
                    break
                out.append("\n")
                i = newline + 1
            elif text.startswith("/*", i):
                state = "block_comment"
                block_depth = 1
                out.append("  ")
                i += 2
            elif text[i] == "r":
                j = i + 1
                while j < n and text[j] == "#":
                    j += 1
                if j < n and text[j] == '"':
                    hashes = text[i + 1:j]
                    raw_end = '"' + hashes
                    out.append(text[i:j + 1])
                    i = j + 1
                    state = "raw_string"
                else:
                    out.append(text[i])
                    i += 1
            elif text[i] == '"':
                state = "string"
                out.append(text[i])
                i += 1
            elif text[i] == "'":
                state = "char"
                out.append(text[i])
                i += 1
            else:
                out.append(text[i])
                i += 1
        elif state == "string":
            out.append(text[i])
            if text[i] == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
            elif text[i] == '"':
                i += 1
                state = "code"
            else:
                i += 1
        elif state == "raw_string":
            if raw_end is not None and text.startswith(raw_end, i):
                out.append(raw_end)
                i += len(raw_end)
                raw_end = None
                state = "code"
            else:
                out.append(text[i])
                i += 1
        elif state == "char":
            out.append(text[i])
            if text[i] == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
            elif text[i] == "'":
                i += 1
                state = "code"
            else:
                i += 1
        elif state == "block_comment":
            if text.startswith("/*", i):
                block_depth += 1
                out.append("  ")
                i += 2
            elif text.startswith("*/", i):
                block_depth -= 1
                out.append("  ")
                i += 2
                if block_depth == 0:
                    state = "code"
            else:
                out.append("\n" if text[i] == "\n" else " ")
                i += 1

    return "".join(out)


def normalized_source(path: pathlib.Path) -> str:
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".rs":
        text = strip_rust_comments(text)
    return (
        text
        .replace("\\\n", " ")
        .replace("\\n", " ")
        .replace('\\"', '"')
    )


def iter_files():
    for root in roots:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if path.as_posix().endswith("tests/dos7_d4_lint_test.rs"):
                continue
            if path.suffix in (".rs", ".sql"):
                yield path


violations = []
IDENT = r"(?:[\"`']?[A-Za-z_][A-Za-z0-9_]*[\"`']?|\[[^\]]+\])"
CLAIMS_TABLE = r"(?:[\"`']?intelligence_claims[\"`']?|\[intelligence_claims\])"
update_re = re.compile(
    rf"\bUPDATE\s+(?:OR\s+\w+\s+)?(?:{IDENT}\s*\.\s*)?{CLAIMS_TABLE}(?=\W|$)",
    re.I,
)

for path in iter_files():
    text = normalized_source(path)
    for match in update_re.finditer(text):
        window = text[match.start(): match.start() + 8000]
        statement_end = window.find(");")
        statement = window if statement_end == -1 else window[:statement_end]
        if "dos7-allowed:" in statement:
            continue

        columns = parse_update_columns(text, match.start())
        denied = sorted({column for column in columns if column not in ALLOWED})
        if denied:
            line = text.count("\n", 0, match.start()) + 1
            violations.append((str(path), line, ", ".join(denied)))

if violations:
    print("UPDATE on non-allowlisted intelligence_claims columns is forbidden.")
    print("Allowed columns are parsed from services/claims.rs::CLAIM_UPDATE_ALLOWED_COLUMNS:")
    print("  " + ", ".join(sorted(ALLOWED)))
    print("Use /* dos7-allowed: ... */ only for documented one-time migration/backfill exceptions.")
    print()
    for path, line, columns in sorted(set(violations)):
        print(f"{path}:{line}: non-allowlisted SET column(s): {columns}")
    sys.exit(1)

print("All UPDATE statements against intelligence_claims target allowlisted mutable columns only.")
PY
