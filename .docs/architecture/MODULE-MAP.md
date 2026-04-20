# Module Map

> Rust backend module inventory (`src-tauri/src/`).
> **Auto-generated:** 2026-04-19 by `.docs/generators/gen-module-map.sh`

**241** Rust files across **29** module directories and **44** standalone modules.

---

## Module Directories

| Module | Files | Public Fns | Purpose |
|--------|-------|-----------|---------|
| `clay/` | 5 | 6 | Clay.earth MCP integration for contact and company enrichment (I228). |
| `commands/` | 10 | 378 | Tauri IPC command handlers |
| `context_provider/` | 4 | 2 | Context provider abstraction for dual-mode operation (ADR-0095). |
| `db/` | 22 | 25 | SQLite database modules |
| `devtools/` | 1 | 11 | Dev tools for scenario switching and mock data. |
| `glean/` | 3 | 10 | Glean OAuth module — MCP OAuth discovery + DCR for Glean's MCP server. |
| `google_api/` | 6 | 27 | Native Google API client (ADR-0049: Eliminate Python runtime) |
| `google_drive/` | 4 | 9 | Google Drive integration for DailyOS (I426). |
| `granola/` | 4 | 8 | Granola integration for local cache transcript sync (I226). |
| `gravatar/` | 4 | 11 | Gravatar MCP server integration for avatar and profile enrichment (I229). |
| `hygiene/` | 6 | 12 | Proactive intelligence maintenance (I145 -- ADR-0058). |
| `intelligence/` | 16 | 59 | Intelligence lifecycle, enrichment orchestration |
| `linear/` | 4 | 3 | Linear issue tracker integration (I346). |
| `mcp/` | 1 | 0 | — |
| `migrations/` | 0 | 0 | SQL schema migrations |
| `oauth/` | 1 | 11 | Shared OAuth2 primitives used by Google and Glean consent flows. |
| `prepare/` | 9 | 24 | Phase 1 preparation operations (ADR-0049: Eliminate Python runtime). |
| `presets/` | 4 | 6 | — |
| `proactive/` | 4 | 12 | Proactive surfacing engine (I260). |
| `processor/` | 11 | 35 | Inbox file processing pipeline. |
| `queries/` | 4 | 8 | — |
| `quill/` | 5 | 12 | Quill MCP client integration for automatic transcript sync. |
| `reports/` | 11 | 38 | Report infrastructure for v0.15.0 (I397). |
| `self_healing/` | 6 | 19 | Intelligence self-healing (I406–I410). |
| `services/` | 23 | 267 | ServiceLayer — mandatory mutation boundary |
| `signals/` | 20 | 53 | Universal signal bus for intelligence fusion (I306 / ADR-0080 Phase 2). |
| `workflow/` | 10 | 46 | Workflow definitions |

## Standalone Modules

