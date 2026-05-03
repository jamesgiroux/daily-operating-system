#!/usr/bin/env bash
set -euo pipefail

ROOTS=(
  src
  src-tauri/src
  src-tauri/tests
  src-tauri/examples
  scripts
  src-tauri/scripts
  .github
  index.html
  src-tauri/Cargo.toml
)

COMMENT_PREFIX='^[[:space:]]*(//|///|//!|/\*|\*|#|--|<!--|\{/\*)'
EPHEMERAL_REF='(DOS-[0-9]+|I[0-9]{2,}|[Tt]he related change|[Ll]ive (Linear )?ticket|See the current implementation)'

if rg -n "${COMMENT_PREFIX}.*${EPHEMERAL_REF}" "${ROOTS[@]}" \
  --glob '!target/**' \
  --glob '!node_modules/**' \
  --glob '!dist/**' \
  --glob '!.git/**'; then
  cat <<'MSG'

Ephemeral issue reference found in a source comment.
Use a descriptive comment, a durable ADR/reference, or remove the comment.
MSG
  exit 1
fi
