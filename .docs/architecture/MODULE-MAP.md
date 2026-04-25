# Module Map

> Rust backend module inventory (`src-tauri/src/`).
> **Auto-generated:** 2026-04-25 by `.docs/generators/gen-module-map.sh`

**269** Rust files across **31** module directories and **44** standalone modules.

---

## Module Directories

| Module | Files | Public Fns | Purpose |
|--------|-------|-----------|---------|
| `clay/` | 5 | 6 | Clay.earth MCP integration for contact and company enrichment (I228). |
| `commands/` | 10 | 399 | Tauri IPC command handlers |
| `context_provider/` | 4 | 2 | Context provider abstraction for dual-mode operation (ADR-0095). |
| `db/` | 23 | 26 | SQLite database modules |
| `devtools/` | 1 | 11 | Dev tools for scenario switching and mock data. |
| `glean/` | 3 | 10 | Glean OAuth module — MCP OAuth discovery + DCR for Glean's MCP server. |
| `google_api/` | 6 | 27 | Native Google API client (ADR-0049: Eliminate Python runtime) |
| `google_drive/` | 4 | 9 | Google Drive integration for DailyOS (I426). |
| `granola/` | 4 | 8 | Granola integration for local cache transcript sync (I226). |
| `gravatar/` | 4 | 11 | Gravatar MCP server integration for avatar and profile enrichment (I229). |
| `hygiene/` | 6 | 12 | Proactive intelligence maintenance (I145 -- ADR-0058). |
| `intelligence/` | 17 | 67 | Intelligence lifecycle, enrichment orchestration |
| `linear/` | 4 | 3 | Linear issue tracker integration (I346). |
| `mcp/` | 1 | 0 | — |
| `migrations/` | 0 | 0 | SQL schema migrations |
| `oauth/` | 1 | 11 | Shared OAuth2 primitives used by Google and Glean consent flows. |
| `prepare/` | 9 | 24 | Phase 1 preparation operations (ADR-0049: Eliminate Python runtime). |
| `presets/` | 4 | 7 | — |
| `proactive/` | 4 | 12 | Proactive surfacing engine (I260). |
| `processor/` | 11 | 35 | Inbox file processing pipeline. |
| `queries/` | 4 | 8 | — |
| `quill/` | 5 | 12 | Quill MCP client integration for automatic transcript sync. |
| `reports/` | 11 | 38 | Report infrastructure for v0.15.0 (I397). |
| `self_healing/` | 6 | 19 | Intelligence self-healing (I406–I410). |
| `services/` | 49 | 306 | ServiceLayer — mandatory mutation boundary |
| `signals/` | 20 | 53 | Universal signal bus for intelligence fusion (I306 / ADR-0080 Phase 2). |
| `workflow/` | 10 | 46 | Workflow definitions |

## Standalone Modules

| Module | Lines | Public Fns | Purpose |
|--------|-------|-----------|---------|
| `accounts.rs` | 1868 | 11 | Account workspace file I/O (I72 / ADR-0047). |
| `action_status.rs` | 96 | 2 | — |
| `activity.rs` | 187 | 3 | User activity monitoring for background task throttling. |
| `audit_log.rs` | 453 | 4 | Tamper-evident audit log for enterprise observability (I471, ADR-0094). |
| `audit.rs` | 150 | 2 | Audit trail for AI-generated data (I297). |
| `backfill_meetings.rs` | 456 | 1 | — |
| `calendar_merge.rs` | 291 | 1 | Calendar hybrid overlay merge (ADR-0032) |
| `capture.rs` | 386 | 1 | Post-meeting capture state machine |
| `commands.rs` | 81 | 0
0 | Legacy monolith command handler (being decomposed) |
| `connectivity.rs` | 151 | 3 | I428: Connectivity tracking and sync freshness. |
| `db_backup.rs` | 607 | 8 | SQLite backup and rebuild-from-filesystem (I76 / ADR-0048) |
| `db_service.rs` | 621 | 3 | Unified async/sync database connection pool (DOS-* DbService refactor). |
| `demo.rs` | 994 | 6 | Production demo data for first-run experience (I56). |
| `embeddings.rs` | 315 | 3 | Local semantic search (nomic-embed-text) |
| `enrichment.rs` | 475 | 1 | Unified enrichment processor. |
| `entity_io.rs` | 166 | 5 | Shared entity I/O helpers (I290). |
| `entity.rs` | 61 | 0
0 | Profile-agnostic tracked entity abstraction (ADR-0045). |
| `error.rs` | 159 | 0
0 | Error types for workflow execution |
| `executor.rs` | 1458 | 1 | Workflow execution engine |
| `export.rs` | 300 | 1 | I429: User data export — ZIP file with human-readable JSON per domain. |
| `focus_capacity.rs` | 423 | 1 | — |
| `focus_prioritization.rs` | 398 | 1 | — |
| `google.rs` | 1736 | 5 | Google authentication and calendar polling |
| `helpers.rs` | 492 | 12 | — |
| `intel_queue.rs` | 3242 | 4 | Background intelligence enrichment queue (I132). |
| `json_loader.rs` | 426 | 1 | JSON data loader with markdown fallback |
| `latency.rs` | 186 | 3 | Lightweight in-memory latency rollups for hot command diagnostics (I197). |
| `lib.rs` | 988 | 1 | App setup, command registration, plugin init |
| `meeting_prep_queue.rs` | 1010 | 7 | Background meeting prep queue. |
| `migrations.rs` | 2126 | 1 | Schema migration framework (ADR-0071). |
| `notification.rs` | 252 | 6 | Native notification wrapper |
| `parser.rs` | 2404 | 14 | Structured data parsing |
| `people.rs` | 639 | 7 | People workspace file I/O (I51 / ADR-0047). |
| `privacy.rs` | 97 | 2 | I430: Privacy controls — data summary, clear intelligence, delete all. |
| `projects.rs` | 857 | 9 | Project workspace file I/O (I50 / ADR-0047). |
| `pty.rs` | 1307 | 5 | PTY Manager for Claude Code subprocess management |
| `risk_briefing.rs` | 596 | 5 | Risk Briefing generation for at-risk accounts. |
| `scheduler.rs` | 748 | 2 | Scheduler for cron-based workflow execution |
| `state.rs` | 1577 | 13 | AppState — DB, PTY, config |
| `task_supervisor.rs` | 34 | 1 | — |
| `types.rs` | 3361 | 7 | Shared type definitions |
| `util.rs` | 1267 | 28 | — |
| `watcher.rs` | 882 | 1 | File watcher for _inbox/ directory |

## Cross-Module Dependencies

| Module | Depends On |
|--------|-----------|
| `commands/` |                     intel_queue,         intel_queue,     context_provider,     db,     glean,     intelligence,     processor, services, types |
| `services/` |         intelligence,     db,     google_api,     intel_queue,     intelligence,     reports,     signals,     state, action_status, commands, db, embeddings, google_api, intelligence, json_loader, linear, parser, pty, reports, signals, state, types |
| `signals/` |             db,         presets,     db,     google_api,     state, db, embeddings, entity, helpers, prepare, types |
| `intelligence/` |         intel_queue,     db,     services,     signals,     types, accounts, context_provider, db, embeddings, error, helpers, presets, signals, state, types, util |
| `prepare/` |     db,     entity,     helpers, db, embeddings, entity, error, google_api, helpers, presets, pty, signals, state, types |
| `reports/` | context_provider, db, embeddings, intelligence, pty, types |
| `db/` | entity, google_api, pub(crate) entity, pub(crate) types, types |