| Module | Lines | Public Fns | Purpose |
|--------|-------|-----------|---------|
| `accounts.rs` | 1868 | 11 | Account workspace file I/O (I72 / ADR-0047). |
| `action_status.rs` | 96 | 2 | — |
| `activity.rs` | 187 | 3 | User activity monitoring for background task throttling. |
| `audit.rs` | 150 | 2 | Audit trail for AI-generated data (I297). |
| `audit_log.rs` | 453 | 4 | Tamper-evident audit log for enterprise observability (I471, ADR-0094). |
| `backfill_meetings.rs` | 456 | 1 | — |
| `calendar_merge.rs` | 290 | 1 | Calendar hybrid overlay merge (ADR-0032) |
| `capture.rs` | 386 | 1 | Post-meeting capture state machine |
| `commands.rs` | 82 | 0
0 | Legacy monolith command handler (being decomposed) |
| `connectivity.rs` | 151 | 3 | I428: Connectivity tracking and sync freshness. |
| `db_backup.rs` | 551 | 8 | SQLite backup and rebuild-from-filesystem (I76 / ADR-0048) |
| `db_service.rs` | 515 | 0
0 | Async database service using tokio-rusqlite. |
| `demo.rs` | 994 | 6 | Production demo data for first-run experience (I56). |
| `embeddings.rs` | 315 | 3 | Local semantic search (nomic-embed-text) |
| `enrichment.rs` | 475 | 1 | Unified enrichment processor. |
| `entity.rs` | 61 | 0
0 | Profile-agnostic tracked entity abstraction (ADR-0045). |
| `entity_io.rs` | 166 | 5 | Shared entity I/O helpers (I290). |
| `error.rs` | 139 | 0
0 | Error types for workflow execution |
| `executor.rs` | 1458 | 1 | Workflow execution engine |
| `export.rs` | 300 | 1 | I429: User data export — ZIP file with human-readable JSON per domain. |
| `focus_capacity.rs` | 423 | 1 | — |
| `focus_prioritization.rs` | 398 | 1 | — |
| `google.rs` | 1728 | 5 | Google authentication and calendar polling |
| `helpers.rs` | 492 | 12 | — |
| `intel_queue.rs` | 2878 | 4 | Background intelligence enrichment queue (I132). |
| `json_loader.rs` | 615 | 3 | JSON data loader with markdown fallback |
| `latency.rs` | 186 | 3 | Lightweight in-memory latency rollups for hot command diagnostics (I197). |
| `lib.rs` | 886 | 1 | App setup, command registration, plugin init |
| `meeting_prep_queue.rs` | 984 | 7 | Background meeting prep queue. |
| `migrations.rs` | 1737 | 1 | Schema migration framework (ADR-0071). |
| `notification.rs` | 252 | 6 | Native notification wrapper |
| `parser.rs` | 2404 | 14 | Structured data parsing |
| `people.rs` | 639 | 7 | People workspace file I/O (I51 / ADR-0047). |
| `privacy.rs` | 97 | 2 | I430: Privacy controls — data summary, clear intelligence, delete all. |
| `projects.rs` | 857 | 9 | Project workspace file I/O (I50 / ADR-0047). |
| `pty.rs` | 994 | 2 | PTY Manager for Claude Code subprocess management |
| `risk_briefing.rs` | 596 | 5 | Risk Briefing generation for at-risk accounts. |
| `scheduler.rs` | 748 | 2 | Scheduler for cron-based workflow execution |
| `state.rs` | 1414 | 12 | AppState — DB, PTY, config |
| `task_supervisor.rs` | 34 | 1 | — |
| `types.rs` | 3251 | 7 | Shared type definitions |
| `util.rs` | 1267 | 28 | — |
| `watcher.rs` | 882 | 1 | File watcher for _inbox/ directory |

## Cross-Module Dependencies

| Module | Depends On |
|--------|-----------|
| `commands/` |                     intel_queue,         intel_queue,     context_provider,     db,     glean,     intelligence,     processor, services, types |
| `services/` |         intelligence,     db,     google_api,     intel_queue,     intelligence,     reports,     signals,     state, action_status, commands, db, embeddings, intelligence, json_loader, linear, parser, pty, reports, signals, state, types |
| `signals/` |             db,     db,     entity,     google_api,     prepare, db, embeddings, entity, helpers, prepare, state, types |
| `intelligence/` |         intel_queue,     db,     signals,     types, accounts, context_provider, db, embeddings, error, helpers, signals, state, types, util |
| `prepare/` |     db,     entity, db, embeddings, entity, error, google_api, helpers, pty, signals, state, types |
| `reports/` | context_provider, db, embeddings, intelligence, pty, types |
| `db/` | entity, google_api, pub(crate) entity, pub(crate) types, types |

## v1.4.0 Substrate Modules

Most of these do not exist in code yet; paths below are where they land per the owning ADRs. This section will update incrementally as implementation progresses.

### New module directories

