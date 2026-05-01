#!/usr/bin/env bash
set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src/abilities" "src-tauri/abilities-macro/src")
else
  roots=("src/abilities" "abilities-macro/src")
fi

pattern='(chrono::)?Utc::now\s*\(\)|rand::thread_rng\s*\(\)|thread_rng\s*\(\)|rand::rng\s*\(\)'

matches="$(
  rg -n "$pattern" "${roots[@]}" \
    | rg -v 'dos-210-grandfathered:' \
    | rg -v '(^|/)src/abilities/registry\.rs:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Direct wall-clock/RNG use is forbidden in ability runtime and macro code."
  echo "Use ServiceContext clock/RNG seams, or add // dos-210-grandfathered: for macro-emitted instrumentation."
  echo "$matches"
  exit 1
fi

echo "No direct clock/RNG use found in ability runtime or macro code."
