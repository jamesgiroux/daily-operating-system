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
files_audited=0

# Fail-closed temp file: mktemp + trap + explicit error checks.
# Without this the audit can false-pass on a CI runner where /tmp is
# read-only or the script's $$ name collides — the gate would silently
# go green after scanning zero files.
TMPDIR_AUDIT="$(mktemp -d 2>/dev/null)" || {
  echo "check_db_mutator_must_use: FAIL — could not create temp directory" >&2
  exit 2
}
trap 'rm -rf "$TMPDIR_AUDIT"' EXIT INT TERM
TMPFILE="$TMPDIR_AUDIT/audit.out"
: > "$TMPFILE" || {
  echo "check_db_mutator_must_use: FAIL — could not initialize temp file at $TMPFILE" >&2
  exit 2
}

for file in "$DB_DIR"/*.rs; do
  rel="${file#"$ROOT_DIR/"}"
  files_audited=$((files_audited + 1))
  if ! python3 - "$file" "$rel" >> "$TMPFILE" <<'PYEOF'
import re
import sys

path, rel = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as fh:
    text = fh.read()
lines = text.split("\n")

PUB_FN = re.compile(r"^\s*pub(?:\([a-z()A-Za-z_:]+\))?\s+fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
TRAIT_FN = re.compile(r"^\s*fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
TRAIT_IMPL = re.compile(r"^\s*impl\s+[A-Za-z_:][\w:<>,\s']*\s+for\s+[A-Za-z_:][\w:<>,\s']*\s*\{")
PUB_TRAIT = re.compile(r"^\s*pub(?:\([a-z()A-Za-z_:]+\))?\s+trait\s+[A-Za-z_]")
RESULT = re.compile(r"->\s*[A-Za-z_:<&'\s,]*Result\b")
MUTATION = re.compile(
    r"(?:INSERT|UPDATE|DELETE|REPLACE)\s+(?:INTO|FROM|OR|TABLE)"
    r"|\.execute(?:_batch|_named)?\s*\("
)

# Track trait-impl/pub-trait scope so trait methods (which use bare `fn`) are
# treated as public-equivalent for the must_use rule.
def find_block_end(start_line):
    """Given a line containing the opening `{` of a block, return the line index
    where the matching `}` lives (last brace on its line). 0-indexed."""
    depth = 0
    started = False
    for idx in range(start_line, len(lines)):
        opens = lines[idx].count("{")
        closes = lines[idx].count("}")
        depth += opens - closes
        if opens > 0:
            started = True
        if started and depth <= 0:
            return idx
    return len(lines) - 1

trait_scopes = []  # list of (end_line, kind) where kind in {"impl", "trait"}
for idx, ln in enumerate(lines):
    if TRAIT_IMPL.match(ln) or PUB_TRAIT.match(ln):
        end = find_block_end(idx)
        trait_scopes.append((idx, end, "impl" if TRAIT_IMPL.match(ln) else "trait"))

def in_trait_scope(line_idx):
    """Return scope kind ('impl' or 'trait') if line_idx is inside one, else None."""
    for start, end, kind in trait_scopes:
        if start < line_idx <= end:
            return kind
    return None

mutators = annotated = failures = 0
i = 0
while i < len(lines):
    line = lines[i]
    m = PUB_FN.match(line)
    name = None
    is_trait_member = False
    if m:
        name = m.group("name")
    else:
        # Bare `fn` inside a trait or trait-impl scope counts as public-equivalent.
        tm = TRAIT_FN.match(line)
        scope_kind = in_trait_scope(i) if tm else None
        if scope_kind is not None:
            name = tm.group("name")
            is_trait_member = True
            # Trait IMPL methods (bare `fn` inside `impl Trait for Type {}`)
            # CANNOT carry `#[must_use]` themselves — Rust rejects the attribute
            # on impl-of-trait methods; the contract lives on the trait DECL and
            # the compiler propagates it. Skip the impl audit; the trait decl in
            # the same file will be audited separately.
            if scope_kind == "impl":
                i += 1
                continue
    if name is None:
        i += 1
        continue
    # Capture full signature until '{' or ';'.
    j = i
    sig = line
    while "{" not in sig and ";" not in sig and j + 1 < len(lines):
        j += 1
        sig += "\n" + lines[j]
    # Trait declarations end with `;` and have no body — still subject to the rule
    # via the trait-method must_use contract.
    is_decl_only = ";" in sig and "{" not in sig
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
    # Capture body until matching brace (skip for trait-decl-only signatures).
    if is_decl_only:
        body = sig
        k = j
    else:
        depth = sig.count("{") - sig.count("}")
        body = sig
        k = j
        while depth > 0 and k + 1 < len(lines):
            k += 1
            body += "\n" + lines[k]
            depth += lines[k].count("{") - lines[k].count("}")
    # For trait declarations the body is empty; skip the mutation check —
    # the trait declaration's must_use is enforced if any impl is a mutator.
    # We still want to flag the trait-decl when at least one impl in the file
    # mutates; conservatively, we require the trait method to carry must_use
    # whenever it returns Result and any impl-of-this-trait in the file mutates.
    if not is_decl_only and not MUTATION.search(body):
        i = k + 1
        continue
    if is_decl_only:
        # Look ahead for a corresponding trait impl method body that mutates.
        any_impl_mutates = False
        for impl_start, impl_end, kind in trait_scopes:
            if kind != "impl":
                continue
            # Search for the same fn name inside this impl block.
            for q in range(impl_start, impl_end + 1):
                im = TRAIT_FN.match(lines[q])
                if im and im.group("name") == name:
                    # Capture the impl method body.
                    sig2 = lines[q]
                    rr = q
                    while "{" not in sig2 and rr + 1 < len(lines):
                        rr += 1
                        sig2 += "\n" + lines[rr]
                    depth2 = sig2.count("{") - sig2.count("}")
                    body2 = sig2
                    while depth2 > 0 and rr + 1 < len(lines):
                        rr += 1
                        body2 += "\n" + lines[rr]
                        depth2 += lines[rr].count("{") - lines[rr].count("}")
                    if MUTATION.search(body2):
                        any_impl_mutates = True
                        break
            if any_impl_mutates:
                break
        if not any_impl_mutates:
            i = j + 1
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
        kind_label = "trait fn" if is_trait_member else ("trait decl" if is_decl_only else "pub fn")
        print(f"{rel}:{i + 1}: {kind_label} {name} returns Result and mutates state but lacks #[must_use]")
        failures += 1
    i = k + 1

print(f"AUDIT {rel} mutators={mutators} annotated={annotated} failures={failures}")
PYEOF
  then
    echo "check_db_mutator_must_use: FAIL — Python audit failed for $rel" >&2
    exit 2
  fi
done

if [[ "$files_audited" -eq 0 ]]; then
  echo "check_db_mutator_must_use: FAIL — no .rs files found in $DB_DIR (heuristic would have false-passed)" >&2
  exit 2
fi

if [[ ! -s "$TMPFILE" ]]; then
  echo "check_db_mutator_must_use: FAIL — audit output file is empty (heuristic would have false-passed)" >&2
  exit 2
fi

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
done < "$TMPFILE"

if [[ "$fail" -ne 0 ]]; then
  printf '\n'
  printf 'check_db_mutator_must_use: FAIL — %d/%d mutator methods missing #[must_use]\n' \
    $((total_mutators - total_annotated)) "$total_mutators" >&2
  printf 'Annotate each method with #[must_use = "<reason>"] above the pub fn.\n' >&2
  exit 1
fi

printf 'check_db_mutator_must_use: OK — %d/%d mutator methods annotated\n' \
  "$total_annotated" "$total_mutators"
