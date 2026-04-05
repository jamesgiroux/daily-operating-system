# I528 — ADR-0098 Data Lifecycle Infrastructure

**Priority:** P1
**Area:** Backend / Data Governance
**Version:** 1.0.0
**Depends on:** I511 (schema decomposition — `data_source` column pattern)
**ADR:** 0098

## Problem

ADR-0098 defines a source-aware data lifecycle: every record tags its provenance source, and revoking a source (Google OAuth, Glean auth, Clay API) purges all data from that source. The `DataSource` enum, `purge_source()` function, and `data_lifecycle.rs` module do not exist yet. Multiple Phase 2 issues depend on this infrastructure:

- **I487** (Glean signal emission) — needs ADR-0098 purge compliance for Glean-sourced signals
- **I505** (Glean stakeholder intelligence) — needs `data_source` tagging on `entity_people` and purge-on-revocation

## Design

### DataSource Enum (`src-tauri/src/db/data_lifecycle.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    User,       // Priority 4 — manual entry, never auto-purged
    Clay,       // Priority 3 — Clay API enrichment
    Glean,      // Priority 2 — Glean search/agent data
    Gravatar,   // Priority 2 — Gravatar profile enrichment
    Google,     // Priority 2 — Calendar/Gmail API data
    Ai,         // Priority 1 — LLM inference
}
```

### purge_source()

```rust
pub struct PurgeReport {
    pub source: DataSource,
    pub people_cleared: usize,
    pub signals_deleted: usize,
    pub relationships_deleted: usize,
    pub enrichment_sources_cleared: usize,
}

pub fn purge_source(db: &ActionDb, source: DataSource) -> Result<PurgeReport>
```

Purges all data tagged with the given source:
- `entity_people` rows where `data_source = source` → unlink (not delete person, just remove entity link)
- `signal_events` where `source = source` → delete
- `person_relationships` where `source = source` → delete
- `enrichment_sources` JSON on people → remove entries for this source
- Person profile fields sourced from this source → null out (check `enrichment_sources` provenance)

**Does NOT purge:**
- User-entered data (source = "user") — never auto-purged
- Entity intelligence (`intelligence.json`) — too intertwined; flag for re-enrichment instead
- Meetings/calendar events — these are user's calendar, not Glean's data

### Integration Points

- **Google OAuth revocation** → `purge_source(DataSource::Google)` — clears calendar/email data
- **Glean auth removal** → `purge_source(DataSource::Glean)` — clears Glean-sourced contacts, signals, relationships
- **Clay API key removal** → `purge_source(DataSource::Clay)` — clears Clay-sourced profile data

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/db/data_lifecycle.rs` | New module: `DataSource` enum, `purge_source()`, `PurgeReport` |
| `src-tauri/src/db/mod.rs` | Add `pub mod data_lifecycle;` |
| `src-tauri/src/db/people.rs` | Add `data_source` awareness to `update_person_profile()` |
| `src-tauri/src/commands.rs` | Wire purge to OAuth revocation flows |

## Acceptance Criteria

1. `DataSource` enum exists with all 6 variants
2. `purge_source(DataSource::Glean)` removes all Glean-sourced entity_people links, signals, and relationships
3. `purge_source(DataSource::Google)` clears Google-sourced data without touching user-entered data
4. Person profile fields are nulled only when their provenance matches the purged source
5. `cargo test` includes unit tests for purge behavior
6. Purge returns `PurgeReport` with counts for audit logging

## Out of Scope

- Intelligence JSON purge (too intertwined — flag for re-enrichment instead)
- UI for purge confirmation (uses existing Settings disconnect flows)
- Historical purge audit log (future enhancement)
