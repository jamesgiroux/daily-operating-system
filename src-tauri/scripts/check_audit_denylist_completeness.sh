#!/usr/bin/env bash
#
# AC-340.7 — denylist completeness CI gate.
#
# Pairs with AC-477.12 (allowlist-primary boundary). Every column added to
# an operational audit storage schema MUST be classified explicitly:
#
#   * named in AUDIT_ONLY_DENYLIST in
#     src-tauri/src/services/claim_receipt/boundary.rs, OR
#   * carry an inline `// receipt-safe: <reason>` justification comment on
#     the same SQL/Rust line as the column declaration.
#
# Pattern modeled on check_claim_writer_allowlist.sh.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  migrations_root="src-tauri/src/migrations"
  boundary_file="src-tauri/src/services/claim_receipt/boundary.rs"
else
  migrations_root="src/migrations"
  boundary_file="src/services/claim_receipt/boundary.rs"
fi

if [[ ! -d "$migrations_root" ]]; then
  echo "No migrations directory found at $migrations_root; skipping."
  exit 0
fi

if [[ ! -f "$boundary_file" ]]; then
  echo "Boundary file missing: $boundary_file" >&2
  exit 1
fi

# Migration files that declare or alter operational audit tables. We scope
# by table-name match across the migrations folder.
audit_tables='sensitivity_reveal_audit|audit_log|provenance_audit_storage|maintenance_audit'

# Migration files that mention one of these tables.
candidate_files="$(
  grep -rEli --include='*.sql' --include='*.rs' -- "($audit_tables)" "$migrations_root" 2>/dev/null \
    || true
)"

if [[ -z "$candidate_files" ]]; then
  echo "DOS-340 AC-340.7: no audit-table migrations present; nothing to classify."
  exit 0
fi

# Extract column names from CREATE TABLE / ALTER TABLE ... ADD COLUMN
# declarations inside the candidate files. SQLite identifier shape:
# leading ident, then whitespace, then a type keyword. We capture the
# column identifier on lines that look like column declarations.
violations=""
for file in $candidate_files; do
  # Skip non-sql files for the column scan; .rs migration drivers don't
  # declare columns inline (they invoke .sql files).
  [[ "$file" == *.sql ]] || continue

  # Match column declarations: lines that begin with whitespace, an
  # identifier, then a SQL type keyword. Avoid CHECK/CONSTRAINT/PRIMARY
  # KEY lines.
  while IFS= read -r line; do
    # Skip lines clearly NOT a column declaration.
    case "$line" in
      *CREATE\ TABLE*|*PRIMARY\ KEY*|*FOREIGN\ KEY*|*UNIQUE*|*CHECK*|*CONSTRAINT*) continue ;;
    esac
    # Pull the candidate column name = first identifier on the line.
    col="$(echo "$line" | sed -E 's/^[[:space:]]+//' | awk '{print $1}' | tr -d ',();')"
    [[ -n "$col" ]] || continue
    # Must look like a SQL identifier.
    [[ "$col" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    # Skip SQL keywords accidentally landing as $col.
    case "$(echo "$col" | tr '[:upper:]' '[:lower:]')" in
      create|table|if|not|exists|insert|update|delete|select|from|where|alter|drop|index|trigger|begin|end|values|into) continue ;;
    esac

    # Receipt-safe escape hatch.
    if echo "$line" | grep -q 'receipt-safe:'; then
      continue
    fi

    # Classified in the denylist?
    if grep -q "\"$col\"" "$boundary_file"; then
      continue
    fi

    # Lifecycle metadata columns common across audit tables get an
    # implicit pass — they're audit-table-housekeeping, not disclosure
    # surfaces (rowid/PK/timestamp shape). The list is intentionally
    # narrow; everything else must be explicitly classified.
    case "$col" in
      id|rowid|created_at|updated_at|recorded_at|inserted_at|version|schema_version) continue ;;
    esac

    violations+="${file}: unclassified audit column \"${col}\"\n"
  done < <(grep -nE '^[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]+(TEXT|INTEGER|REAL|BLOB|NUMERIC|BOOLEAN|TIMESTAMP|DATETIME|VARCHAR)' "$file" || true)
done

if [[ -n "$violations" ]]; then
  echo "DOS-340 AC-340.7: operational audit migration adds column(s) that are" >&2
  echo "neither in AUDIT_ONLY_DENYLIST nor marked '// receipt-safe: <reason>'." >&2
  echo >&2
  printf '%b' "$violations" >&2
  exit 1
fi

echo "DOS-340 AC-340.7: audit denylist completeness clean."
