# I482 — Role-Aware Glean Query Optimization

**Priority:** P1
**Area:** Backend / Connectors + Intelligence
**Depends on:** I479–I481 (shipped v0.15.2), MCP OAuth + DCR (shipped on `dev`)

## Problem

`GleanContextProvider::entity_search_queries()` sends generic queries to Glean:

```rust
"account" => queries.push(format!("{} account", acct.name));
"project" => queries.push(format!("{} project", proj.name));
"person"  => queries.push(person.name.clone());
```

These return broad, unfocused results. When tested against a real Glean instance (Globex Holdings account), the generic `"{name} account"` query returned a mix of Salesforce data, commercials, key roles, support signals, and account activity — but Glean itself suggested more targeted queries would yield better results.

The role preset system already defines exactly what a CS user cares about (ARR, health, renewal, champion changes) but none of that vocabulary reaches the Glean query builder.

## Desired Outcome

Glean queries are shaped by the active role preset's vocabulary, vitals, lifecycle events, and briefing emphasis. A CSM searching for Globex Holdings gets renewal timeline, health score, open support tickets, champion contacts, and expansion signals — not a generic company overview.

## Design

### 1. Preset-Aware Query Generation

`entity_search_queries()` gains access to the active `RolePreset` and generates targeted queries per entity type.

**Account queries for CS preset:**
| Query Pattern | Source |
|---|---|
| `"{name} renewal"` | `lifecycle_events` contains `"renewal"` |
| `"{name} health score"` | `vocabulary.health_label` |
| `"{name} ARR"` or `"{name} contract value"` | `vocabulary.primary_metric` |
| `"{name} open tickets"` or `"{name} support escalation"` | `lifecycle_events` contains `"escalation"` |
| `"{name} champion"` or `"{name} executive sponsor"` | `stakeholder_roles[].label` |
| `"{name} QBR"` or `"{name} business review"` | `vocabulary.cadence_noun` |
| `"{name} expansion"` or `"{name} upsell"` | `lifecycle_events` contains `"expansion"` |
| `"{name} onboarding"` | `lifecycle_events` contains `"onboarding_complete"` |
| Domain queries (unchanged) | `account_domains` |

**Person queries:**
| Query Pattern | Source |
|---|---|
| `"{name}"` (unchanged) | Person name |
| `"{email}"` (unchanged) | Person email |
| `"{name} {company}"` | Person + linked entity name |
| `"{name} role"` or `"{name} title"` | Always useful for contact enrichment |

**Project queries:**
| Query Pattern | Source |
|---|---|
| `"{name} project"` (unchanged) | Project name |
| `"{name} status"` | `vitals.project[].key == "status"` |
| `"{name} timeline"` or `"{name} target date"` | `vitals.project[].key == "target_date"` |

### 2. Query Budget and Prioritization

Glean MCP search has latency cost per query. Budget: **max 8 queries per entity enrichment** (up from current 1-3).

Priority order for accounts:
1. Name + domain (identity — always first)
2. Urgency drivers (`renewal_within_90d`, `health_declining`, `champion_change`)
3. Primary/secondary signal (`arr`, `health`)
4. Lifecycle events relevant to current lifecycle stage
5. Stakeholder role discovery

### 3. Lifecycle-Stage Filtering

If the account has a known lifecycle stage (e.g., `"renewal"`), weight queries toward that stage:
- **Onboarding**: `"{name} onboarding"`, `"{name} go-live"`, `"{name} implementation"`
- **Renewal**: `"{name} renewal"`, `"{name} contract"`, `"{name} pricing"`, `"{name} competitor"`
- **At-risk**: `"{name} escalation"`, `"{name} churn"`, `"{name} support tickets"`, `"{name} NPS"`

### 4. Result Deduplication

Multiple targeted queries may return overlapping Glean documents. Deduplicate by document URL before assembling context. Current `gather_entity_context()` already concatenates all search results — add URL-based dedup.

### 5. Account Detail Page Enrichment (Future)

Glean can surface structured data that maps directly to existing account detail sections:

| Glean Source | Account Detail Section | Notes |
|---|---|---|
| Salesforce opportunity data | Vitals (ARR, renewal date) | Could auto-populate vitals from Glean |
| Gong call recordings | Recent activity / intelligence | Meeting summaries, sentiment |
| Zendesk/Intercom tickets | Risk signals | Open ticket count, escalations |
| Gainsight health scores | Health vital | Direct health score sync |
| Salesforce contacts | Stakeholder map | Auto-discover stakeholders |
| Slack channels | Communication signals | Activity frequency, sentiment |

This is a larger effort (structured extraction from Glean results) and should be scoped as a follow-on issue if validated.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/context_provider/glean.rs` | `entity_search_queries()` takes `&RolePreset`, generates preset-aware queries. Add dedup in `gather_entity_context()`. |
| `src-tauri/src/context_provider/mod.rs` | Pass preset through `ContextProvider::gather_entity_context()` signature (or load from AppState). |
| `src-tauri/src/intel_queue.rs` | Pass preset when calling `gather_entity_context()`. |

## Acceptance Criteria

1. With CS preset active and Glean connected, refreshing Globex Holdings produces queries like `"Globex Holdings renewal"`, `"Globex Holdings health score"`, `"Globex Holdings champion"` — not just `"Globex Holdings account"`
2. Query count is capped at 8 per entity enrichment
3. Duplicate Glean documents (same URL) are deduplicated before context assembly
4. Preset vocabulary terms appear in Glean search queries (e.g., `vocabulary.primary_metric` → `"{name} ARR"`)
5. A non-CS preset (e.g., Sales) generates different queries reflecting its vocabulary (e.g., `"{name} pipeline"`, `"{name} deal stage"`)
6. Intelligence output for Glean-connected accounts contains more specific, actionable data than the generic query baseline

## Out of Scope

- Structured data extraction from Glean results (mapping Salesforce fields to vitals) — future issue
- New account detail page sections — future issue
- Glean `read_document` follow-up queries based on search results — already implemented, unchanged
- Changes to the MCP OAuth or DCR flow — already shipped
