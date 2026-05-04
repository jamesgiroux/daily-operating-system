#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 0 ]]; then
  ROOT_DIR="$(cd "$1" && pwd)"
else
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

SRC_DIR="$ROOT_DIR/src-tauri/src"
PATTERN='\b(write_intelligence_json|fenced_write_intelligence_json)[[:space:]]*\('
FN_DEF_PATTERN='fn[[:space:]]+(write_intelligence_json|fenced_write_intelligence_json)[[:space:]]*[<(]'

if [[ ! -d "$SRC_DIR" ]]; then
  echo "ERROR: source directory not found: $SRC_DIR" >&2
  exit 1
fi

count_char() {
  local text="$1"
  local char="$2"
  local stripped="${text//"$char"/}"
  echo $((${#text} - ${#stripped}))
}

is_cfg_test_context() {
  local file="$1"
  local target="$2"
  awk -v target="$target" '
    function count_char(s, c, n, i) {
      n = 0
      for (i = 1; i <= length(s); i++) {
        if (substr(s, i, 1) == c) n++
      }
      return n
    }
    {
      line = $0
      if (line ~ /#\[cfg\(test\)\]/) pending_test = 1

      opens = count_char(line, "{")
      closes = count_char(line, "}")

      if (pending_test && line ~ /(mod|fn|impl)[^{;]*\{/) {
        test_depth = depth + opens - closes
        if (test_depth <= depth) test_depth = depth + 1
        pending_test = 0
      }

      if (NR == target) {
        if (test_depth > 0 || pending_test) print "yes"; else print "no"
        exit
      }

      depth += opens - closes
      if (test_depth > 0 && depth < test_depth) test_depth = 0
    }
  ' "$file"
}

enclosing_fn_name() {
  local file="$1"
  local target="$2"
  awk -v target="$target" '
    function count_char(s, c, n, i) {
      n = 0
      for (i = 1; i <= length(s); i++) {
        if (substr(s, i, 1) == c) n++
      }
      return n
    }
    {
      line = $0
      opens = count_char(line, "{")
      closes = count_char(line, "}")

      if (line ~ /(^|[^[:alnum:]_])fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*[<(]/) {
        fn_line = line
        sub(/^.*(^|[^[:alnum:]_])fn[[:space:]]+/, "", fn_line)
        sub(/[[:space:]]*[<(].*$/, "", fn_line)
        current_fn = fn_line
        fn_depth = depth + opens - closes
        if (fn_depth <= depth) fn_depth = depth + 1
      }

      if (NR == target) {
        print current_fn
        exit
      }

      depth += opens - closes
      if (fn_depth > 0 && depth < fn_depth) {
        current_fn = ""
        fn_depth = 0
      }
    }
  ' "$file"
}

violations=0

while IFS= read -r hit; do
  file="${hit%%:*}"
  rest="${hit#*:}"
  line_no="${rest%%:*}"
  line_text="${rest#*:}"

  case "$file" in
    "$SRC_DIR/intelligence/write_fence.rs") continue ;;
  esac

  if [[ "$line_text" =~ ^[[:space:]]*// ]]; then
    continue
  fi
  if [[ "$line_text" =~ $FN_DEF_PATTERN ]]; then
    continue
  fi
  if [[ "$(is_cfg_test_context "$file" "$line_no")" == "yes" ]]; then
    continue
  fi

  fn_name="$(enclosing_fn_name "$file" "$line_no")"
  if [[ "$fn_name" == *_post_commit_* ]]; then
    continue
  fi

  echo "$hit"
  echo "  rationale: intelligence.json disk writes must run only after the DB transaction commits."
  echo "  fix: route through a post-commit helper or intelligence/write_fence.rs."
  violations=$((violations + 1))
done < <(grep -rEn "$PATTERN" "$SRC_DIR" 2>/dev/null || true)

if [[ "$violations" -gt 0 ]]; then
  echo
  echo "ERROR: ${violations} intelligence disk write call(s) outside approved post-commit/test/fence contexts."
  exit 1
fi
