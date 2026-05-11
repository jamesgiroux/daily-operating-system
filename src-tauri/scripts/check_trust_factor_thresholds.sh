#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TRUST_DIR="$TAURI_ROOT/abilities-runtime/src/abilities/trust"
FLOAT_LITERAL_PATTERN='(?<![A-Za-z0-9_])[0-9][0-9_]*\.[0-9][0-9_]*(?:[A-Za-z_][A-Za-z0-9_]*)?'

if [[ ! -d "$TRUST_DIR" ]]; then
  echo "Trust factor threshold lint: missing trust directory: $TRUST_DIR"
  exit 1
fi

offenders="$(mktemp)"
trap 'rm -f "$offenders"' EXIT

set +e
rg -n --pcre2 "$FLOAT_LITERAL_PATTERN" "$TRUST_DIR" \
  -g '*.rs' \
  -g '!**/*_test.rs' \
  -g '!**/config.rs' > "$offenders"
status=$?
set -e

if [[ "$status" -eq 0 ]]; then
  echo "Trust factor threshold lint: float literals found outside config.rs and *_test.rs:"
  cat "$offenders"
  exit 1
fi

if [[ "$status" -ne 1 ]]; then
  echo "Trust factor threshold lint: rg failed with status $status"
  exit "$status"
fi

echo "Trust factor threshold lint passed."
