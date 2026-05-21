#!/usr/bin/env bash
#
# Workspace-ingestion mutation lint.
#
# Runtime writes to workspace lifecycle/run/link tables and direct workspace-file
# filesystem mutation must stay inside services/workspace_ingestion/.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

allowed_regex='services/workspace_ingestion/|tests/workspace_ingestion_|tests/workspace_mutation_allowlist_test\.rs|src/accounts\.rs|src/processor/|src/projects\.rs|src/commands/app_support\.rs|src/commands/workspace\.rs|src/db_backup\.rs|src/services/accounts\.rs'
table_pattern='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO|UPDATE|DELETE[[:space:]]+FROM)[[:space:]]+(workspace_file_lifecycle|document_ingestion_runs|document_entity_links)\b'
fs_pattern='(std::fs::(write|rename|copy|remove_file|remove_dir|remove_dir_all|create_dir|create_dir_all)|tokio::fs::(write|rename|copy|remove_file|remove_dir|remove_dir_all|create_dir|create_dir_all))'

table_matches="$(
  grep -rEni --include='*.rs' --include='*.sql' "$table_pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev '^[^:]+:[0-9]+:[[:space:]]*(//|--)' \
    | grep -Ev "($allowed_regex)" \
    || true
)"

fs_matches="$(
  grep -rEn --include='*.rs' "$fs_pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "($allowed_regex)" \
    | grep -Ei 'workspace|Workspace|workspace_root|workspace_path|canonical_path|file_ref' \
    || true
)"

if [[ -n "$table_matches" || -n "$fs_matches" ]]; then
  echo "Workspace lifecycle/run/link table writes and workspace-file filesystem writes must route through services/workspace_ingestion."
  echo
  if [[ -n "$table_matches" ]]; then
    echo "Table write violations:"
    echo "$table_matches"
    echo
  fi
  if [[ -n "$fs_matches" ]]; then
    echo "Filesystem write violations:"
    echo "$fs_matches"
    echo
  fi
  exit 1
fi

echo "Workspace mutation allowlist clean."
