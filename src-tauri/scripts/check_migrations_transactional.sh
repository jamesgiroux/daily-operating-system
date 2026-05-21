#!/usr/bin/env bash
#
# v1.4.4 W1W2 L2 cycle-2 class-wide gate: destructive migrations MUST be
# wrapped in BEGIN IMMEDIATE / COMMIT.
#
# Root class: the migration runner at `src-tauri/src/migrations.rs` invokes
# `conn.execute_batch(sql)` which does NOT wrap a multi-statement batch in
# a single transaction unless the SQL itself contains explicit
# `BEGIN IMMEDIATE; ... COMMIT;`. When a destructive batch (DROP TABLE,
# DROP VIEW, ALTER TABLE ... RENAME, or any table-rebuild) executes in
# autocommit mode, each statement commits separately. Multi-process readers
# opening the encrypted DB during the migration window can then observe an
# intermediate schema state (e.g., the moment between DROP and CREATE) and
# fail with "no such table/view".
#
# This script is the structural gate that prevents the class from
# recurring. For every SQL file under `src-tauri/src/migrations/`, if the
# file contains a destructive pattern, it MUST also contain `BEGIN
# IMMEDIATE` and `COMMIT`. Non-destructive migrations (pure CREATE / pure
# INSERT / pure CREATE INDEX) are not required to wrap — they remain
# safe under the per-statement autocommit semantics.
#
# History:
#   - v243 introduced the class via non-transactional DROP VIEW + CREATE
#     VIEW; v244 fixed it via BEGIN IMMEDIATE wrap (L3 cycle-2 F3).
#   - v245 reintroduced the class via non-transactional CREATE/INSERT/
#     DROP/RENAME table rebuild; W1W2 L2 cycle-2 fix wraps it.
#
# Invocation: bash src-tauri/scripts/check_migrations_transactional.sh

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
search_root="${DAILYOS_MIGRATIONS_ROOT:-$repo_root/src-tauri/src/migrations}"

if [[ ! -d "$search_root" ]]; then
  echo "FAIL: migrations directory not found at $search_root" >&2
  exit 1
fi

