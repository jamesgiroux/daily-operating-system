#!/bin/bash
# Re-copies src/styles/design-tokens.css to the reference's _shared/styles/
# mirror so the static reference HTML stays in sync with production tokens.
#
# Triggered by arch-doc-updater.sh when src/styles/design-tokens.css changes.
# Idempotent: just a cp.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/src/styles/design-tokens.css"
DEST="$ROOT/.docs/design/reference/_shared/styles/design-tokens.css"

if [ -f "$SRC" ] && [ -d "$(dirname "$DEST")" ]; then
  cp "$SRC" "$DEST"
fi
