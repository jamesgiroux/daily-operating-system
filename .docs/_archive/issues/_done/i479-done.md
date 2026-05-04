# I479 — ContextProvider Trait + LocalContextProvider (Pure Refactor)

**Status:** Done
**Priority:** P1
**Version:** 0.15.2
**Area:** Backend / Architecture
**ADR:** 0095

## Summary

Extract the intelligence context gathering path behind a `ContextProvider` trait so that `intel_queue.rs`, report generators, and any future consumer call a single interface rather than `build_intelligence_context()` directly. `LocalContextProvider` wraps the existing function with zero behavior change. This is the seam that I480 (GleanContextProvider) plugs into.

## Acceptance Criteria

### Trait definition

1. `context_provider/mod.rs` exists with a `ContextProvider` trait exposing `gather_entity_context()` (async, returns context payload) and `mode()` (returns `ContextMode`).
2. `ContextMode` enum has at least `Local` and `GleanAdditive` / `GleanGoverned` variants.
3. `GleanStrategy` enum distinguishes Additive (merge Glean on top of local) from Governed (Glean replaces local for overlapping fields).

### LocalContextProvider

4. `context_provider/local.rs` implements `ContextProvider` for `LocalContextProvider`.
5. `LocalContextProvider::gather_entity_context()` delegates to the existing `build_intelligence_context()` function. No new AI calls, no new data sources.
6. `LocalContextProvider::mode()` returns `ContextMode::Local`.

### AppState integration

7. `state.rs` has `context_provider: Arc<dyn ContextProvider>` on `AppState`.
8. On startup, `LocalContextProvider` is initialized as the default when no Glean config exists in the DB.

### Consumer migration

9. `intel_queue.rs` calls `state.context_provider.gather_entity_context()` instead of `build_intelligence_context()` directly.
10. All 4 report generators (SWOT, account health, EBR/QBR, risk briefing) route through `ContextProvider`.
11. No behavioral change — all existing intelligence and report output is identical before and after this refactor.

### ADR

12. ADR-0095 exists at `.docs/decisions/` documenting the dual-mode context architecture.

### Tests

13. `cargo test` passes.
14. `cargo clippy -- -D warnings` passes.

## Files

### New
- `src-tauri/src/context_provider/mod.rs` — `ContextProvider` trait, `ContextMode`, `GleanStrategy`, persistence helpers
- `src-tauri/src/context_provider/local.rs` — `LocalContextProvider` wrapping `build_intelligence_context()`

### Modified
- `src-tauri/src/state.rs` — `context_provider: Arc<dyn ContextProvider>` field
- `src-tauri/src/intel_queue.rs` — call through `ContextProvider` trait
- `src-tauri/src/lib.rs` — initialize `LocalContextProvider` on startup
- Report generator files — route through `ContextProvider`
