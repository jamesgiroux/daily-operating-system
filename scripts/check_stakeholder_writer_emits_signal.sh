#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${STAKEHOLDER_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC_DIR="$ROOT_DIR/src-tauri/src"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "stakeholder signal lint: missing source dir: $SRC_DIR" >&2
  exit 2
fi

failures=0

while IFS= read -r -d '' file; do
  rel="${file#"$ROOT_DIR"/}"
  case "$rel" in
    src-tauri/src/services/derived_state.rs) continue ;;
    src-tauri/src/demo.rs) continue ;;
    src-tauri/src/migrations.rs) continue ;;
    src-tauri/src/devtools/*) continue ;;
  esac

  awk -v file="$rel" '
    {
      lines[NR] = $0
      if (!test_tail_seen && $0 ~ /#\[cfg\(test\)\]/) {
        test_tail_seen = 1
        test_tail_start = NR
      }
    }
    END {
      n = NR
      if (test_tail_seen) {
        n = test_tail_start - 1
      }
      write_re = "(insert([[:space:]]+or[[:space:]]+(ignore|replace))?[[:space:]]+into|update|delete[[:space:]]+from)[^\"]*(account_stakeholders|entity_members)([^[:alnum:]_]|$)"
      for (i = 1; i <= n; i++) {
        if (tolower(lines[i]) !~ write_re) {
          continue
        }
        start = i - 30
        if (start < 1) {
          start = 1
        }
        stop = i + 30
        if (stop > n) {
          stop = n
        }
        skip = 0
        emit = 0
        signal = 0
        for (j = start; j <= stop; j++) {
          if (lines[j] ~ /stakeholder-cache-skip:[[:space:]]*[^[:space:]]/) {
            skip = 1
          }
          if (lines[j] ~ /emit_in_transaction[[:space:]]*\(/) {
            emit = 1
          }
          if (lines[j] ~ /stakeholders_changed|STAKEHOLDERS_CHANGED_SIGNAL/) {
            signal = 1
          }
        }
        if (!skip && !(emit && signal)) {
          printf "%s:%d: stakeholder table write lacks nearby emit_in_transaction(stakeholders_changed) or stakeholder-cache-skip rationale\n", file, i
          failures++
        }
      }
      if (failures > 0) {
        exit 1
      }
    }
  ' "$file" || failures=$((failures + 1))
done < <(find "$SRC_DIR" -type f -name '*.rs' -print0)

if [[ "$failures" -ne 0 ]]; then
  echo "stakeholder signal lint failed with $failures violation(s)" >&2
  exit 1
fi

echo "stakeholder signal lint passed"
