#!/usr/bin/env bash
# prevent silent discarding of feedback / tombstone / file-write
# results. Catches `let _ = ...` patterns where the RHS calls one of the
# protected functions, in any of these call forms:
#   - method-call:   foo.write_intelligence_json(...)
#   - bare function: write_intelligence_json(...)
#   - qualified:     crate::intelligence::write_intelligence_json(...)
#   - typed binding: let _: T = ...
#   - named-prefix:  let _ignored = ...
#
# Does NOT catch (acceptable for W0; structural enforcement via
# `#[must_use]` + clippy::let_underscore_must_use is  territory):
#   - match { _ => () }
#   - if let Err(_) = ...
#   - .ok();
#   - wrapper-function indirection

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The named functions we want to protect.
FUNCTIONS='record_feedback_event|create_suppression_tombstone|write_intelligence_json'

# The pattern matches `let _<anything> [: type] = <anything> fn(` where the
# function call may appear in any of: method form (`.fn(`), qualified path
# form (`::fn(`), or bare form (`fn(`). `\b` is a zero-width word boundary
# that matches at start-of-identifier, covering all three forms.
PATTERN="let[[:space:]]+_[[:alnum:]_]*([[:space:]]*:[[:space:]]*[^=]+)?[[:space:]]*=[[:space:]].*\\b(${FUNCTIONS})[[:space:]]*\\("

violations=0
while IFS= read -r line; do
  # `grep -rEn` output: <file>:<lineno>:<text>
  # Skip the lint script itself + plan docs that describe the pattern.
  case "$line" in
    "$ROOT_DIR/scripts/check_no_let_underscore_feedback.sh"*) continue ;;
    "$ROOT_DIR/.docs/"*) continue ;;
  esac
  echo "$line"
  violations=$((violations + 1))
done < <(grep -rEn "$PATTERN" \
  "$ROOT_DIR/src-tauri/src/" \
  "$ROOT_DIR/src/" \
  2>/dev/null || true)

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: ${violations} swallowed feedback/tombstone/file-write call(s) detected."
  echo "Pattern: let _<...> = ...(record_feedback_event|create_suppression_tombstone|write_intelligence_json)(...)"
  echo "Use \`?\` propagation or explicit \`match\` with logging."
  exit 1
fi
