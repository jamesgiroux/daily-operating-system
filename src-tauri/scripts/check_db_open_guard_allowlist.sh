#!/usr/bin/env bash
#
# DB open guard lint.
#
# Direct rusqlite opens of file-backed databases must stay behind the known DB
# chokepoints, where DbMode's production-path guard can run before key fetch or
# file open. New file-backed Connection::open/open_with_flags call sites outside
# this allowlist must either move behind ActionDb/DbService or carry a per-line
# `db-open-guard-allowed:` rationale.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src")
else
  roots=("src")
fi

allowed_path_regex='(^|/)db/core\.rs:|(^|/)db_service\.rs:|(^|/)db/key_provider\.rs:|(^|/)db/encryption\.rs:|(^|/)db_backup\.rs:|(^|/)migrations\.rs:|(^|/)harness/runner\.rs:|(^|/)services/claims_backfill\.rs:'
pattern='(rusqlite::)?Connection::open(_with_flags)?[[:space:]]*\('

matches="$(
  grep -rEn --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -v 'open_in_memory' \
    | grep -vE ':\s*(//|//!|///|\*)' \
    | grep -Ev "$allowed_path_regex" \
    | grep -v 'db-open-guard-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Direct file-backed SQLite opens outside the DB open guard allowlist are forbidden." >&2
  echo "Route through ActionDb/DbService guarded helpers, or add a narrow allowlist rationale." >&2
  echo >&2
  echo "Allowed chokepoint/exception files:" >&2
  echo "  - src-tauri/src/db/core.rs" >&2
  echo "  - src-tauri/src/db_service.rs" >&2
  echo "  - src-tauri/src/db/key_provider.rs" >&2
  echo "  - src-tauri/src/db/encryption.rs" >&2
  echo "  - src-tauri/src/db_backup.rs" >&2
  echo "  - src-tauri/src/migrations.rs" >&2
  echo "  - src-tauri/src/harness/runner.rs" >&2
  echo "  - src-tauri/src/services/claims_backfill.rs" >&2
  echo >&2
  echo "$matches" >&2
  exit 1
fi

echo "DB open guard allowlist clean."
