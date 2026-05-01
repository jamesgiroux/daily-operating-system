#!/usr/bin/env bash
set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src/abilities" "src-tauri/abilities-macro/src")
else
  roots=("src/abilities" "abilities-macro/src")
fi

pattern='crate::(db|state|db_service|queries|pty)::|tokio::fs::|std::fs::write|File::create|OpenOptions'

matches="$(grep -rEn --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null || true)"

if [[ -n "$matches" ]]; then
  echo "Raw DB/state/filesystem imports are forbidden in ability runtime and macro code."
  echo "$matches"
  exit 1
fi

echo "No raw DB/state/filesystem imports found in ability runtime or macro code."
