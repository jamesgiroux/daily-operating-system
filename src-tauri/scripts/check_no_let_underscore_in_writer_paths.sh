#!/usr/bin/env bash
#
# Writer-path lint: direct SQL execute results must not be silently
# discarded. A caller that intentionally treats a write as advisory must
# document that with a `// best-effort:` comment on the line immediately
# above the `let _ = ...execute(...)` statement.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  prefix="src-tauri/src"
else
  prefix="src"
fi

files=()

add_tree() {
  local dir="$1"
  [[ -d "$dir" ]] || return 0
  while IFS= read -r file; do
    files+=("$file")
  done < <(find "$dir" -type f -name '*.rs' | sort)
}

add_file() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  files+=("$file")
}

# Subtree scans cover logical writer surfaces that are expected to grow or split.
add_tree "$prefix/services"
add_tree "$prefix/db"
add_tree "$prefix/self_healing"

# Leaf modules below are writer paths that do not fit one writer-surface subtree.
add_file "$prefix/pty.rs"
add_file "$prefix/privacy.rs"
add_file "$prefix/executor.rs"
add_file "$prefix/db_backup.rs"
add_file "$prefix/intel_queue.rs"
add_file "$prefix/migrations.rs"

violations=""
for file in "${files[@]}"; do
  [[ -f "$file" ]] || continue
  file_violations="$(
    awk '
      function reset_tracking() {
        tracking = 0
        start_line = 0
        start_text = ""
        window = ""
        allowed = 0
      }
      function check_window() {
        if (window ~ /\.(execute|execute_batch)[[:space:]]*\(/ && !allowed) {
          print FILENAME ":" start_line ":" start_text
        }
      }
      /let[[:space:]]+_[[:space:]]*=/ && $0 !~ /^[[:space:]]*\/\// && !tracking {
        tracking = 1
        start_line = FNR
        start_text = $0
        window = $0 "\n"
        allowed = (prev ~ /^[[:space:]]*\/\/[[:space:]]*best-effort:/)
        if ($0 ~ /;/) {
          check_window()
          reset_tracking()
        }
        prev = $0
        next
      }
      tracking {
        window = window $0 "\n"
        if ($0 ~ /;/) {
          check_window()
          reset_tracking()
        }
      }
      { prev = $0 }
    ' "$file"
  )"
  if [[ -n "$file_violations" ]]; then
    violations+="$file_violations"$'\n'
  fi
done

violations="$(printf '%s\n' "$violations" | sed '/^$/d')"
if [[ -n "$violations" ]]; then
  echo "Silent let-underscore SQL execute writes are forbidden in audited writer paths."
  echo "Propagate the error, log it with context, or add a durable // best-effort: rationale directly above."
  echo
  echo "$violations"
  exit 1
fi

echo "No silent let-underscore SQL execute writes found in audited writer paths."
