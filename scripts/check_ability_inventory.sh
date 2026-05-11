#!/usr/bin/env bash
# Purpose: prevent drift between the live AbilityRegistry and the
# committed `tools/dailyos-abilities.json` surface inventory artifact.
#
# Mechanism: build and run `emit_ability_inventory` into a temp file,
# diff against the committed inventory, fail with the unified diff if
# they differ. The committed artifact is the contract Wave 3 (WP plugin
# + custom MCP server) and Wave 4 (block code) read at install / boot
# time; any drift here means three surfaces fall out of sync silently.
#
# Exit codes:
#   0  inventory matches the committed artifact.
#   1  inventory drift detected (or build / IO failure).
#
# How to run: ./scripts/check_ability_inventory.sh
#
# DailyOS ability-surface inventory drift check.

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

COMMITTED="tools/dailyos-abilities.json"
BINARY="emit_ability_inventory"

if [[ ! -f "$COMMITTED" ]]; then
  echo "check_ability_inventory: committed inventory missing at $COMMITTED" >&2
  echo "  remediation: cargo run --manifest-path src-tauri/Cargo.toml -p abilities-runtime --bin $BINARY -- --out $COMMITTED" >&2
  exit 1
fi

TEMP_FILE="$(mktemp -t dailyos-abilities-actual.XXXXXX.json)"
trap 'rm -f "$TEMP_FILE"' EXIT

# Build + run the emitter. We use `cargo run` so the script works from
# a fresh checkout without a pre-built target tree; cache layers in CI
# make subsequent runs cheap.
if ! cargo run --quiet --manifest-path src-tauri/Cargo.toml \
  -p abilities-runtime --bin "$BINARY" -- --out "$TEMP_FILE"; then
  echo "check_ability_inventory: $BINARY failed to emit inventory" >&2
  exit 1
fi

if ! diff -u "$COMMITTED" "$TEMP_FILE"; then
  cat >&2 <<'MSG'

check_ability_inventory: drift detected between live AbilityRegistry and committed tools/dailyos-abilities.json.

The diff above shows the deviation. To accept the change, regenerate the artifact:

  cargo run --manifest-path src-tauri/Cargo.toml -p abilities-runtime --bin emit_ability_inventory -- --out tools/dailyos-abilities.json

Then commit tools/dailyos-abilities.json alongside the descriptor change. The artifact is consumed by the WordPress plugin, the custom MCP server, and SurfaceClient introspection; keeping it committed prevents three surfaces from drifting silently.
MSG
  exit 1
fi

echo "check_ability_inventory: tools/dailyos-abilities.json matches the live registry."
