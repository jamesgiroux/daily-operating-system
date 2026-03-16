#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

HOTSPOT_FILES=(
  "src-tauri/src/commands.rs"
  "src-tauri/src/intel_queue.rs"
  "src-tauri/src/processor/transcript.rs"
  "src-tauri/src/workflow/deliver.rs"
  "src-tauri/src/workflow/reconcile.rs"
  "src-tauri/src/hygiene.rs"
)

while IFS= read -r command_file; do
  HOTSPOT_FILES+=("${command_file#$ROOT_DIR/}")
done < <(find "$ROOT_DIR/src-tauri/src/commands" -maxdepth 1 -name '*.rs' | sort)

METHOD_PATTERN='(db|tx)\.(upsert|insert|update|delete|remove|reset|create|append|bump|link|unlink|merge|archive|record|mark|clear|copy|set_)[A-Za-z0-9_]*'
RAW_SQL_PATTERN='conn_ref\(\)\.execute'

violations=0

for rel_path in "${HOTSPOT_FILES[@]}"; do
  file_path="$ROOT_DIR/$rel_path"
  if [[ ! -f "$file_path" ]]; then
    echo "Missing hotspot file: $rel_path"
    violations=1
    continue
  fi

  cutoff_line="$(grep -n '#\[cfg(test)\]' "$file_path" | head -n1 | cut -d: -f1 || true)"
  if [[ -z "$cutoff_line" ]]; then
    cutoff_line=999999
  fi

  awk \
    -v cutoff="$cutoff_line" \
    -v method="$METHOD_PATTERN" \
    -v raw="$RAW_SQL_PATTERN" \
    -v rel="$rel_path" '
      NR >= cutoff { exit }
      {
        line = $0
        trimmed = line
        sub(/^[[:space:]]+/, "", trimmed)

        if (index(line, "DIRECT_DB_ALLOWED:") > 0) {
          allow_next = 1
          next
        }

        if (trimmed ~ /^\/\//) {
          next
        }

        if (allow_next) {
          if (trimmed != "") {
            allow_next = 0
          }
          next
        }

        if (line ~ method) {
          printf("Service-layer boundary violation: %s:%d\n", rel, NR)
          print "  direct mutation method call detected"
          violations = 1
          next
        }

        if (line ~ raw) {
          printf("Service-layer boundary violation: %s:%d\n", rel, NR)
          print "  raw SQL write via conn_ref().execute(...) detected"
          violations = 1
        }
      }
      END {
        if (violations) {
          exit 10
        }
      }
    ' "$file_path"
  status=$?
  if [[ "$status" -eq 10 ]]; then
    violations=1
    continue
  fi
  if [[ "$status" -ne 0 ]]; then
    exit "$status"
  fi
done

if [[ "$violations" -ne 0 ]]; then
  cat <<'EOF'
One or more direct DB mutation patterns were found in I512 hotspot files.
Route writes through service-owned mutation APIs, or annotate justified exceptions:
  // DIRECT_DB_ALLOWED: <reason>
EOF
  exit 1
fi

echo "Service-layer boundary check passed."
