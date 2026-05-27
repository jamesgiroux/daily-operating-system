#!/usr/bin/env bash
# scripts/check_meetings_writer_boundary.sh
#
# Lane B of v1.4.8 (Meetings Substrate v2) — enforce that every meeting-row
# INSERT/UPDATE outside the explicit allowlist routes through
# `services::meetings_writer`.
#
# Allowlisted paths (raw INSERT/UPDATE on `meetings` permitted):
#   - src-tauri/src/services/meetings_writer/**     (the writer substrate itself)
#   - src-tauri/src/db/meetings.rs                  (canonical DB functions adapters call)
#   - src-tauri/src/db/accounts.rs                  (meeting_type reclassification)
#   - src-tauri/src/db/data_lifecycle.rs            (privacy scrubs + lifecycle)
#   - src-tauri/src/db/claim_invalidation.rs        (legacy seed for migration tests)
#   - src-tauri/src/db/mod_tests.rs                 (unit-test fixtures)
#   - src-tauri/src/migrations.rs                   (schema migrations)
#   - src-tauri/migrations/**                       (.sql migrations)
#   - src-tauri/src/devtools/**                     (devtools seeding)
#   - src-tauri/src/demo.rs                         (demo seeding)
#   - tests/**, *_test.rs, *_tests.rs               (test code)
#   - inside `#[cfg(test)] mod tests { ... }` blocks (production files with test-only seeds)
#
# Anything else writing `INSERT INTO meetings` or `UPDATE meetings SET ...`
# directly through `conn.execute` / `.prepare` is a substrate bypass.

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

EXIT=0

is_allowlisted_file() {
  local file="$1"
  case "$file" in
    src-tauri/src/services/meetings_writer/*)        return 0 ;;
    src-tauri/src/db/meetings.rs)                    return 0 ;;
    src-tauri/src/db/accounts.rs)                    return 0 ;;
    src-tauri/src/db/data_lifecycle.rs)              return 0 ;;
    src-tauri/src/db/claim_invalidation.rs)          return 0 ;;
    src-tauri/src/db/mod_tests.rs)                   return 0 ;;
    src-tauri/src/migrations.rs)                     return 0 ;;
    src-tauri/migrations/*)                          return 0 ;;
    src-tauri/src/devtools/*)                        return 0 ;;
    src-tauri/src/demo.rs)                           return 0 ;;
    src-tauri/tests/*)                               return 0 ;;
    *_test.rs|*_tests.rs)                            return 0 ;;
  esac
  return 1
}

# Check whether a given line number in a Rust file falls inside a
# `#[cfg(test)] mod ... { ... }` block. Approximation: walk backward through
# the file looking for the nearest `#[cfg(test)]` annotation; if one exists
# before this line, treat the line as test-scoped. Cheap and accurate enough
# for an advisory gate.
is_inside_test_module() {
  local file="$1"
  local line_no="$2"
  awk -v target="$line_no" '
    /^[[:space:]]*#\[cfg\(test\)\]/ { last_test = NR }
    NR == target {
      if (last_test > 0) { print "yes" } else { print "no" }
      exit
    }
  ' "$file"
}

scan_file() {
  local file="$1"
  [ -f "$file" ] || return 0
  if is_allowlisted_file "$file"; then
    return 0
  fi

  # Grep raw INSERT/UPDATE on meetings; collect (line, content) tuples.
  local matches
  matches="$(grep -n -E 'INSERT INTO meetings|UPDATE meetings SET|UPDATE meetings\b' "$file" 2>/dev/null || true)"
  [ -z "$matches" ] && return 0

  while IFS= read -r match; do
    [ -z "$match" ] && continue
    local line_no
    line_no="$(printf '%s' "$match" | cut -d: -f1)"
    local in_test
    in_test="$(is_inside_test_module "$file" "$line_no")"
    if [ "$in_test" = "yes" ]; then
      continue
    fi
    if [ $EXIT -eq 0 ]; then
      echo "Raw meetings-row INSERT/UPDATE found outside services/meetings_writer/." >&2
      echo "Route through services::meetings_writer::write() with a WriteRequest." >&2
      echo "" >&2
    fi
    printf '%s:%s\n' "$file" "$match" >&2
    EXIT=1
  done <<< "$matches"
}

if [ "${1:-}" = "--staged" ]; then
  STAGED_FILES="$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null || true)"
  while IFS= read -r file; do
    [ -z "$file" ] && continue
    case "$file" in *.rs) scan_file "$file" ;; esac
  done <<< "$STAGED_FILES"
else
  while IFS= read -r file; do
    scan_file "$file"
  done < <(find src-tauri/src -name "*.rs" -type f)
fi

exit $EXIT
