#!/usr/bin/env bash
# sync-chrome.sh — mirror the DailyOS app's design chrome into this theme.
#
# Source of truth: .docs/design/reference/_shared/ in the dailyos-repo.
# One-way: never edit synced files directly; re-run this script when the
# reference updates.
#
# Override the source by exporting DAILYOS_REFERENCE_ROOT.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
DEFAULT_SOURCE="$REPO_ROOT/.docs/design/reference/_shared"
SOURCE="${DAILYOS_REFERENCE_ROOT:-$DEFAULT_SOURCE}"
DEST="$REPO_ROOT/wp/dailyos/theme/assets/chrome"

if [[ ! -d "$SOURCE" ]]; then
  echo "ERROR: reference source not found at: $SOURCE" >&2
  echo "Set DAILYOS_REFERENCE_ROOT to the path of .docs/design/reference/_shared/" >&2
  exit 1
fi

mkdir -p "$DEST/styles" "$DEST/fonts"

copy_if_changed() {
  local src="$1"
  local dst="$2"
  if [[ ! -f "$src" ]]; then
    echo "  ! MISSING SOURCE: $src" >&2
    return 1
  fi
  if [[ -f "$dst" ]] && cmp -s "$src" "$dst"; then
    return 0
  fi
  cp "$src" "$dst"
  echo "  ✓ $(basename "$dst")"
}

echo "DailyOS chrome sync"
echo "  source: $SOURCE"
echo "  dest:   $DEST"
echo ""

echo "Tokens + fonts:"
copy_if_changed "$SOURCE/fonts.css" "$DEST/fonts.css"
copy_if_changed "$SOURCE/styles/design-tokens.css" "$DEST/styles/design-tokens.css"

echo ""
echo "Chrome modules:"
for module in MagazinePageLayout AtmosphereLayer FolioBar FloatingNavIsland Pill; do
  copy_if_changed "$SOURCE/styles/${module}.module.css" "$DEST/styles/${module}.module.css"
done

echo ""
echo "Font files:"
shopt -s nullglob
for woff in "$SOURCE/fonts"/*.woff2; do
  copy_if_changed "$woff" "$DEST/fonts/$(basename "$woff")"
done
shopt -u nullglob

echo ""
echo "chrome.js (with overrides applied):"
python3 "$SCRIPT_DIR/patch-chrome-js.py" < "$SOURCE/chrome.js" > "$DEST/chrome.js.tmp"
if [[ -f "$DEST/chrome.js" ]] && cmp -s "$DEST/chrome.js.tmp" "$DEST/chrome.js"; then
  rm "$DEST/chrome.js.tmp"
else
  mv "$DEST/chrome.js.tmp" "$DEST/chrome.js"
  echo "  ✓ chrome.js (patched)"
fi

# Record source revision so drift is auditable.
SOURCE_REPO_ROOT="$(cd "$SOURCE" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -n "$SOURCE_REPO_ROOT" ]]; then
  SHA="$(git -C "$SOURCE_REPO_ROOT" rev-parse HEAD:.docs/design/reference/_shared/chrome.js 2>/dev/null || echo unknown)"
  DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf "source: .docs/design/reference/_shared/chrome.js\nsha: %s\nsynced_at: %s\npatches_applied: 1,2,3,4,5,6,7,8,9a\n" "$SHA" "$DATE" > "$DEST/.synced-from"
fi

echo ""
echo "Done."
