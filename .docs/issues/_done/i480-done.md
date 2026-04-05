# I480 — GleanContextProvider + Cache + Migration

**Status:** Done
**Priority:** P1
**Version:** 0.15.2
**Area:** Backend / Connectors + Intelligence
**ADR:** 0096

## Summary

Implement `GleanContextProvider` as a second `ContextProvider` implementation that fetches entity context from Glean's MCP-compatible API. Includes a two-layer cache (DashMap in-memory + SQLite persistent) with TTL-based invalidation, and a DB migration for cache and config storage. When Glean is unreachable, falls back gracefully to local context.

## Acceptance Criteria

### GleanMcpClient

1. `context_provider/glean.rs` exists with `GleanMcpClient` struct implementing HTTP JSON-RPC calls to Glean.
2. Client supports `search`, `search_people`, and `read_document` MCP tool calls.
3. `Accept: application/json, text/event-stream` header is included on all requests (MCP compatibility).
4. HTTP errors and timeouts do not propagate — Glean outage results in graceful fallback to local context, not a crash or empty context.

### Cache layer

5. `context_provider/cache.rs` exists with DashMap-backed in-memory cache + SQLite persistence.
6. TTL-based invalidation: documents 1 hour, profiles 24 hours, org graph 4 hours.
7. Cache is warm on second request for the same entity — no redundant Glean API calls within TTL window.
8. SQLite persistence survives app restarts — cached data is available immediately on relaunch without hitting Glean.

### Migration

9. Migration 052 adds `glean_document_cache` and `context_mode_config` tables.
10. Migration runs cleanly on existing databases. `cargo test` migration tests pass.

### Two-phase gather

11. Context gathering runs in two phases: Phase A (local DB, milliseconds) and Phase B (Glean network, 200-2000ms).
12. Phase A completes and is usable even if Phase B fails or times out.

### Context merging strategies

13. Additive strategy: Glean context is merged on top of local context. Local data is preserved; Glean adds net-new fields.
14. Governed strategy: For overlapping fields, Glean context replaces local context. Glean is the authoritative source.
15. Strategy selection is persisted in `context_mode_config` and respected on every gather call.

### Fallback behavior

16. If Glean endpoint is unreachable, `GleanContextProvider` logs a warning and returns local-only context. No error propagates to the caller.
17. If Glean returns partial data (some calls succeed, some fail), the successful data is merged and partial failure is logged.

### ADR

18. ADR-0096 exists at `.docs/decisions/` documenting the Glean context provider design.

### Tests

19. `cargo test` passes.
20. `cargo clippy -- -D warnings` passes.

## Files

### New
- `src-tauri/src/context_provider/glean.rs` — `GleanMcpClient`, `GleanContextProvider`
- `src-tauri/src/context_provider/cache.rs` — DashMap + SQLite cache with TTL

### Modified
- `src-tauri/src/context_provider/mod.rs` — register `GleanContextProvider`
- `src-tauri/src/migrations.rs` — migration 052
- `src-tauri/src/lib.rs` — initialize Glean provider when config exists in DB

## Notes

- Glean MCP endpoint is organization-specific. Users provide their endpoint URL and OAuth token in Settings (I481).
- DashMap chosen over `HashMap<RwLock>` for lock-free concurrent reads — context gathering may run from multiple async tasks.
