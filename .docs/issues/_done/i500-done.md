# I500 — Glean Org-Score Parsing

**Priority:** P1
**Area:** Backend / Connectors
**Version:** v1.0.0 (Phase 2)
**Depends on:** I503 (OrgHealthData type)
**ADR:** 0097

## Problem

The user's organization maintains a multi-factor health model (product usage, support SLAs, commercial data) that produces structured health scores distributed via Salesforce and indexed by Glean. DailyOS currently receives this data as unstructured free text in Glean search result snippets — fields like `health_score_3_green`, `renewal_likelihood`, `growth_tier`, `customer_stage`, `support_tier`, and `icp_fit` appear as tags or org-level fields but are never parsed into structured data.

This means DailyOS cannot use the org's existing health model as a baseline score. It either ignores it entirely or lets the LLM attempt ad-hoc interpretation of the free text during enrichment. ADR-0097 specifies that when Glean provides an org health score, it should be the authoritative baseline — DailyOS adds relationship context on top, not a competing number.

## Design

### Parsing Location

The extraction happens inside `GleanContextProvider::gather_glean_context()` in `src-tauri/src/context_provider/glean.rs`. After search results are collected (`all_results: Vec<GleanSearchResult>`), a new function parses the results into an `OrgHealthData` struct before they are assembled into `file_contents`.

### Parser Function

```rust
/// Extract structured health data from Glean search results.
///
/// Scans snippets and titles for known org health field patterns.
/// Returns None if no health data is found in any result.
pub fn parse_org_health_data(
    results: &[GleanSearchResult],
    account_name: &str,
) -> Option<OrgHealthData>
```

### Pattern Matching Strategy

The org's health data appears in Glean search results in several formats:

1. **Salesforce account records** (doc_type: `salesforce_account`): Snippets contain structured field-value pairs like `Health Score: Green`, `Renewal Likelihood: Yellow`, `Growth Tier: Tier 1 - Expansion`.

2. **Zendesk org fields** (doc_type: `zendesk_organization`): Tags contain `health_score_3_green`, `health_score_2_yellow`, `health_score_1_red` and org fields like `customer_stage`, `support_tier`.

3. **Dashboard/report pages**: Titles or snippets may contain `{account_name} Health: Green` or similar formatted summaries.

The parser uses a priority order:
1. Salesforce account records (most authoritative — source of truth for commercial data)
2. Zendesk organization fields (reliable for support-side health)
3. Any other document with matching patterns (lowest confidence)

### Extraction Patterns

```rust
// Health band extraction — multiple formats
// "health_score_3_green" → "green"
// "health_score_2_yellow" → "yellow"
// "health_score_1_red" → "red"
// "Health Score: Green" → "green"
// "Health: Green" → "green"
static HEALTH_BAND_PATTERNS: &[(&str, &str)] = &[
    ("health_score_3_green", "green"),
    ("health_score_2_yellow", "yellow"),
    ("health_score_1_red", "red"),
];

// Regex patterns for field extraction
// "Renewal Likelihood: Green" → "green"
// "Growth Tier: Tier 1 – Expansion" → "Tier 1 – Expansion"
// "Customer Stage: Adoption" → "Adoption"
// "Support Tier: Enhanced" → "Enhanced"
// "ICP Fit: Good Fit" → "Good Fit"
```

Each pattern is case-insensitive. The parser scans both `snippet` and `title` of each `GleanSearchResult`.

### Storage

`OrgHealthData` is stored on the `entity_intelligence` table in the `org_health TEXT` JSON column (created by I503's migration `054_health_schema_evolution.sql`). It is also passed to `compute_account_health()` in I499 as the baseline source. I500 does NOT create this column — I503 owns the migration.

The `gathered_at` timestamp records when the Glean data was parsed. The `source` field records which Glean document type provided the data (e.g., `"glean_salesforce"`, `"glean_zendesk"`).

### Cache Integration

Parsed `OrgHealthData` is cached via the existing `GleanCache` with `CacheKind::Document` and key `org_health:{account_id}`. Cache TTL follows the existing Glean cache policy. On cache hit, the cached `OrgHealthData` is returned without re-parsing.

### Fallback Behavior

When Glean is not connected, or when search results contain no recognizable health fields, `parse_org_health_data()` returns `None`. The health scoring engine (I499) falls back to computing its own baseline from relationship signals with an explicit lower-confidence band.

### Flow Integration

**Data flow from Glean provider to health engine:**

```
GleanContextProvider::gather_glean_context()
  → parse_org_health_data(&results, &entity_name)
  → Store OrgHealthData on GleanEntityData (new field)
  → GleanEntityData flows into EnrichmentInput (intel_queue.rs)
       │
       ├─ Write to DB: db.update_org_health(entity_id, &org_health_json)
       │   → entity_intelligence.org_health TEXT column (created by I503 migration)
       │
       └─ Pass to compute_account_health(db, account, Some(&org_health))
           → OrgHealthData used as baseline score in AccountHealth
           → Divergence detection compares org band vs relationship score
```

**Key design point:** `OrgHealthData` is both persisted (for historical reference and offline use) and passed in-memory to the health engine (for same-cycle computation). The DB column `entity_intelligence.org_health` is created by I503's migration. I500 only reads/writes it.

**Where IntelligenceContext lives:** The `IntelligenceContext` struct is in `intelligence/prompts.rs` (NOT `context_provider/mod.rs` as originally stated). Add `pub org_health: Option<OrgHealthData>` there. The field flows through the `build_enrichment_prompt()` call chain.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/context_provider/glean.rs` | Add `parse_org_health_data()` function. Call it in `gather_glean_context()` for account entities. Store result on `GleanEntityData`. |
| `src-tauri/src/intelligence/prompts.rs` | Add `pub org_health: Option<OrgHealthData>` to `IntelligenceContext` (this is where the struct lives, not `context_provider/mod.rs`). |
| `src-tauri/src/intel_queue.rs` | Pass `org_health` from `IntelligenceContext` to `compute_account_health()`. |

## Acceptance Criteria

1. Given a Glean search result containing `health_score_3_green` in its snippet, `parse_org_health_data()` returns `OrgHealthData { health_band: Some("green"), .. }`
2. Given a Glean search result containing `Renewal Likelihood: Yellow`, the parsed `OrgHealthData` includes `renewal_likelihood: Some("yellow")`
3. Given a Glean search result with `Customer Stage: Adoption`, `Growth Tier: Tier 1 – Expansion`, and `Support Tier: Enhanced`, all three fields are extracted
4. Given search results with no health-related fields, `parse_org_health_data()` returns `None`
5. The parser correctly identifies `salesforce_account` doc_type results as highest priority source
6. Parsed `OrgHealthData` is cached and reused on subsequent enrichment cycles (verified by cache hit log)
7. `cargo test` includes unit tests with sample Glean snippets covering all extraction patterns
8. When Glean is not connected, the system gracefully falls back to computed baseline (no panics, no errors)

## Out of Scope

- Computing the DailyOS baseline from relationship signals (I499)
- Using `OrgHealthData` to influence the enrichment prompt narrative (I499 integration)
- Displaying org health data in the UI (I502)
- Supporting non-Glean enterprise search providers
- Structured extraction of non-health Glean fields (Salesforce contacts, Gong recordings, etc.) — future issue
