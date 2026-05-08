#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INVENTORY="$ROOT_DIR/.docs/plans/wave-W1/W1-B-channel-inventory.md"

if [ ! -f "$INVENTORY" ]; then
  echo "missing channel inventory: $INVENTORY" >&2
  exit 1
fi

violations=0

while IFS= read -r hit; do
  file="${hit%%:*}"
  case "$file" in
    "$ROOT_DIR/src-tauri/src/signals/bus.rs") ;;
    "$ROOT_DIR/src-tauri/src/migrations.rs") ;;
    "$ROOT_DIR/src-tauri/src/db/accounts.rs") ;;
    *)
      echo "$hit"
      violations=$((violations + 1))
      ;;
  esac
done < <(
  rg -n -U --pcre2 \
    "INSERT\\s+(OR\\s+\\w+\\s+)?INTO\\s+signal_events|UPDATE\\s+(OR\\s+\\w+\\s+)?signal_events\\s+SET" \
    "$ROOT_DIR/src-tauri/src" \
    -g '*.rs' \
    2>/dev/null || true
)

if [ "$violations" -gt 0 ]; then
  echo "direct signal_events writes must route through signals::bus::emit_signal* or an inventoried lifecycle override" >&2
  exit 1
fi

required_inventory_entries=(
  "src-tauri/src/services"
  "src-tauri/src/abilities"
  "src-tauri/src/bridges/tauri.rs"
  "src-tauri/src/bridges/mcp.rs"
  "src-tauri/src/bridges/worker.rs"
  "src-tauri/src/bridges/eval.rs"
  "src-tauri/src/signals/derived_state_subscribers.rs"
  "src-tauri/src/signals/event_trigger.rs"
  "src-tauri/src/devtools/mod.rs"
  "src-tauri/src/migrations.rs"
)

for entry in "${required_inventory_entries[@]}"; do
  if ! grep -Fq "$entry" "$INVENTORY"; then
    echo "channel inventory missing required entry: $entry" >&2
    violations=$((violations + 1))
  fi
done

while IFS= read -r bin_file; do
  rel="${bin_file#"$ROOT_DIR/"}"
  if ! grep -Fq "$rel" "$INVENTORY"; then
    echo "channel inventory missing binary: $rel" >&2
    violations=$((violations + 1))
  fi
done < <(find "$ROOT_DIR/src-tauri/src/bin" -maxdepth 1 -type f -name '*.rs' | sort)

if ! grep -Fq "policy_for(signal: &SignalType)" "$ROOT_DIR/src-tauri/src/signals/policy_registry.rs"; then
  echo "policy registry must expose policy_for(signal: &SignalType)" >&2
  violations=$((violations + 1))
fi

if [ "$violations" -gt 0 ]; then
  exit 1
fi

echo "signal policy registry lint passed"
