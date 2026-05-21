#!/usr/bin/env bash
#
# v1.4.4 W2 inline-style boundary lint (per L0-packet-W2-entity-surfaces.md
# V1.2 §10 invariant "Per-project tint via CSS custom property" /
# wp-skill H3 cycle-2 hardening).
#
# Allowlist (tightened from V1.1 per codex-challenge F3): only the
# --dailyos-* namespace is permitted on both sides of the inline-style
# custom-property assignment. The two permitted shapes are:
#
#   1. style="--dailyos-<seg>: var(--dailyos-<seg>);?"
#   2. 'style' => '--dailyos-<seg>: var(--dailyos-<seg>);?'
#      (inside a get_block_wrapper_attributes($args) array)
#
# Every other inline-style emission must move into a CSS Module /
# theme.json / block stylesheet.
#
# Invocation:           bash src-tauri/scripts/check_no_inline_style_exception.sh
# Self-test (negative): DAILYOS_INLINE_STYLE_ROOT=<fixture> bash <this>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
search_root="${DAILYOS_INLINE_STYLE_ROOT:-$repo_root}"

search_paths=(
  "$search_root/wp/dailyos/blocks"
  "$search_root/wp/dailyos/theme"
  "$search_root/wp/dailyos/includes"
  "$search_root/wp/dailyos/patterns"
)

# Allowlist regex — accepts the two permitted shapes (both quote styles).
allowlist='(style=("|'\'')--dailyos-[a-z][a-z0-9-]*:[[:space:]]*var\(--dailyos-[a-z][a-z0-9-]*\);?("|'\''))|(("|'\'')style("|'\'')[[:space:]]*=>[[:space:]]*("|'\'')--dailyos-[a-z][a-z0-9-]*:[[:space:]]*var\(--dailyos-[a-z][a-z0-9-]*\);?("|'\''))'
suspect_attr='style=("|'\'')[^"'\'']+("|'\'')'
suspect_arg='("|'\'')style("|'\'')[[:space:]]*=>[[:space:]]*("|'\'')[^"'\'']+("|'\'')'

# Contexts to skip (helper / validator / comment lines).
skip_context='(esc_attr|sanitize|preg_match|allowlist|check_no_inline_style|dailyos_inner_block_wrapper_attrs|args\[("|'\'')style("|'\'')\])'

# Baseline known-pre-existing violations carried into v1.4.4 W2 with this
# gate's introduction. New emissions outside the allowlist must NOT be
# added to this list — they need to be fixed at PR time. Entries are
# "<file>:<lineno>" relative to repo root.
#
# These trace to:
#   - wp/dailyos/blocks/avatar/render-functions.php — computed
#     background-color / initial-color emission, predates the boundary
#     (v1.4.2 W4-F primitive); refactor to CSS custom property tracked in
#     the DailyOS Codebase Maintenance & Production Quality project
#     (b8e6aea4-d47e-4f3a-b03d-a05bec914aeb).
#   - wp/dailyos/theme/templates/single-dailyos_account.html — core
#     wp-block-column flex-basis attribute serialization; output by core,
#     not authored by us; tracked under the same maintenance project as
#     a column-layout migration to theme.json once core supports it.
baseline_entries=(
  "wp/dailyos/blocks/avatar/render-functions.php:126"
  "wp/dailyos/blocks/avatar/render-functions.php:133"
  "wp/dailyos/theme/templates/single-dailyos_account.html:7"
  "wp/dailyos/theme/templates/single-dailyos_account.html:12"
)

is_baselined() {
  local key="$1"
  local entry
  for entry in "${baseline_entries[@]}"; do
    if [[ "$entry" == "$key" ]]; then
      return 0
    fi
  done
  return 1
}

violations=()

scan_file() {
  local file="$1"
  local lineno=0
  local line
  while IFS= read -r line; do
    lineno=$((lineno + 1))
    # Skip pure-comment lines.
    if [[ "$line" =~ ^[[:space:]]*(\#|//|\*|/\*) ]]; then
      continue
    fi
    if echo "$line" | grep -Eq "$skip_context"; then
      continue
    fi
    if echo "$line" | grep -Eq 'style=""'; then
      continue
    fi
    local has_suspect=0
    if echo "$line" | grep -Eq "$suspect_attr"; then
      has_suspect=1
    fi
    if echo "$line" | grep -Eq "$suspect_arg"; then
      has_suspect=1
    fi
    if [[ $has_suspect -eq 0 ]]; then
      continue
    fi
    if echo "$line" | grep -Eq "$allowlist"; then
      continue
    fi
    # Baseline allowlist (pre-existing emissions — see top of file).
    local rel_key="${file#$search_root/}:$lineno"
    if is_baselined "$rel_key"; then
      continue
    fi
    violations+=( "$file:$lineno: $line" )
  done < "$file"
}

for path in "${search_paths[@]}"; do
  if [[ ! -d "$path" ]]; then
    continue
  fi
  while IFS= read -r -d '' file; do
    case "$file" in
      */check_no_inline_style_exception.sh*) continue ;;
    esac
    scan_file "$file"
  done < <(find "$path" -type f \( -name "*.php" -o -name "*.html" \) -print0)
done

if [[ ${#violations[@]} -gt 0 ]]; then
  echo "check_no_inline_style_exception FAIL: ${#violations[@]} disallowed inline-style emission(s):" >&2
  for v in "${violations[@]}"; do
    echo "  $v" >&2
  done
  echo >&2
  echo "Only the --dailyos-*: var(--dailyos-*); shape is permitted in inline style attributes." >&2
  echo "Move any other inline style into a CSS Module / theme.json / block stylesheet." >&2
  exit 1
fi

echo "check_no_inline_style_exception PASS: no disallowed inline-style emissions found under wp/dailyos/."
