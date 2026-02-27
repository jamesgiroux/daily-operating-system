# ADR-0095: Dual-Mode Context Architecture — Local + Glean

**Date:** 2026-02-27
**Status:** Accepted
**Target:** v0.16.2 (or next available)
**Extends:** ADR-0086 (Intelligence as Shared Service), ADR-0091 (IntelligenceProvider Abstraction)

## Context

Maeve Lander (Glean adoption lead at VIP) positioned DailyOS as a "presentation and interaction layer" on top of Glean's knowledge graph. For VIP/Automattic, Glean already indexes Salesforce, Gong, Zendesk, Confluence, Gmail, and LinkedIn — the same sources DailyOS currently reaches via separate connectors (Clay, Gravatar, Gmail API).

The strategic question: can Glean replace the individual connectors as the primary context source for entity intelligence, while keeping DailyOS's local-first architecture for personal deployments?

## Decision

### Two Orthogonal Abstractions

Introduce a `ContextProvider` trait that sits alongside ADR-0091's `IntelligenceProvider`:

- **ContextProvider** (this ADR) — where entity context is gathered (local DB/files vs. Glean search)
- **IntelligenceProvider** (ADR-0091) — how assembled context is synthesized into intelligence (Claude Code vs. Ollama vs. OpenAI)

A Glean deployment uses `GleanContextProvider` + `ClaudeCodeProvider`. A personal deployment uses `LocalContextProvider` + `ClaudeCodeProvider`. An air-gapped deployment uses `LocalContextProvider` + `OllamaProvider`. The combinations are independent.

### The ContextProvider Trait

```rust
pub trait ContextProvider: Send + Sync {
    fn gather_entity_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        prior: Option<&IntelligenceJson>,
    ) -> Result<IntelligenceContext, ContextError>;

    fn provider_name(&self) -> &str;
    fn is_remote(&self) -> bool;
}
```

### Context Mode

```rust
pub enum ContextMode {
    Local,                    // Today's behavior (default)
    Glean {
        endpoint: String,     // Glean MCP server URL
        keychain_key: String, // macOS Keychain key for OAuth token
        strategy: GleanStrategy,
    },
}

pub enum GleanStrategy {
    Additive,  // Glean primary + local signals merged
    Governed,  // Glean only — suppress local connectors
}
```

### Glean is a Context Source, Not an Intelligence Provider

The PTY enrichment path, signal bus, meeting prep queue, and `IntelligenceJson` schema are all unchanged. What changes is where `build_intelligence_context()` gets its data. The `GleanContextProvider` calls Glean's MCP server (search + read_document tools) to populate `IntelligenceContext.file_contents` and enriches `stakeholders` from Glean's org graph.

### Two-Phase Gather

Glean searches are 200-2000ms HTTP calls. The current flow holds a DB lock during context assembly (milliseconds). In Glean mode:

- **Phase A** (holds DB lock, ms): Read always-local data — meetings, actions, captures, user_context, prior_intelligence
- **Phase B** (no DB lock, network): Query Glean for entity documents, person relationships, stakeholders. Results cached.

### Connector Gating

| Connector | Additive | Governed |
|---|---|---|
| Google Calendar | Active | Active |
| Granola/Quill | Active | Active |
| Linear | Active | Active |
| Gmail API | Active | **Disabled** |
| Clay (Smithery) | **Disabled** | **Disabled** |
| Gravatar | **Disabled** | **Disabled** |

In both Glean strategies, Clay and Gravatar are disabled (Glean replaces these). Gmail is only disabled in Governed mode (Glean indexes Gmail directly).

### Caching

- In-memory: `DashMap<String, CacheEntry>` for hot path
- Persistent: `glean_document_cache` SQLite table for cross-restart
- TTLs: Documents 1h, Person profiles 24h, Org graph 4h
- Manual refresh bypasses cache

### Outage Handling

Per-call fallback: `GleanContextProvider` returns `ContextError::Timeout` → intel_queue falls back to local-only context for that entity. Logged at WARN. Not re-queued for immediate retry.

### Mode Switching

Stored in `context_mode_config` DB table. Requires app restart to take effect. On switch:
- Local → Glean: Gmail/Clay/Gravatar pollers disabled. No existing intelligence discarded.
- Glean → Local: Connectors re-enable. Fresh sweep.

## Key Files

| File | Change |
|------|--------|
| `src-tauri/src/context_provider/mod.rs` | Trait, enums, persistence helpers |
| `src-tauri/src/context_provider/local.rs` | `LocalContextProvider` — wraps `build_intelligence_context()` |
| `src-tauri/src/context_provider/glean.rs` | `GleanContextProvider` — Glean MCP client, two-phase gather |
| `src-tauri/src/context_provider/cache.rs` | `GleanCache` — DashMap + DB table |
| `src-tauri/src/intel_queue.rs` | Calls `state.context_provider.gather_entity_context()` |
| `src-tauri/src/state.rs` | `context_provider: Arc<dyn ContextProvider>` on AppState |
| `src-tauri/src/enrichment.rs` | Skip Clay/Gravatar in Glean mode |
| `src-tauri/src/signals/bus.rs` | Glean source weights (0.7, 60-day half-life) |
| `src-tauri/src/google.rs` | Gmail poller gated by Governed mode |
| `src-tauri/src/migrations/052_glean_document_cache.sql` | Cache table + mode config |
| `src/components/settings/ContextSourceSection.tsx` | Settings UI |

## Consequences

**Positive:**
- Enterprise customers can use Glean as the knowledge graph without maintaining separate connectors
- Personal deployments are unaffected — `LocalContextProvider` is the default
- Intelligence pipeline (PTY → parse → write) is identical in both modes
- Clean separation: context gathering vs. intelligence synthesis are independent

**Negative:**
- Requires app restart for mode changes (acceptable for a rare config change)
- Glean MCP protocol may evolve — client must be maintained
- Two-phase gather adds complexity to the enrichment path

**Neutral:**
- `intelligence.json` is retained in Glean mode — it's DailyOS's synthesis artifact
- Signal bus weights for Glean sources (0.7) are between user corrections (1.0) and Clay/Gravatar (0.6)
