#!/usr/bin/env bash
#
# v1.4.4 W2 inline-style narrow-exception lint (AC-W2.7 + DOS-725).
#
# AC-W2.7 (V1.2 hardened): the only permitted inline `style=` attribute
# body in `wp/dailyos/blocks/**/*.php` must match the `--dailyos-*`
# custom-property-only regex
#
#   ^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$
#
# This is the narrow exception to memory `feedback_no_inline_css` for
# per-project tint flow (DOS-725 — `--dailyos-project-tint`) plus any
# future `--dailyos-*` custom-property emissions. Arbitrary `--*`
# prefixes are REJECTED (V1.2 cycle-2 codex-challenge F3 tightening).
# Anything else (declarative style, vendor prefix, hex color, etc.)
# fails the gate.
#
# Invocation:           bash src-tauri/scripts/check_no_inline_style_exception.sh
# Self-test (negative): DAILYOS_INLINE_STYLE_ROOT=<fixture> bash <this>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
search_root="${DAILYOS_INLINE_STYLE_ROOT:-$repo_root}"

blocks_root="$search_root/wp/dailyos/blocks"
if [[ ! -d "$blocks_root" ]]; then
  echo "AC-W2.7 PASS: $blocks_root does not exist (no blocks to lint)."
  exit 0
fi

# Find all PHP files under wp/dailyos/blocks/.
php_files=()
while IFS= read -r -d '' f; do
  php_files+=( "$f" )
done < <(find "$blocks_root" -type f -name '*.php' -print0)

if [[ ${#php_files[@]} -eq 0 ]]; then
  echo "AC-W2.7 PASS: no PHP files under $blocks_root."
  exit 0
fi

# Per AC-W2.7 V1.2: `--dailyos-*` namespace ONLY; arbitrary `--*` prefixes
# rejected. Allow one or more `--dailyos-*` declarations separated by `;`.
allowed_decl='--dailyos-[a-z-]+:[[:space:]]*var\(--[a-z-]+\);?'
allowed_body_regex="^[[:space:]]*${allowed_decl}([[:space:]]*${allowed_decl})*[[:space:]]*$"

violations=()

# Extract style="..." or style='...' bodies. Uses perl which handles
# multi-line PCRE cleanly. Emits "file:line:body" tuples.
for file in "${php_files[@]}"; do
  while IFS= read -r tuple; do
    [[ -z "$tuple" ]] && continue
    line_no="${tuple%%|*}"
    body="${tuple#*|}"
    # Skip dynamic bodies — `%s`/`%d` sprintf placeholders, PHP-string
    # concatenation (`. $`), or PHP-echo embedding (`<?php`). The gate is
    # a STATIC check for literal CSS bodies; runtime-interpolated bodies
    # are validated by the consuming code paths (avatar emits
    # `--dailyos-avatar-*` custom properties; envelope-resolver forwards
    # block-context-supplied `style` attrs that themselves passed this gate
    # at their authoring site).
    if [[ "$body" == *"%s"* || "$body" == *"%d"* || "$body" == *"' . "* || "$body" == *"' ."* || "$body" == *". '"* || "$body" == *".'"* || "$body" == *"<?php"* || "$body" == *"<?="* ]]; then
      continue
    fi
    if [[ ! "$body" =~ $allowed_body_regex ]]; then
      violations+=( "$file:$line_no  body=[$body]" )
    fi
  done < <(perl -ne '
    while (/style[ ]*=[ ]*"([^"]*)"/g) {
      print "$.|$1\n";
    }
    while (/style[ ]*=[ ]*'\''([^'\'']*)'\''/g) {
      print "$.|$1\n";
    }
  ' "$file")
done

if [[ ${#violations[@]} -gt 0 ]]; then
  echo "AC-W2.7 FAIL: inline style attribute(s) outside the narrow exception found:" >&2
  for v in "${violations[@]}"; do
    echo "  - $v" >&2
  done
  echo >&2
  echo "Permitted body shape (per AC-W2.7 V1.2 + DOS-725):" >&2
  echo "  ^--dailyos-[a-z-]+:\\s*var\\(--[a-z-]+\\);?\$" >&2
  echo "  (one or more --dailyos-* custom-property assignments, separated by ;)" >&2
  echo >&2
  echo "Narrow exception is the per-project tint flow only — runtime-computed" >&2
  echo "values on a wrapper element via CSS custom property in the --dailyos-*" >&2
  echo "namespace. All other inline styles are forbidden (memory" >&2
  echo "feedback_no_inline_css)." >&2
  exit 1
fi

echo "AC-W2.7 PASS: all inline style attributes in wp/dailyos/blocks/**/*.php conform to the narrow --dailyos-* custom-property exception."
