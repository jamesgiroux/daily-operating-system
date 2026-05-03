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

files=(
  "$prefix/services/intelligence.rs"
  "$prefix/services/linear.rs"
  "$prefix/intel_queue.rs"
  "$prefix/self_healing/detector.rs"
  "$prefix/self_healing/quality.rs"
  "$prefix/self_healing/feedback.rs"
  "$prefix/self_healing/scheduler.rs"
  "$prefix/executor.rs"
  "$prefix/pty.rs"
  "$prefix/privacy.rs"
  "$prefix/db_backup.rs"
)

violations=""
for file in "${files[@]}"; do
  [[ -f "$file" ]] || continue
  file_violations="$(
    awk '
      /let[[:space:]]+_[[:space:]]*=[^;]*(\.execute|\.execute_batch)[[:space:]]*\(/ {
        if (prev !~ /^[[:space:]]*\/\/[[:space:]]*best-effort:/) {
          print FILENAME ":" FNR ":" $0
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