| Path | Purpose | ADR |
|---|---|---|
| `src-tauri/src/abilities/` | Runtime contract layer | [0102](../decisions/0102-abilities-as-runtime-contract.md) |
| `src-tauri/src/abilities/mod.rs` | Registry + root | [0102 §2](../decisions/0102-abilities-as-runtime-contract.md) |
| `src-tauri/src/abilities/context.rs` | `AbilityContext` | [0102 §5](../decisions/0102-abilities-as-runtime-contract.md) |
| `src-tauri/src/abilities/result.rs` | `AbilityResult<T>`, `AbilityError`, `AbilityOutput<T>` | [0102 §2](../decisions/0102-abilities-as-runtime-contract.md) |
| `src-tauri/src/abilities/provenance.rs` | Provenance envelope types | [0105](../decisions/0105-provenance-as-first-class-output.md) |
| `src-tauri/src/abilities/read/` | Read abilities | [0102 §3](../decisions/0102-abilities-as-runtime-contract.md) |
| `src-tauri/src/abilities/transform/` | Transform abilities | [0102 §3](../decisions/0102-abilities-as-runtime-contract.md) |
| `src-tauri/src/abilities/publish/` | Publish abilities (publish_to_p2 first) | [0117](../decisions/0117-publish-boundary-pencil-and-pen.md) |
| `src-tauri/src/abilities/maintenance/` | Maintenance abilities | [0103](../decisions/0103-maintenance-ability-safety-constraints.md) |
| `src-tauri/src/abilities/evaluator.rs` | Runtime evaluator pass | [0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md) |
| `src-tauri/src/scoring/` | Unified scoring substrate | [0114](../decisions/0114-scoring-unification.md) |
| `src-tauri/src/scoring/factors.rs` | Pure-function factor primitives | [0114 §1](../decisions/0114-scoring-unification.md) |
| `src-tauri/src/scoring/extract/` | Per-surface input extractors | [0114 §2](../decisions/0114-scoring-unification.md) |
| `src-tauri/src/scoring/extract/trust.rs` | Trust Compiler input extraction | [DOS-5](https://linear.app/a8c/issue/DOS-5) |
| `src-tauri/src/signals/types.rs` | `SignalType` enum (Phase 0 prerequisite) | [0115 R1.1](../decisions/0115-signal-granularity-audit.md#r11-signaltype-enum-is-a-prerequisite-not-a-feature-of-this-adr) |
| `src-tauri/src/signals/policy_registry.rs` | Compile-time policy registry | [0115 §3](../decisions/0115-signal-granularity-audit.md) |
| `src-tauri/src/signals/invalidation_jobs.rs` | Durable job model + worker | [0115 §5](../decisions/0115-signal-granularity-audit.md) |
| `src-tauri/src/db/key_provider.rs` | `DbKeyProvider` trait seam | [0116 R1.1](../decisions/0116-tenant-control-plane-boundary.md#r11-current-signature--correct-the-seam-shape) |
| `src-tauri/src/services/context.rs` | `ServiceContext` struct ([DOS-209](https://linear.app/a8c/issue/DOS-209) Phase 0) | [0104](../decisions/0104-execution-mode-and-mode-aware-services.md) |
| `src-tauri/src/services/claims.rs` | Claim service (propose/commit/retract) | [0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md) |
| `src-tauri/src/intelligence/provider.rs` | `IntelligenceProvider` trait ([DOS-259](https://linear.app/a8c/issue/DOS-259)) | [0091](../decisions/0091-intelligence-provider-abstraction.md) + [0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) |
| `src-tauri/src/publish/` | Pencil/Pen publish framework | [0117](../decisions/0117-publish-boundary-pencil-and-pen.md) |
| `src-tauri/src/publish/outbox.rs` | Outbox worker + idempotency | [0117 §3](../decisions/0117-publish-boundary-pencil-and-pen.md) |
| `src-tauri/src/publish/clients/` | `DestinationClient` implementations | [0117 §5](../decisions/0117-publish-boundary-pencil-and-pen.md) |
| `src-tauri/src/publish/confirmation_broker.rs` | One-time-use `ConfirmationToken` | [0117 R1.9](../decisions/0117-publish-boundary-pencil-and-pen.md#r19-confirmation-token-security) |
| `src-tauri/src/observability/` | Observability contract + NDJSON subscriber | [0120](../decisions/0120-observability-contract.md) |

### Existing modules gaining v1.4.0 responsibilities

| Path | What changes | ADR |
|---|---|---|
| `src-tauri/src/signals/bus.rs` | Consolidated `emit_signal`; legacy variants migrated away | [0115 §4](../decisions/0115-signal-granularity-audit.md) |
| `src-tauri/src/intelligence/consistency.rs` | Consumed by runtime evaluator as failure-suspicion trigger | [0119 §4](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md) |
| `src-tauri/src/intelligence/validation.rs` | Consumed by evaluator + evaluation harness | [0110](../decisions/0110-evaluation-harness-for-abilities.md) + [0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md) |
| `src-tauri/src/intelligence/dimension_prompts.rs` | Prompts gain `prompt_template_id` + `prompt_template_version` | [0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) |
| `src-tauri/src/intelligence/health_scoring.rs` | Phase 2 (v1.6.0): migrates to `scoring::factors`; `compute_account_health` split pure/write | [0114 R1.1/R1.2](../decisions/0114-scoring-unification.md) |
| `src-tauri/src/db/encryption.rs` | Becomes `LocalKeychain` impl of `DbKeyProvider` | [0116 R1.1](../decisions/0116-tenant-control-plane-boundary.md#r11-current-signature--correct-the-seam-shape) |
| `src-tauri/src/db/core.rs` + `db_service.rs` | `open_with_provider()` alongside zero-arg defaults | [0116 R1.3](../decisions/0116-tenant-control-plane-boundary.md#r13-di-migration-is-real--acknowledge-and-scope) |
| `src-tauri/src/services/*.rs` | Every mutation function takes `&ServiceContext` | [0104](../decisions/0104-execution-mode-and-mode-aware-services.md) |
| `src-tauri/src/services/intelligence.rs` | PTY orchestration behind `IntelligenceProvider::PtyClaudeCode` | [DOS-259](https://linear.app/a8c/issue/DOS-259) |
| `src-tauri/src/intelligence/glean_provider.rs` | Implements `IntelligenceProvider` | [DOS-259](https://linear.app/a8c/issue/DOS-259) |
| `src-tauri/src/migrations.rs` + `migrations/` | ~10 new migrations for substrate tables | Multiple |

### Brownfield-as-greenfield: modules for aggressive replacement

Per founder D1 (2026-04-20):

| Target | Action | Rationale |
|---|---|---|
| 318 legacy signal emit call sites | Migrated to `emit_signal(signal)`; legacy variants deleted | [0115 §4](../decisions/0115-signal-granularity-audit.md) |
| ~44 files with scattered SQL mutations | Consolidated through `services/`; CI-blocked outside | [0101](../decisions/0101-service-boundary-enforcement.md) |
| Direct `Utc::now()` / `rand::thread_rng()` | Replaced with `ctx.clock` / `ctx.rng`; CI-linted | [0104](../decisions/0104-execution-mode-and-mode-aware-services.md) |
| 27 JSON columns on `entity_assessment` | Wholesale replaced by `intelligence_claims`; destructive migration | [0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md) |
| `suppression_tombstones` + `DismissedItem` + `account_stakeholder_roles.dismissed_at` | Retired; tombstones in claims | [0113 R1.3](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r13-consolidate-existing-tombstone-infrastructure-dont-duplicate-it) |

### Process-wide singletons to be wrapped for Evaluate mode isolation

| Singleton | Location | Plan |
|---|---|---|
| `LAST_TRANSCRIPT_NOTIFICATION` | `notification.rs` | Wrap in `ServiceContext` |
| `NODE_BINARY` / `NPX_BINARY` / `NPM_BINARY` | `util.rs` | Wrap in `ExternalClients` |
| `DISCOVERY_CACHE` | `intelligence/glean_provider.rs` | Move to `GleanProvider` instance state |
| `db::encryption::CACHED_KEY` | `db/encryption.rs` | Move to `LocalKeychain::cached_key` with `invalidate_cache()` | [0116 R1.1](../decisions/0116-tenant-control-plane-boundary.md#r11-current-signature--correct-the-seam-shape) |

