#!/usr/bin/env bash
#
# DOS-7 migration walkthrough — produces a per-mechanism before/after
# diff report on a copy of the user's dev DB.
#
# Read-only mode (default): snapshots the legacy dismissal tables and
# prints to stdout. Does NOT modify the input DB.
#
# Apply mode (--apply): copies the input DB into a tempfile, applies
# the DOS-7 SQL backfills (mechanisms 1-8) directly via sqlite3, and
# writes a markdown diff report. Mechanism 9 (DismissedItem JSON-blob
# entries) is NOT applied here because it requires the Rust runtime;
# operators run that via the in-app cutover when DOS-7 ships. The
# report explicitly notes the m9 gap.
#
# Usage:
#   bash scripts/dos7-migration-walkthrough.sh --db <path>
#                                              [--apply]
#                                              [--out <markdown-path>]

set -euo pipefail

DB_PATH=""
APPLY=false
OUT_PATH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db)        DB_PATH="$2"; shift 2 ;;
    --apply)     APPLY=true; shift ;;
    --out)       OUT_PATH="$2"; shift 2 ;;
    -h|--help)
      sed -n '3,20p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "$DB_PATH" ]]; then
  echo "usage: $0 --db <path> [--apply] [--out <path>]" >&2
  exit 2
fi
if [[ ! -f "$DB_PATH" ]]; then
  echo "DB not found: $DB_PATH" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MIG_DIR="$REPO_ROOT/src-tauri/src/migrations"

snapshot() {
  local label="$1"
  local db="$2"
  echo "## ${label}"
  echo
  echo "Legacy dismissal tables (rows with dismissed semantics):"
  echo

  count=$(sqlite3 "$db" "SELECT count(*) FROM suppression_tombstones;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 1 \`suppression_tombstones\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM account_stakeholder_roles WHERE dismissed_at IS NOT NULL;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 2 \`account_stakeholder_roles.dismissed_at\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM email_dismissals;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 3 \`email_dismissals\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM meeting_entity_dismissals;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 4 \`meeting_entity_dismissals\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM linking_dismissals;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 5 \`linking_dismissals\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM briefing_callouts WHERE dismissed_at IS NOT NULL;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 6 \`briefing_callouts.dismissed_at\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM nudge_dismissals;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 7 \`nudge_dismissals\`: ${count} rows"

  count=$(sqlite3 "$db" "SELECT count(*) FROM triage_snoozes;" 2>/dev/null || echo "n/a")
  echo "- Mechanism 8 \`triage_snoozes\`: ${count} rows"

  echo
  if sqlite3 "$db" "SELECT count(*) FROM intelligence_claims LIMIT 1;" >/dev/null 2>&1; then
    total=$(sqlite3 "$db" "SELECT count(*) FROM intelligence_claims WHERE claim_state = 'tombstoned';")
    echo "\`intelligence_claims\` tombstoned total: ${total}"
    echo
    echo "By backfill mechanism:"
    for prefix in m1 m2 m3 m4 m5 m6 m7 m8 m9; do
      pc=$(sqlite3 "$db" "SELECT count(*) FROM intelligence_claims WHERE id LIKE '${prefix}-%';")
      echo "- mechanism ${prefix}: ${pc} claims"
    done
  else
    echo "\`intelligence_claims\` does not exist yet — DOS-7 schema migration has not run."
  fi
  echo
}

if ! $APPLY; then
  snapshot "Snapshot — read-only" "$DB_PATH"
  exit 0
fi

if [[ -z "$OUT_PATH" ]]; then
  TS="$(date -u +%Y%m%dT%H%M%SZ)"
  OUT_DIR="$REPO_ROOT/.docs/plans/wave-W3"
  mkdir -p "$OUT_DIR"
  OUT_PATH="$OUT_DIR/dos-7-migration-walkthrough-${TS}.md"
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
WORK_DB="$WORK_DIR/dos7-walkthrough.sqlite"
cp "$DB_PATH" "$WORK_DB"

{
  echo "# DOS-7 migration walkthrough"
  echo
  echo "Generated $(date -u +%Y-%m-%dT%H:%M:%SZ) on a copy of \`${DB_PATH}\`."
  echo
  echo "Source DB: \`${DB_PATH}\`"
  echo "Working copy: \`${WORK_DB}\` (destroyed on exit)"
  echo
  snapshot "BEFORE" "$WORK_DB"

  echo "## Cutover"
  echo
  echo "Applying SQL backfill migrations (mechanisms 1-8) directly via \`sqlite3\`:"
  echo

  applied_count=0
  for sql_file in "$MIG_DIR/130_dos_7_claims_backfill_a1.sql" \
                  "$MIG_DIR/131_dos_7_claims_backfill_a2.sql"; do
    if [[ -f "$sql_file" ]]; then
      base=$(basename "$sql_file")
      echo "- Applied \`${base}\`"
      sqlite3 "$WORK_DB" < "$sql_file"
      applied_count=$((applied_count + 1))
    else
      echo "- MISSING: \`${sql_file}\`"
    fi
  done

  echo
  echo "Applied ${applied_count} SQL migration file(s)."
  echo
  echo "**Mechanism 9 (DismissedItem JSON-blob) is NOT applied by this script** — it requires the Rust runtime's \`run_dos7_cutover\` hook reading per-entity \`intelligence.json\` files. The in-app cutover handles m9 alongside the SQL backfills when DOS-7 ships. This script demonstrates the SQL-side transformation only."
  echo
  snapshot "AFTER" "$WORK_DB"

  echo "## Notes"
  echo
  echo "- Backfill rows are identified by \`id\` prefix: \`m1-\` through \`m9-\` map to the 9 dismissal mechanisms in DOS-7's plan."
  echo "- \`claim_state='tombstoned'\` + \`data_source='legacy_dismissal'\` distinguishes backfill claims from runtime-written claims."
  echo "- Duplicate-pair handling (mechanisms 4 + 5) records loser rows as \`claim_corroborations\` with \`source_mechanism='..._dup'\`."
  echo "- This walkthrough is read-only on the source DB; the working copy is destroyed when the script exits."
} > "$OUT_PATH"

echo "Walkthrough report written to: $OUT_PATH"
