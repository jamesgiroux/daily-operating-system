# Module Map

> Rust backend module inventory (`src-tauri/src/`).
> **Auto-generated:** 2026-05-09 by `.docs/generators/gen-module-map.sh`

**314** Rust files across **37** module directories and **45** standalone modules.

---

## Module Directories

| Module | Files | Public Fns | Purpose |
|--------|-------|-----------|---------|
| `abilities/` | 1 | 0 | Ability substrate modules. |
| `bin/` | 3 | 0 | Binary entry points |
| `bridges/` | 6 | 1 | Runtime-surface bridges for ability invocation. |
| `clay/` | 5 | 6 | Clay.earth MCP integration for contact and company enrichment. |
| `commands/` | 12 | 399 | Tauri IPC command handlers |
| `context_provider/` | 4 | 2 | Context provider abstraction for dual-mode operation (ADR-0095). |
| `db/` | 26 | 28 | SQLite database modules |
| `devtools/` | 1 | 11 | Dev tools for scenario switching and mock data. |
| `glean/` | 3 | 10 | Glean OAuth module — MCP OAuth discovery + DCR for Glean's MCP server. |
| `google_api/` | 6 | 27 | Native Google API client (ADR-0049: Eliminate Python runtime) |
| `google_drive/` | 4 | 9 | Google Drive integration for DailyOS. |
| `granola/` | 4 | 8 | Granola integration for local cache transcript sync. |
| `gravatar/` | 4 | 11 | Gravatar MCP server integration for avatar and profile enrichment. |
| `harness/` | 8 | 33 | — |
| `hygiene/` | 6 | 12 | Proactive intelligence maintenance (- ADR-0058). |
| `intelligence/` | 20 | 71 | Intelligence lifecycle, enrichment orchestration |
| `linear/` | 4 | 3 | Linear issue tracker integration. |
| `mcp/` | 1 | 6 | — |
| `migrations/` | 1 | 0 | SQL schema migrations |
| `oauth/` | 1 | 11 | Shared OAuth2 primitives used by Google and Glean consent flows. |
| `observability/` | 1 | 0 | — |
| `prepare/` | 9 | 24 | Phase 1 preparation operations (ADR-0049: Eliminate Python runtime). |
| `presets/` | 4 | 7 | — |
| `proactive/` | 4 | 12 | Proactive surfacing engine. |
| `processor/` | 11 | 36 | Inbox file processing pipeline. |
| `queries/` | 4 | 8 | — |
| `quill/` | 5 | 12 | Quill MCP client integration for automatic transcript sync. |
| `reports/` | 11 | 38 | Report infrastructure for v0.15.0. |
| `self_healing/` | 6 | 19 | Intelligence self-healing (–). |
| `services/` | 63 | 369 | ServiceLayer — mandatory mutation boundary |
| `signals/` | 22 | 58 | Universal signal bus for intelligence fusion (ADR-0080 Phase 2). |
| `workflow/` | 10 | 46 | Workflow definitions |

## Standalone Modules

