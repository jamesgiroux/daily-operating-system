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

METHOD_PATTERN='([[:<:]]db[[:>:]]|[[:<:]]tx[[:>:]])\.(upsert|insert|update|delete|remove|reset|create|append|bump|link|unlink|merge|archive|record|mark|clear|copy|set_)[A-Za-z0-9_]*\('
RAW_SQL_PATTERN='conn_ref\(\)\.execute\('

violations=0

for rel_path in "${HOTSPOT_FILES[@]}"; do
  file_path="$ROOT_DIR/$rel_path"
  if [[ ! -f "$file_path" ]]; then
    echo "Missing hotspot file: $rel_path"
    violations=1
    continue
  fi

  cutoff_line="$(rg -n '#\[cfg\(test\)\]' "$file_path" | head -n1 | cut -d: -f1 || true)"
  if [[ -z "$cutoff_line" ]]; then
    cutoff_line=999999
  fi

  allow_next=0
  while IFS= read -r numbered_line; do
    line_no="${numbered_line%%:*}"
    line="${numbered_line#*:}"
    trimmed="$(printf '%s' "$line" | sed 's/^[[:space:]]*//')"

    if [[ "$line" == *"DIRECT_DB_ALLOWED:"* ]]; then
      allow_next=1
      continue
    fi

    if [[ "$trimmed" =~ ^// ]]; then
      continue
    fi

    if [[ "$allow_next" -eq 1 ]]; then
      if [[ -n "$trimmed" ]]; then
        allow_next=0
      fi
      continue
    fi

    if printf '%s\n' "$line" | grep -Eq "$METHOD_PATTERN"; then
      echo "Service-layer boundary violation: $rel_path:$line_no"
      echo "  direct mutation method call detected"
      violations=1
      continue
    fi

    if printf '%s\n' "$line" | grep -Eq "$RAW_SQL_PATTERN"; then
      echo "Service-layer boundary violation: $rel_path:$line_no"
      echo "  raw SQL write via conn_ref().execute(...) detected"
      violations=1
    fi
  done < <(awk -v cutoff="$cutoff_line" 'NR < cutoff { print NR ":" $0 }' "$file_path")
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
