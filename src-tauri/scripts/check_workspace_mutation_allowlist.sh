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

allowed_regex='services/workspace_ingestion/|tests/workspace_ingestion_|tests/workspace_mutation_allowlist_test\.rs|tests/watcher_fixture_|src/accounts\.rs|src/processor/|src/projects\.rs|src/commands/app_support\.rs|src/commands/workspace\.rs|src/db_backup\.rs|src/services/accounts\.rs'
table_pattern='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO|UPDATE|DELETE[[:space:]]+FROM)[[:space:]]+(workspace_file_lifecycle|document_ingestion_runs|document_entity_links)\b'
fs_pattern='(std::fs::(write|rename|copy|remove_file|remove_dir|remove_dir_all|create_dir|create_dir_all)|tokio::fs::(write|rename|copy|remove_file|remove_dir|remove_dir_all|create_dir|create_dir_all))'
comment_allowlist_pattern='dos7-allowed: (drive-staging-v146|inbox-bootstrap|entity-markdown-regen|content-index-cache|transcript-direct-write-v146|devtools-w5-backfill-fixture|v146-validation-fixture)'

filter_comment_allowlist() {
  local matches="$1"
  local filtered=""
  local file line rest start context

  while IFS=: read -r file line rest; do
    [[ -z "${file:-}" || -z "${line:-}" ]] && continue
    start=$(( line > 4 ? line - 4 : 1 ))
    context="$(sed -n "${start},${line}p" "$file")"
    if [[ "$context" =~ $comment_allowlist_pattern ]]; then
      continue
    fi
    filtered+="${file}:${line}:${rest}"$'\n'
  done <<< "$matches"

  printf '%s' "$filtered"
}

filter_cfg_test_modules() {
  python3 -c '
import re
import sys
from collections import defaultdict

files = defaultdict(list)
for raw in sys.stdin:
    line = raw.rstrip("\n")
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
            source = fh.read().split("\n")
    except OSError:
        for _, hit in hits:
            print(hit)
        continue

    test_ranges = []
    pending_cfg_test = False
    in_test_module = False
    range_start = None
    depth = 0

    for idx, src_line in enumerate(source):
        stripped = src_line.strip()
        if pending_cfg_test and re.match(r"(pub\s+)?mod\s+\w+", stripped):
            in_test_module = True
            range_start = idx
            depth = src_line.count("{") - src_line.count("}")
            pending_cfg_test = False
            continue
        if pending_cfg_test and not stripped.startswith("#["):
            pending_cfg_test = False
        if re.match(r"\s*#\[cfg\s*\(\s*test\s*\)\s*\]", src_line):
            pending_cfg_test = True
            continue
        if in_test_module:
            depth += src_line.count("{") - src_line.count("}")
            if depth <= 0:
                test_ranges.append((range_start + 1, idx + 1))
                in_test_module = False

    def in_test_range(line_no):
        return any(start <= line_no <= end for start, end in test_ranges)

    for line_no, hit in hits:
        if not in_test_range(line_no):
            print(hit)
'
}

table_matches="$(
  grep -rEni --include='*.rs' --include='*.sql' "$table_pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev '^[^:]+:[0-9]+:[[:space:]]*(//|--)' \
    | grep -Ev "($allowed_regex)" \
    || true
)"
table_matches="$(printf '%s' "$table_matches" | filter_cfg_test_modules)"
table_matches="$(filter_comment_allowlist "$table_matches")"

fs_matches="$(
  grep -rEn --include='*.rs' "$fs_pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "($allowed_regex)" \
    | grep -Ei 'workspace|Workspace|workspace_root|workspace_path|canonical_path|file_ref' \
    || true
)"
fs_matches="$(printf '%s' "$fs_matches" | filter_cfg_test_modules)"
fs_matches="$(filter_comment_allowlist "$fs_matches")"

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
