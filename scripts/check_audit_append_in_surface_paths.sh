#!/usr/bin/env bash
set -euo pipefail

WAIVER_FILE="scripts/audit-append-waivers.txt"

if [[ ! -f "$WAIVER_FILE" ]]; then
  echo "Missing $WAIVER_FILE"
  exit 1
fi

while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  if [[ ! "$line" =~ ^[^:]+:[^:]+:permanent$ ]]; then
    echo "Invalid waiver entry: $line"
    exit 1
  fi
done < "$WAIVER_FILE"

status=0
while IFS=: read -r file line _rest; do
  snippet=$(sed -n "${line},$((line + 12))p" "$file")
  event_kind=$(printf '%s\n' "$snippet" | perl -0ne 'print $1 if /audit(?:_log)?\.append\s*\(\s*"[^"]+"\s*,\s*"([^"]+)"/s')
  event_kind=${event_kind:-unknown_event_kind}
  key="$file:$event_kind"

  if ! grep -Fxq "$key:permanent" "$WAIVER_FILE"; then
    echo "legacy audit append in surface path: $file:$line event=$event_kind"
    status=1
  fi
done < <(rg -n 'audit(_log)?\.append\(' src-tauri/src/commands/*.rs src-tauri/src/services/*.rs || true)

if [[ "$status" -ne 0 ]]; then
  echo "Use emit_surface_audit with Actor::User for user-initiated command/service audit sites."
fi

exit "$status"
