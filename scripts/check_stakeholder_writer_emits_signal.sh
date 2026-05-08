#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${STAKEHOLDER_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC_DIR="$ROOT_DIR/src-tauri/src"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "stakeholder signal lint: missing source dir: $SRC_DIR" >&2
  exit 2
fi

is_allowlisted_writer_file() {
  case "$1" in
    src-tauri/src/db/accounts.rs) return 0 ;;
    src-tauri/src/db/core.rs) return 0 ;;
    src-tauri/src/db/entity_linking.rs) return 0 ;;
    src-tauri/src/db/people.rs) return 0 ;;
    src-tauri/src/db/projects.rs) return 0 ;;
  esac
  return 1
}

failures=0

while IFS= read -r -d '' file; do
  rel="${file#"$ROOT_DIR"/}"
  case "$rel" in
    src-tauri/src/services/derived_state.rs) continue ;;
    src-tauri/src/demo.rs) continue ;;
    src-tauri/src/migrations.rs) continue ;;
    src-tauri/src/devtools/*) continue ;;
  esac

  allowlisted_writer=0
  if is_allowlisted_writer_file "$rel"; then
    allowlisted_writer=1
  fi

  helper_file=0
  if [[ "$rel" == "src-tauri/src/services/stakeholder_writer.rs" ]]; then
    helper_file=1
  fi

  awk -v file="$rel" -v allowlisted_writer="$allowlisted_writer" -v helper_file="$helper_file" '
    function max(a, b) { return a > b ? a : b }
    function min(a, b) { return a < b ? a : b }
    function has_wrapper(start, stop,    j) {
      for (j = start; j <= stop; j++) {
        if (lines[j] ~ /stakeholder_writer::write_with_stakeholders_changed(_for_entities)?[[:space:]]*\(/ ||
            lines[j] ~ /write_with_stakeholders_changed(_for_entities)?[[:space:]]*\(/) {
          return 1
        }
      }
      return 0
    }
    function is_non_graph_update(i, stop,    j, text) {
      if (tolower(lines[i]) !~ /update[^\"]*account_stakeholders/) {
        return 0
      }
      text = ""
      for (j = i; j <= stop; j++) {
        text = text "\n" tolower(lines[j])
      }
      if (text ~ /(set[[:space:]]+(engagement|assessment)|data_source_engagement|data_source_assessment)/ &&
          text !~ /(set|,)[[:space:]]*(account_id|person_id|status|relationship_type)[[:space:]]*=/) {
        return 1
      }
      return 0
    }
    function has_direct_stakeholder_emit(i, stop,    j, saw_emit, saw_signal) {
      saw_emit = 0
      saw_signal = 0
      for (j = i; j <= stop; j++) {
        if (lines[j] ~ /emit_in_transaction[[:space:]]*\(/) {
          saw_emit = 1
        }
        if (lines[j] ~ /STAKEHOLDERS_CHANGED_SIGNAL|stakeholders_changed/) {
          saw_signal = 1
        }
      }
      return saw_emit && saw_signal
    }
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
        line = tolower(lines[i])
        if (line !~ write_re) {
          continue
        }
        start = max(1, i - 60)
        stop = min(n, i + 30)
        if (allowlisted_writer || is_non_graph_update(i, stop) || has_wrapper(start, stop)) {
          continue
        }
        printf "%s:%d: stakeholder graph write must use stakeholder_writer::write_with_stakeholders_changed or an allowlisted DB writer\n", file, i
        failures++
      }

      if (!helper_file) {
        for (i = 1; i <= n; i++) {
          if (lines[i] !~ /emit_in_transaction[[:space:]]*\(|STAKEHOLDERS_CHANGED_SIGNAL|stakeholders_changed/) {
            continue
          }
          stop = min(n, i + 12)
          if (has_direct_stakeholder_emit(i, stop)) {
            printf "%s:%d: direct stakeholders_changed emit must use services::stakeholder_writer helper\n", file, i
            failures++
          }
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