shopt -s nullglob
sql_files=( "$search_root"/*.sql )
shopt -u nullglob

if [[ ${#sql_files[@]} -eq 0 ]]; then
  echo "FAIL: no .sql migration files found under $search_root" >&2
  exit 1
fi

# Legacy allowlist: destructive migrations that shipped before this gate
# existed. Replaying them is a no-op once applied (each migration row
# records applied_at), and editing the SQL would change the content hash
# in installs that already ran them. The gate is forward-looking: any NEW
# destructive migration (added after this list was frozen at v245 W1W2 L2
# cycle-2) MUST satisfy the BEGIN IMMEDIATE / COMMIT contract. The v243
# entry remains here only because v244 supersedes it with an atomic
# rebuild of the same view; the v243 run is now a no-op on fresh installs
# anyway because v244 immediately overwrites the view.
#
# DO NOT add new entries to this list. New destructive migrations get
# BEGIN IMMEDIATE / COMMIT.
legacy_allowlist=(
  "003_account_team.sql"
  "010_foreign_keys.sql"
  "011_proposed_actions.sql"
  "014_granola_sync.sql"
  "023_drop_meeting_account_id.sql"
  "032_junction_fks_and_expr_indexes.sql"
  "039_person_relationships_types.sql"
  "049_drive_rename_type_column.sql"
  "067_feedback_unique_constraint.sql"
  "069_account_events_expand.sql"
  "070_captures_metadata.sql"
  "074_action_status_vocabulary.sql"
  "085_action_status_priority_v2.sql"
  "097_email_pending_retry_state.sql"
  "122_dos_321_collapse_commitment_dupes.sql"
  "135_dos_294_typed_feedback_schema.sql"
  "139_dos_301_projection_failed_index_v2.sql"
  "140_dos_287_temporal_scope_closed.sql"
  "141_user_note_claim_type_backfill.sql"
  "145_dos_379_entity_members_entity_fk.sql"
  "146_dos_212_signal_events_data_source.sql"
  "152_dos_215_temporal_entity_type_keys.sql"
  "153_targeted_repair_invalidation_jobs.sql"
  "158_dos_276_commitment_claim_identity.sql"
  "160_dos_276_commitment_identity_backlog_index.sql"
  "171_dos_565_drop_surface_bearer_token_hash.sql"
  "243_meeting_prep_status_indexed_view_deterministic.sql"
)

is_legacy_allowed() {
  local name="$1"
  local entry
  for entry in "${legacy_allowlist[@]}"; do
    if [[ "$entry" == "$name" ]]; then
      return 0
    fi
  done
  return 1
}

# Destructive patterns that MUST be wrapped in BEGIN IMMEDIATE / COMMIT.
# Regex is grep -E flavor, case-insensitive (we lower-case via grep -i).
destructive_patterns=(
  "^[[:space:]]*DROP[[:space:]]+TABLE"
  "^[[:space:]]*DROP[[:space:]]+VIEW"
  "^[[:space:]]*DROP[[:space:]]+INDEX"
  "^[[:space:]]*DROP[[:space:]]+TRIGGER"
  "^[[:space:]]*ALTER[[:space:]]+TABLE[[:space:]]+.*[[:space:]]+RENAME"
  "^[[:space:]]*ALTER[[:space:]]+TABLE[[:space:]]+.*[[:space:]]+DROP[[:space:]]+COLUMN"
)

# Strip SQL line comments (-- ...) before scanning, so a `-- DROP TABLE`
# reference inside a doc block does not trigger the gate. Block comments
# (/* ... */) are stripped naively per-line; multi-line block comments are
# rare in our migrations and the false-positive cost is "wrap a benign
# migration in BEGIN/COMMIT" which is harmless.
strip_sql_comments() {
  local file="$1"
  # Remove everything from `--` to end-of-line; remove /* ... */ on same line.
  sed -E -e 's/--.*$//' -e 's#/\*.*\*/##g' "$file"
}

violations=()

for file in "${sql_files[@]}"; do
  base="$(basename "$file")"
  stripped="$(strip_sql_comments "$file")"

  matched_pattern=""
  for pattern in "${destructive_patterns[@]}"; do
    if printf '%s\n' "$stripped" | grep -Eiq "$pattern"; then
      matched_pattern="$pattern"
      break
    fi
  done

  if [[ -z "$matched_pattern" ]]; then
    continue
  fi

  if is_legacy_allowed "$base"; then
    continue
  fi

  has_begin=0
  has_commit=0
  if printf '%s\n' "$stripped" | grep -Eiq '^[[:space:]]*BEGIN[[:space:]]+IMMEDIATE[[:space:]]*;'; then
    has_begin=1
  fi
  if printf '%s\n' "$stripped" | grep -Eiq '^[[:space:]]*COMMIT[[:space:]]*;'; then
    has_commit=1
  fi

  if [[ $has_begin -eq 1 && $has_commit -eq 1 ]]; then
    continue
  fi

  violations+=( "$base  (matched pattern: ${matched_pattern}; BEGIN IMMEDIATE=$has_begin, COMMIT=$has_commit)" )
done

if [[ ${#violations[@]} -gt 0 ]]; then
  echo "FAIL: destructive migration(s) missing BEGIN IMMEDIATE / COMMIT wrap:" >&2
  for v in "${violations[@]}"; do
    echo "  - $v" >&2
  done
  echo >&2
  echo "Destructive migrations (DROP TABLE/VIEW/INDEX/TRIGGER, ALTER TABLE" >&2
  echo "... RENAME, table-rebuilds) MUST wrap their statements in" >&2
  echo "BEGIN IMMEDIATE; ... COMMIT; so multi-process readers cannot" >&2
  echo "observe intermediate schema states. See" >&2
  echo "src-tauri/src/migrations/244_meeting_prep_status_view_transactional.sql" >&2
  echo "for the canonical pattern." >&2
  exit 1
fi

echo "PASS: every destructive migration is wrapped in BEGIN IMMEDIATE / COMMIT."
