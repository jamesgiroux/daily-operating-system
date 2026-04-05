#!/bin/bash
# Generates .docs/architecture/DATA-MODEL.md from SQL migration files.
# Replays CREATE TABLE / ALTER TABLE / CREATE INDEX to build current schema.

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MIG_DIR="$ROOT/src-tauri/src/migrations"
OUT="$ROOT/.docs/architecture/DATA-MODEL.md"

MIG_COUNT=$(ls "$MIG_DIR"/*.sql 2>/dev/null | wc -l | tr -d ' ')
FIRST_MIG=$(ls "$MIG_DIR"/*.sql 2>/dev/null | head -1 | xargs basename 2>/dev/null || echo "?")
LAST_MIG=$(ls "$MIG_DIR"/*.sql 2>/dev/null | tail -1 | xargs basename 2>/dev/null || echo "?")

# Temp file for table list
TMPFILE=$(mktemp)
trap "rm -f $TMPFILE" EXIT

# Pass 1: Find all CREATE TABLE statements (macOS-compatible grep)
for mig in "$MIG_DIR"/*.sql; do
  migname=$(basename "$mig" .sql)
  grep -i 'CREATE TABLE' "$mig" 2>/dev/null | grep -v '^\s*--' | sed -E 's/.*CREATE TABLE (IF NOT EXISTS )?`?([a-z_]+)`?.*/\2/' | while read -r table; do
    # Skip lines that don't look like table names
    echo "$table" | grep -qE '^[a-z_]+$' && echo "$table|$migname"
  done
done | sort -t'|' -k1,1 -u > "$TMPFILE"

{
  echo "# Data Model Reference"
  echo ""
  echo "**Auto-generated:** $(date +%Y-%m-%d) by \`.docs/generators/gen-data-model.sh\`"
  echo ""
  echo "**Database:** SQLite (SQLCipher-encrypted, WAL mode)"
  echo "**Migrations:** $MIG_COUNT files (\`$FIRST_MIG\` through \`$LAST_MIG\`)"
  echo "**DB modules:** \`src-tauri/src/db/\`"
  echo ""
  echo "---"
  echo ""

  # Summary table first
  echo "## Table Inventory"
  echo ""
  echo "| Table | Created In | Columns Added Later |"
  echo "|-------|-----------|-------------------|"

  while IFS='|' read -r table migname; do
    # Find ALTER TABLE ADD COLUMN for this table
    alters=$(grep -rli "ALTER TABLE.*$table.*ADD" "$MIG_DIR"/*.sql 2>/dev/null | xargs -I{} basename {} .sql 2>/dev/null | grep -v "$migname" | tr '\n' ', ' | sed 's/,$//' | sed 's/,/, /g' || true)
    [ -z "$alters" ] && alters="—"
    echo "| \`$table\` | \`$migname\` | $alters |"
  done < "$TMPFILE"

  echo ""
  echo "---"
  echo ""

  # Detailed table schemas
  echo "## Table Details"
  echo ""

  while IFS='|' read -r table migname; do
    echo "### \`$table\`"
    echo ""
    echo "**Created in:** \`$migname\`"
    echo ""

    # Extract columns from CREATE TABLE block
    has_cols=false
    for mig in "$MIG_DIR"/*.sql; do
      awk -v tbl="$table" '
        BEGIN { IGNORECASE=1; found=0 }
        tolower($0) ~ "create table.*" tolower(tbl) { found=1; next }
        found && /\);/ { found=0 }
        found && /^[[:space:]]+[a-z_]/ {
          line = $0
          gsub(/^[[:space:]]+/, "", line)
          gsub(/,[[:space:]]*$/, "", line)
          if (line !~ /^(PRIMARY|UNIQUE|FOREIGN|CHECK|CONSTRAINT)/i) {
            split(line, parts, " ")
            col = parts[1]
            gsub(/`/, "", col)
            type_rest = line
            sub(/^[^ ]+ /, "", type_rest)
            printf "| `%s` | %s |\n", col, type_rest
          }
        }
      ' "$mig" 2>/dev/null
    done | head -40 | {
      while IFS= read -r line; do
        if [ "$has_cols" = false ]; then
          echo "| Column | Definition |"
          echo "|--------|-----------|"
          has_cols=true
        fi
        echo "$line"
      done
    }

    # ALTER TABLE additions
    for mig in "$MIG_DIR"/*.sql; do
      mn=$(basename "$mig" .sql)
      grep -i "ALTER TABLE.*$table.*ADD" "$mig" 2>/dev/null | sed -E 's/.*ADD (COLUMN )?`?([a-z_]+)`?.*/\2/' | while read -r col; do
        [ -n "$col" ] && echo "- \`$col\` *(added in $mn)*"
      done
    done || true

    # Indexes
    indexes=$(grep -rhi "CREATE.*INDEX.*ON.*$table" "$MIG_DIR"/*.sql 2>/dev/null | sed -E 's/.*(idx_[a-z_]+).*/\1/' | sort -u | tr '\n' ', ' | sed 's/,$//' | sed 's/,/, /g' || true)
    if [ -n "$indexes" ]; then
      echo ""
      echo "**Indexes:** $indexes"
    fi

    echo ""
    echo "---"
    echo ""
  done < "$TMPFILE"

} > "$OUT"

TABLE_COUNT=$(wc -l < "$TMPFILE" | tr -d ' ')
echo "Generated DATA-MODEL.md: $TABLE_COUNT tables from $MIG_COUNT migrations"
