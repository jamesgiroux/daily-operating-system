#!/bin/bash
# Generates .docs/architecture/MODULE-MAP.md from Rust source structure.
# Maps all modules, their public functions, and inter-module dependencies.

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/src-tauri/src"
OUT="$ROOT/.docs/architecture/MODULE-MAP.md"

RS_FILES=$(find "$SRC" -name "*.rs" -not -name "*test*" | wc -l | tr -d ' ')
DIRS=$(find "$SRC" -type d -mindepth 1 | wc -l | tr -d ' ')
STANDALONE=$(ls "$SRC"/*.rs 2>/dev/null | wc -l | tr -d ' ')

{
  echo "# Module Map"
  echo ""
  echo "> Rust backend module inventory (\`src-tauri/src/\`)."
  echo "> **Auto-generated:** $(date +%Y-%m-%d) by \`.docs/generators/gen-module-map.sh\`"
  echo ""
  echo "**$RS_FILES** Rust files across **$DIRS** module directories and **$STANDALONE** standalone modules."
  echo ""
  echo "---"
  echo ""

  # Module directories
  echo "## Module Directories"
  echo ""
  echo "| Module | Files | Public Fns | Purpose |"
  echo "|--------|-------|-----------|---------|"

  for dir in $(find "$SRC" -type d -mindepth 1 -maxdepth 1 | sort); do
    modname=$(basename "$dir")
    file_count=$(find "$dir" -name "*.rs" | wc -l | tr -d ' ')
    pub_fns=$(grep -rch "^pub fn \|^pub async fn " "$dir" 2>/dev/null | awk '{s+=$1}END{print s+0}')

    # Extract purpose from mod.rs doc comment
    purpose=""
    if [ -f "$dir/mod.rs" ]; then
      purpose=$(head -5 "$dir/mod.rs" | grep "^//!" | head -1 | sed 's/^\/\/! *//' || true)
    fi
    if [ -z "$purpose" ]; then
      case "$modname" in
        bin) purpose="Binary entry points" ;;
        commands) purpose="Tauri IPC command handlers" ;;
        db) purpose="SQLite database modules" ;;
        services) purpose="ServiceLayer — mandatory mutation boundary" ;;
        signals) purpose="Signal bus, propagation, feedback" ;;
        intelligence) purpose="Intelligence lifecycle, enrichment orchestration" ;;
        prepare) purpose="Daily briefing preparation pipeline" ;;
        processor) purpose="Inbox file processing and AI features" ;;
        workflow) purpose="Multi-step AI workflow orchestration" ;;
        reports) purpose="Report generation (Health, EBR, SWOT, etc.)" ;;
        google_api) purpose="Google Calendar and Gmail API" ;;
        google_drive) purpose="Google Drive document import" ;;
        glean) purpose="Glean enterprise knowledge integration" ;;
        linear) purpose="Linear issue tracker integration" ;;
        oauth) purpose="OAuth flow management" ;;
        clay) purpose="Clay contact enrichment via Smithery" ;;
        self_healing) purpose="Proactive self-healing and recovery" ;;
        context_provider) purpose="Intelligence context providers" ;;
        devtools) purpose="Developer tools and mock data" ;;
        migrations) purpose="SQL schema migrations" ;;
        proactive) purpose="Proactive intelligence detection" ;;
        *) purpose="—" ;;
      esac
    fi

    echo "| \`$modname/\` | $file_count | $pub_fns | $purpose |"
  done

  echo ""
  echo "## Standalone Modules"
  echo ""
  echo "| Module | Lines | Public Fns | Purpose |"
  echo "|--------|-------|-----------|---------|"

  for file in $(ls "$SRC"/*.rs 2>/dev/null | sort); do
    modname=$(basename "$file" .rs)
    [ "$modname" = "main" ] && continue
    lines=$(wc -l < "$file" | tr -d ' ')
    pub_fns=$(grep -c "^pub fn \|^pub async fn " "$file" 2>/dev/null || echo "0")

    purpose=$(head -5 "$file" | grep "^//!" | head -1 | sed 's/^\/\/! *//' || true)
    if [ -z "$purpose" ]; then
      case "$modname" in
        lib) purpose="App setup, command registration, plugin init" ;;
        commands) purpose="Legacy monolith command handler (being decomposed)" ;;
        state) purpose="AppState — DB, PTY, config" ;;
        types) purpose="Shared type definitions" ;;
        error) purpose="Error types" ;;
        scheduler) purpose="Background task scheduling" ;;
        executor) purpose="Task execution engine" ;;
        pty) purpose="Claude Code subprocess management" ;;
        intel_queue) purpose="Background intelligence processing (PTY)" ;;
        meeting_prep_queue) purpose="Meeting briefing assembly (mechanical)" ;;
        enrichment) purpose="Person enrichment pipeline (Clay, Gravatar, AI)" ;;
        embeddings) purpose="Local semantic search (nomic-embed-text)" ;;
        parser) purpose="Structured data parsing" ;;
        json_loader) purpose="JSON file loading utilities" ;;
        google) purpose="Google OAuth + API client" ;;
        audit_log) purpose="Tamper-evident activity log" ;;
        risk_briefing) purpose="Risk briefing generation" ;;
        entity) purpose="Entity CRUD operations" ;;
        entity_io) purpose="Entity import/export" ;;
        capture) purpose="Meeting outcome capture" ;;
        connectivity) purpose="Network connectivity checks" ;;
        activity) purpose="User activity tracking" ;;
        accounts) purpose="Account operations" ;;
        *) purpose="—" ;;
      esac
    fi

    echo "| \`$modname.rs\` | $lines | $pub_fns | $purpose |"
  done

  echo ""
  echo "## Cross-Module Dependencies"
  echo ""
  echo "| Module | Depends On |"
  echo "|--------|-----------|"

  for dir in commands services signals intelligence prepare reports db; do
    if [ -d "$SRC/$dir" ]; then
      deps=$(grep -rh "use crate::" "$SRC/$dir/" 2>/dev/null | sed 's/use crate::\([a-z_]*\).*/\1/' | sort -u | grep -v "$dir" | tr '\n' ', ' | sed 's/,$//' | sed 's/,/, /g' || true)
      [ -n "$deps" ] && echo "| \`$dir/\` | $deps |"
    fi
  done || true

  echo ""
} > "$OUT"

echo "Generated MODULE-MAP.md: $RS_FILES files, $DIRS directories"
