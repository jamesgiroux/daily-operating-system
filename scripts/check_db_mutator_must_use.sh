#!/usr/bin/env bash
# Enforce #[must_use] on every public mutation method in src-tauri/src/db/*.rs that
# returns Result.
#
# A "mutation method" is a `pub fn` (or `pub(crate) fn`) that:
# - returns a Result, AND
# - has a body that contains SQL DML keywords (INSERT, UPDATE, DELETE, REPLACE) or
#   rusqlite mutating calls (.execute, .execute_batch, .execute_named).
#
# Why this lint exists: Result is already #[must_use], but explicit annotations
# document the intent at the call site, lower the cost of a wrapper trait or type
# alias accidentally erasing the implicit must_use, and create an audit trail for
# DB mutation semantics. DOS-342 promised "every public DB mutation method that
# returns Result" carries an explicit annotation; this lint guards that promise
# against drift.
#
# Run: bash scripts/check_db_mutator_must_use.sh
# Set CHECK_DB_MUTATOR_REPORT=1 to print a per-file summary.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB_DIR="$ROOT_DIR/src-tauri/src/db"

if [[ ! -d "$DB_DIR" ]]; then
  echo "check_db_mutator_must_use: missing $DB_DIR" >&2
  exit 2
fi

shopt -s nullglob
fail=0
total_mutators=0
total_annotated=0

for file in "$DB_DIR"/*.rs; do
  rel="${file#"$ROOT_DIR/"}"
  python3 - "$file" "$rel" <<'PYEOF'
import re
import sys

path, rel = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as fh:
    text = fh.read()
lines = text.split("\n")

PUB_FN = re.compile(r"^\s*pub(?:\([a-z]+\))?\s+fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
RESULT = re.compile(r"->\s*[A-Za-z_:<&'\s,]*Result\b")
MUTATION = re.compile(
    r"(?:INSERT|UPDATE|DELETE|REPLACE)\s+(?:INTO|FROM|OR|TABLE)"
    r"|\.execute(?:_batch|_named)?\s*\("
)

mutators = annotated = failures = 0
i = 0
while i < len(lines):
    line = lines[i]
    m = PUB_FN.match(line)
    if not m:
        i += 1
        continue
    name = m.group("name")
    # Capture full signature until '{' or ';'.
    j = i
    sig = line
    while "{" not in sig and ";" not in sig and j + 1 < len(lines):
        j += 1
        sig += "\n" + lines[j]
    if ";" in sig and "{" not in sig:
        i = j + 1
        continue
    if not RESULT.search(sig):
        i = j + 1
        continue
    # Pure constructors are not "mutation methods" even though they may run setup pragmas.
    # Connection/db handle types are independently #[must_use] via their own definition.
    # NOTE: with_transaction is intentionally NOT excluded — its closure can do writes;
    # `let _ = db.with_transaction(|tx| tx.upsert(...))` is the canonical footgun.
    if name == "new" or name.startswith("open_") or name.startswith("from_"):
        i = j + 1
        continue
    # Capture body until matching brace.
    depth = sig.count("{") - sig.count("}")
    body = sig
    k = j
    while depth > 0 and k + 1 < len(lines):
        k += 1
        body += "\n" + lines[k]
        depth += lines[k].count("{") - lines[k].count("}")
    if not MUTATION.search(body):
        i = k + 1
        continue
    mutators += 1
    # Walk back from the pub fn line over attrs/comments/blank lines to find #[must_use].
    has_must_use = False
    p = i - 1
    while p >= 0:
        ln = lines[p].strip()
        if not ln:
            p -= 1
            continue
        if ln.startswith("//"):
            p -= 1
            continue
        if ln.startswith("#["):
            if "must_use" in lines[p]:
                has_must_use = True
                break
            p -= 1
            continue
        # Multi-line attribute (inner lines without leading `#[`).
        if ln.startswith(("\"", "(", ")")) and p > 0 and "#[" in lines[p - 1]:
            p -= 1
            continue
        break
    if has_must_use:
        annotated += 1
    else:
        print(f"{rel}:{i + 1}: pub fn {name} returns Result and mutates state but lacks #[must_use]")
        failures += 1
    i = k + 1

print(f"AUDIT {rel} mutators={mutators} annotated={annotated} failures={failures}")
PYEOF
done > /tmp/check_db_mutator_must_use.out

while IFS= read -r line; do
  case "$line" in
    AUDIT*)
      m=$(printf '%s\n' "$line" | sed -n 's/.*mutators=\([0-9]*\).*/\1/p')
      a=$(printf '%s\n' "$line" | sed -n 's/.*annotated=\([0-9]*\).*/\1/p')
      f=$(printf '%s\n' "$line" | sed -n 's/.*failures=\([0-9]*\).*/\1/p')
      total_mutators=$((total_mutators + ${m:-0}))
      total_annotated=$((total_annotated + ${a:-0}))
      if [[ -n "${CHECK_DB_MUTATOR_REPORT:-}" ]]; then
        printf '%s\n' "$line"
      fi
      ;;
    *)
      printf '%s\n' "$line"
      fail=1
      ;;
  esac
done < /tmp/check_db_mutator_must_use.out
rm -f /tmp/check_db_mutator_must_use.out

if [[ "$fail" -ne 0 ]]; then
  printf '\n'
  printf 'check_db_mutator_must_use: FAIL — %d/%d mutator methods missing #[must_use]\n' \
    $((total_mutators - total_annotated)) "$total_mutators" >&2
  printf 'Annotate each method with #[must_use = "<reason>"] above the pub fn.\n' >&2
  exit 1
fi

printf 'check_db_mutator_must_use: OK — %d/%d mutator methods annotated\n' \
  "$total_annotated" "$total_mutators"
