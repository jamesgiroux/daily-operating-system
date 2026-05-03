#!/usr/bin/env bash
set -euo pipefail

# INTERIM — (separate crate split) is the structural fix; this script
# is best-effort enforcement until then.

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src/abilities" "src-tauri/abilities-macro/src")
else
  roots=("src/abilities" "abilities-macro/src")
fi

matches=""

run_grep_check() {
  local description="$1"
  local pattern="$2"
  local found

  found="$(grep -rEn --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null || true)"
  if [[ -n "$found" ]]; then
    matches+=$'\n'
    matches+="# ${description}"$'\n'
    matches+="$found"$'\n'
  fi
}

# Direct raw module access such as `crate::db::ActionDb`.
run_grep_check \
  "direct crate db/state/service module path" \
  'crate::(db|state|db_service|queries|pty)::'

# Grouped imports such as `use crate::{db::ActionDb, state::AppState};`.
run_grep_check \
  "grouped crate db/state/service import" \
  'use[[:space:]]+crate::\{[^}]*\b(db|state|db_service|queries|pty)\b[^}]*\}'

# Raw SQLite connection creation bypassing the ServiceContext boundary.
run_grep_check \
  "raw sqlite connection open" \
  'rusqlite::Connection::open|rusqlite::Connection::open_in_memory|sqlite::Connection::open'

# Filesystem mutators through std::fs or tokio::fs.
run_grep_check \
  "filesystem mutator" \
  'std::fs::(write|rename|create_dir|create_dir_all|remove_file|remove_dir|remove_dir_all|copy)|tokio::fs::(write|rename|create_dir|create_dir_all|remove_file|remove_dir|remove_dir_all|copy)'

# File API mutators and direct file opening.
run_grep_check \
  "File API creation/opening" \
  'File::(create|create_new|open)'

# OpenOptions can construct write handles even when the write call is indirect.
run_grep_check \
  "OpenOptions file handle construction" \
  'OpenOptions'

# Crate aliases such as `use crate as app;` followed by `app::db::...`.
while IFS= read -r file; do
  aliases="$(grep -Eo 'use[[:space:]]+crate[[:space:]]+as[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*;' "$file" 2>/dev/null \
    | sed -E 's/use[[:space:]]+crate[[:space:]]+as[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*;/\1/' || true)"
  for alias in $aliases; do
    found="$(grep -En "${alias}::(db|state|db_service|queries|pty)::" "$file" 2>/dev/null || true)"
    if [[ -n "$found" ]]; then
      matches+=$'\n'
      matches+="# crate alias db/state/service module path"$'\n'
      matches+="$(printf '%s\n' "$found" | sed "s#^#${file}:#")"$'\n'
    fi
  done
done < <(find "${roots[@]}" -type f -name '*.rs' 2>/dev/null)

if [[ -n "$matches" ]]; then
  echo "Raw DB/state/filesystem imports are forbidden in ability runtime and macro code."
  echo "$matches"
  exit 1
fi

echo "No raw DB/state/filesystem imports found in ability runtime or macro code."
