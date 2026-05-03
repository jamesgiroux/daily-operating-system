#!/bin/bash
# Re-syncs every src/**/*.module.css into the reference's _shared/styles/
# mirror, strips the @import design-tokens line (the reference links
# design-tokens.css separately), and re-runs scope-modules.py to prefix
# every class selector with its module name (avoiding cross-module
# collisions when modules are loaded as plain CSS).
#
# Triggered by arch-doc-updater.sh when any src/**/*.module.css changes.
# Idempotent: safe to run repeatedly.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEST_DIR="$ROOT/.docs/design/reference/_shared/styles"
SCOPER="$ROOT/.docs/design/reference/_shared/scope-modules.py"

[ -d "$DEST_DIR" ] || exit 0

# Copy every src module CSS into the flat _shared/styles/ directory.
find "$ROOT/src/components" "$ROOT/src/pages" -name "*.module.css" 2>/dev/null | while read -r src; do
  cp "$src" "$DEST_DIR/$(basename "$src")"
done

# Strip @import design-tokens lines (the reference links it separately;
# leaving these in would cause double-import warnings).
sed -i '' '/^@import "..\/..\/styles\/design-tokens.css";$/d; /^@import "..\/..\/..\/styles\/design-tokens.css";$/d' "$DEST_DIR"/*.module.css 2>/dev/null || true

# Re-run scope-modules.py to prefix every class selector with its module name.
# Idempotent — already-prefixed classes are skipped.
if [ -f "$SCOPER" ]; then
  (cd "$(dirname "$SCOPER")" && python3 "$(basename "$SCOPER")") > /dev/null 2>&1 || true
fi