| Module | Lines | Public Fns | Purpose |
|--------|-------|-----------|---------|
| `accounts.rs` | 1780 | 10 | Account workspace file I/O (ADR-0047). |
| `action_status.rs` | 96 | 2 | — |
| `activity.rs` | 187 | 3 | User activity monitoring for background task throttling. |
| `audit_log.rs` | 469 | 4 | Tamper-evident audit log for enterprise observability (ADR-0094). |
| `audit.rs` | 150 | 2 | Audit trail for AI-generated data. |
| `backfill_meetings.rs` | 456 | 1 | — |
| `calendar_merge.rs` | 291 | 1 | Calendar hybrid overlay merge (ADR-0032) |
| `capture.rs` | 402 | 1 | Post-meeting capture state machine |
| `commands.rs` | 85 | 0
0 | Legacy monolith command handler (being decomposed) |
| `connectivity.rs` | 151 | 3 | Connectivity tracking and sync freshness. |
| `db_backup.rs` | 650 | 8 | SQLite backup and rebuild-from-filesystem (ADR-0048) |
| `db_service.rs` | 654 | 3 | Unified async/sync database connection pool (DOS-* DbService refactor). |
| `demo.rs` | 1002 | 6 | Production demo data for first-run experience. |
| `embeddings.rs` | 315 | 3 | Local semantic search (nomic-embed-text) |
| `enrichment.rs` | 494 | 1 | Unified enrichment processor. |
| `entity_io.rs` | 166 | 5 | Shared entity I/O helpers. |
| `entity.rs` | 61 | 0
0 | Profile-agnostic tracked entity abstraction (ADR-0045). |
| `error.rs` | 159 | 0
0 | Error types for workflow execution |
| `executor.rs` | 1598 | 1 | Workflow execution engine |
| `export.rs` | 300 | 1 | User data export — ZIP file with human-readable JSON per domain. |
| `focus_capacity.rs` | 423 | 1 | — |
| `focus_prioritization.rs` | 398 | 1 | — |
| `google.rs` | 1829 | 5 | Google authentication and calendar polling |
| `helpers.rs` | 493 | 12 | — |
| `intel_queue.rs` | 5811 | 8 | Background intelligence enrichment queue. |
| `json_loader.rs` | 426 | 1 | JSON data loader with markdown fallback |
| `latency.rs` | 186 | 3 | Lightweight in-memory latency rollups for hot command diagnostics. |
| `lib.rs` | 1101 | 1 | App setup, command registration, plugin init |
| `meeting_prep_queue.rs` | 1068 | 7 | Background meeting prep queue. |
| `migrations.rs` | 2958 | 1 | Schema migration framework (ADR-0071). |
| `notification.rs` | 262 | 6 | Native notification wrapper |
| `parser.rs` | 2404 | 14 | Structured data parsing |
| `people.rs` | 667 | 7 | People workspace file I/O (ADR-0047). |
| `privacy.rs` | 98 | 2 | Privacy controls — data summary, clear intelligence, delete all. |
| `projects.rs` | 905 | 9 | Project workspace file I/O (ADR-0047). |
| `pty.rs` | 1300 | 5 | PTY Manager for Claude Code subprocess management |
| `release_gate.rs` | 1737 | 7 | — |
| `risk_briefing.rs` | 612 | 5 | Risk Briefing generation for at-risk accounts. |
| `scheduler.rs` | 778 | 2 | Scheduler for cron-based workflow execution |
| `state.rs` | 2069 | 13 | AppState — DB, PTY, config |
| `task_supervisor.rs` | 34 | 1 | — |
| `types.rs` | 3365 | 7 | Shared type definitions |
| `util.rs` | 1267 | 28 | — |
| `watcher.rs` | 969 | 1 | File watcher for _inbox/ directory |

## Cross-Module Dependencies

| Module | Depends On |
|--------|-----------|
| `commands/` |                     intel_queue,         intel_queue,     context_provider,     db,     glean,     intelligence,     processor,     services, abilities, bridges, services, state, types |
| `services/` |         db,         intelligence,     abilities,     db,     google_api,     intel_queue,     intelligence,     reports,     signals,     state, abilities, action_status, commands, db, embeddings, google_api, intel_queue, intelligence, json_loader, linear, parser, pty, pub abilities, reports, signals, state, types |
| `signals/` |             db,         presets,     db,     google_api,     state, db, embeddings, entity, helpers, prepare, services, types |
| `intelligence/` |         intel_queue,     db,     services,     signals,     types, accounts, context_provider, db, embeddings, error, helpers, presets, pty, signals, state, types, util |
| `prepare/` |     db,     entity,     helpers, db, embeddings, entity, error, google_api, helpers, presets, pty, signals, state, types |
| `reports/` | context_provider, db, embeddings, intelligence, pty, types |
| `db/` |     intelligence, entity, google_api, pub abilities, pub(crate) entity, pub(crate) types, types |

