#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INVENTORY="$ROOT_DIR/.docs/plans/wave-W1/W1-B-channel-inventory.md"

if [ ! -f "$INVENTORY" ]; then
  echo "missing channel inventory: $INVENTORY" >&2
  exit 1
fi

violations=0

# Filter rg hits through Python so we can skip lines inside `#[cfg(test)]` modules
# (test fixtures legitimately seed signal_events for setup; the policy-registry
# contract applies to production paths only).
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
    2>/dev/null \
    | python3 -c '
import re
import sys
from collections import defaultdict

# Build per-file maps of cfg(test) module line ranges so we can drop hits inside.
files = defaultdict(list)
for line in sys.stdin:
    line = line.rstrip("\n")
    if not line:
        continue
    parts = line.split(":", 2)
    if len(parts) < 3:
        print(line)
        continue
    files[parts[0]].append((int(parts[1]), line))

for path, hits in files.items():
    try:
        with open(path, encoding="utf-8") as fh:
            src = fh.read().split("\n")
    except OSError:
        for _, hit in hits:
            print(hit)
        continue
    test_ranges = []
    pending = False
    depth = 0
    in_test = False
    range_start = None
    for idx, ln in enumerate(src):
        stripped = ln.strip()
        if pending and stripped.startswith("mod "):
            in_test = True
            range_start = idx
            depth = ln.count("{") - ln.count("}")
            pending = False
            continue
        if pending and not stripped.startswith("#["):
            pending = False
        if re.match(r"\s*#\[cfg\s*\(\s*test\s*\)\s*\]", ln):
            pending = True
            continue
        if in_test:
            depth += ln.count("{") - ln.count("}")
            if depth <= 0:
                test_ranges.append((range_start, idx))
                in_test = False
    def in_test_block(line_no):
        for lo, hi in test_ranges:
            if lo + 1 <= line_no <= hi + 1:
                return True
        return False
    for line_no, hit in hits:
        if not in_test_block(line_no):
            print(hit)
' \
    || true
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
