# I644 Field Promotion Matrix

Maps every Glean-accessible field to its DB destination, source kind, freshness, and I644 acceptance criterion.

> This is a .docs/ file -- local context only, never committed to remote.

## Source Kind Legend

- **fact** -- system-backed value from a source of record (Salesforce, Zendesk). Safe to promote into canonical account state.
- **signal** -- observed evidence that informs scoring and narrative but is not itself canonical truth.
- **inference** -- AI synthesis or editorial judgment from Glean chat. Not safe to promote without user confirmation.

## Source Confidence Tiers (ADR-0100)

| Source | Confidence | Notes |
|--------|-----------|-------|
| User correction | 1.0 | Sacred -- never overwritten |
| Salesforce (via Glean CRM) | 0.9 | glean_crm |
| Zendesk (via Glean) | 0.85 | glean_zendesk |
| Gong (via Glean) | 0.8 | glean_gong |
| Glean AI synthesis | 0.7 | glean_chat |
| Slack / P2 | 0.5 | Informs narrative only, never adjusts scores |
| PTY synthesis | 0.5 | Local fallback |

---

## 1. Commercial (ARR, contract, renewal, pricing)

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| ARR (system value) | Salesforce | fact | `accounts.arr` + `account_source_refs` | 24h | 5, 8 |
| ARR (AI estimate) | Glean synthesis | inference | `account_source_refs` only | 72h | 9 |
| ARR range low | Salesforce | fact | `accounts.arr_range_low` (not yet added) | 24h | 5 |
| ARR range high | Salesforce | fact | `accounts.arr_range_high` (not yet added) | 24h | 5 |
| Contract start | Salesforce | fact | `contract_context.contract_start` (entity_assessment) | 30d | 5 |
| Contract end / renewal date | Salesforce | fact | `accounts.contract_end` + `account_source_refs` | 24h | 5 |
| Contract type | Salesforce | fact | `contract_context.contract_type` (entity_assessment) | 30d | 7 |
| Auto-renew | Salesforce | fact | `contract_context.auto_renew` (entity_assessment) | 30d | 7 |
| Multi-year remaining | Salesforce | fact | `contract_context.multi_year_remaining` (entity_assessment) | 30d | 7 |
| Previous renewal outcome | Salesforce / Glean | signal | `contract_context.previous_renewal_outcome` (entity_assessment) | 90d | 7 |
| Customer fiscal year start | Salesforce | fact | `contract_context.customer_fiscal_year_start` (entity_assessment) | 90d | 7 |
| Procurement notes | Glean synthesis | inference | `contract_context.procurement_notes` (entity_assessment) | 30d | 9 |
| Renewal likelihood | Salesforce | fact | `accounts.renewal_likelihood` (not yet added) | 24h | 5, 6 |
| Renewal opportunity stage | Salesforce | fact | `accounts.commercial_stage` (migration 076) | 24h | 6, 11 |
| Renewal outlook confidence | Glean synthesis | inference | `renewal_outlook.confidence` (entity_assessment) | 72h | 9 |
| Renewal risk factors | Glean synthesis | inference | `renewal_outlook.risk_factors` (entity_assessment) | 72h | 9 |
| Expansion potential | Glean synthesis | inference | `renewal_outlook.expansion_potential` (entity_assessment) | 72h | 9 |
| Recommended renewal start | Glean synthesis | inference | `renewal_outlook.recommended_start` (entity_assessment) | 72h | 9 |
| Negotiation leverage | Glean synthesis | inference | `renewal_outlook.negotiation_leverage` (entity_assessment) | 72h | 9 |
| Negotiation risk | Glean synthesis | inference | `renewal_outlook.negotiation_risk` (entity_assessment) | 72h | 9 |
| Expansion signals | Gong / Salesforce / Glean | signal | `expansion_signals[]` (entity_assessment dimensions_json) | 30d | 7 |
| Expansion ARR impact | Salesforce | signal | `expansion_signals[].arr_impact` (entity_assessment) | 30d | 7 |
| Blockers | Gong / meetings | signal | `blockers[]` (entity_assessment dimensions_json) | 14d | 7 |
| Product classification | Salesforce | fact | `product_classification.products[]` (entity_assessment) + `account_products` table | 14d | 5 |
| Product type | Salesforce | fact | `account_products.product_type` (I651) | 14d | 5 |
| Product tier | Salesforce | fact | `account_products.tier` (I651) | 14d | 5 |
| Product ARR | Salesforce | fact | `account_products.arr` (I651) | 14d | 5 |
| Product billing terms | Salesforce | fact | `account_products.billing_terms` (I651) | 14d | 5 |

## 2. Relationship (stakeholders, champion, engagement)

> Note: I652 already handles the stakeholder DB model. Fields here describe what Glean provides and where it lands.

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| Stakeholder name | Salesforce / Gong / Glean | fact | `account_stakeholders.person_id` + `people.name` | 30d | 7 |
| Stakeholder role (job title) | Salesforce / Glean | fact | `people.role` | 30d | 7 |
| Stakeholder account role | Glean synthesis | inference | `account_stakeholder_roles.role` | 30d | 9 |
| Stakeholder engagement level | Gong / meetings | signal | `account_stakeholders.engagement` | 14d | 7 |
| Stakeholder assessment | Glean synthesis | inference | `account_stakeholders.assessment` | 30d | 9 |
| Champion designation | User | fact | `account_stakeholders.role` contains "champion" | indefinite | -- |
| Champion strength | Glean synthesis | inference | `relationship_depth.champion_strength` (entity_assessment) | 30d | 9 |
| Executive access | Glean synthesis | inference | `relationship_depth.executive_access` (entity_assessment) | 30d | 9 |
| Stakeholder coverage level | Glean synthesis | inference | `coverage_assessment.level` (entity_assessment) | 30d | 9 |
| Coverage gaps | Glean synthesis | inference | `coverage_assessment.gaps[]` (entity_assessment) | 30d | 9 |
| Role fill rate | computed | signal | `coverage_assessment.role_fill_rate` (entity_assessment) | 30d | -- |
| Organizational changes | Glean / Gong | signal | `organizational_changes[]` (entity_assessment dimensions_json) | 14d | 7 |
| Internal team members | Salesforce / Glean | fact | `internal_team[]` (entity_assessment dimensions_json) | 30d | 7 |
| Stakeholder insights | Glean synthesis | inference | `stakeholder_insights[]` (entity_assessment) | 30d | 9 |

## 3. Support (tier, CSAT, tickets, SLA)

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| Support tier / package | Zendesk / Salesforce | fact | `account_technical_footprints.support_tier` (I649) | 7d | 5 |
| CSAT score | Zendesk | signal | `account_technical_footprints.csat_score` (I649) | 30d | 7 |
| Open ticket count | Zendesk | signal | `account_technical_footprints.open_tickets` (I649) | 1d | 7 |
| Critical ticket count | Zendesk | signal | `support_health.critical_tickets` (entity_assessment) | 1d | 7 |
| Avg resolution time | Zendesk | signal | `support_health.avg_resolution_time` (entity_assessment) | 7d | 7 |
| Support health trend | Glean synthesis | inference | `support_health.trend` (entity_assessment) | 7d | 9 |
| NPS score | Survey tool | fact | `accounts.nps` + `nps_csat.nps` (entity_assessment) | 90d | 5 |
| CSAT (survey) | Survey tool | signal | `nps_csat.csat` (entity_assessment) | 30d | 7 |
| Survey date | Survey tool | fact | `nps_csat.survey_date` (entity_assessment) | 90d | 7 |
| Survey verbatim | Survey tool | signal | `nps_csat.verbatim` (entity_assessment) | 90d | 9 |
| Services stage | Internal | fact | `account_technical_footprints.services_stage` (I649) | 30d | 7 |

## 4. Technical (footprint, adoption, entitlements)

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| Usage tier | Telemetry / Glean | signal | `account_technical_footprints.usage_tier` (I649) | 7d | 7 |
| Adoption score | Telemetry / Glean | signal | `account_technical_footprints.adoption_score` (I649) | 7d | 7 |
| Active users | Telemetry / Glean | signal | `account_technical_footprints.active_users` (I649) | 7d | 7 |
| Adoption rate | Glean synthesis | inference | `product_adoption.adoption_rate` (entity_assessment) | 7d | 9 |
| Adoption trend | Glean synthesis | inference | `product_adoption.trend` (entity_assessment) | 7d | 9 |
| Feature adoption list | Glean synthesis | inference | `product_adoption.feature_adoption[]` (entity_assessment) | 14d | 9 |
| Last active date | Telemetry / Glean | signal | `product_adoption.last_active` (entity_assessment) | 7d | 7 |
| Integrations | Glean / internal | signal | `account_technical_footprints.integrations_json` (I649) | 30d | 7 |
| Active subscription count | Salesforce | fact | not yet extracted -- derive from `account_products` count | 14d | 5 |
| Primary product | Salesforce | fact | not yet extracted -- derive from highest-ARR `account_products` row | 14d | 5 |
| Customer status | Salesforce | fact | not yet extracted -- maps to `accounts.lifecycle` | 7d | 5 |

## 5. Growth (potential, ICP fit, expansion signals)

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| Growth potential score | Glean synthesis | inference | `accounts.growth_potential_score` (not yet added) | 30d | 5 |
| ICP fit score | Glean synthesis | inference | `accounts.icp_fit_score` (not yet added) | 30d | 5 |
| Growth tier | Glean / CRM | signal | `org_health.growth_tier` (entity_assessment) | 30d | 7 |
| Customer stage | Salesforce | fact | `org_health.customer_stage` (entity_assessment) | 7d | 7 |
| ICP fit (qualitative) | Glean synthesis | inference | `org_health.icp_fit` (entity_assessment) | 30d | 9 |
| Competitor name | Gong / Slack / Glean | signal | `competitive_context[].competitor` (entity_assessment) | 14d | 7 |
| Competitor threat level | Glean synthesis | inference | `competitive_context[].threat_level` (entity_assessment) | 14d | 9 |
| Strategic priorities | Glean synthesis | inference | `strategic_priorities[]` (entity_assessment) | 30d | 9 |

## 6. Narrative (executive assessment, risks, wins -- NOT promoted to accounts columns)

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| Executive assessment | Glean synthesis | inference | `entity_assessments.executive_assessment` (DB) | 72h | 9 |
| Pull quote | Glean synthesis | inference | `entity_assessments.dimensions_json -> pullQuote` (DB) | 72h | 9 |
| Risks | Gong / Zendesk / Glean | signal/inference | `entity_assessments.risks_json` (DB) | 14d | 9 |
| Recent wins | Gong / Salesforce / Glean | signal | `entity_assessments.wins_json` (DB) | 30d | 9 |
| Working / not working / unknowns | Glean synthesis | inference | `entity_assessments.current_state_json` (DB) | 30d | 9 |
| Value delivered | Gong / meetings / Glean | signal | `entity_assessments` (via intelligence.json -> DB) | 30d | 9 |
| Success metrics | Glean synthesis | inference | `entity_assessments` (via intelligence.json -> DB) | 30d | 9 |
| Success plan signals | Glean synthesis | inference | `entity_assessments` (via intelligence.json -> DB) | 30d | 9 |
| Open commitments | Gong / meetings / Glean | signal | `entity_assessments` (via intelligence.json -> DB) | 14d | 9 |
| Gong call summaries | Gong | signal | `gong_call_summaries[]` (entity_assessment) | 7d | 7 |
| Company description | Glean | inference | `company_context.description` (entity_assessment) | 90d | 9 |
| Company industry | Glean / Salesforce | fact | `company_context.industry` (entity_assessment) | 90d | 7 |
| Company size | Glean / Salesforce | fact | `company_context.size` (entity_assessment) | 90d | 7 |
| Company headquarters | Glean | inference | `company_context.headquarters` (entity_assessment) | 90d | 9 |
| Domains | Glean / email | signal | `account_domains` table | 30d | -- |

## 7. Health Scoring (computed, not directly from Glean)

| Field | Source System | Source Kind | DB Destination | Freshness | AC |
|-------|-------------|-------------|----------------|-----------|-----|
| Health score (0-100) | Computed (ADR-0097) | computed | `entity_assessments.health_score` | per-enrichment | -- |
| Health band | Computed | computed | `entity_assessments.health_band` | per-enrichment | -- |
| Health trend direction | Computed + inference | inference | `entity_assessments` health JSON | per-enrichment | -- |
| Health narrative | Glean synthesis | inference | `entity_assessments` health JSON | per-enrichment | 9 |
| 6 dimension scores | Computed (ADR-0097) | computed | `entity_assessments` health JSON | per-enrichment | -- |
| Org health baseline | Glean / CRM | signal | `org_health_data` in entity_assessment | 30d | 7 |

---

## Fields NOT YET Extracted (I644 scope additions)

These fields are defined in the I644 issue spec but do not yet have extraction code:

| Field | Proposed Source | Proposed Destination | Status |
|-------|----------------|---------------------|--------|
| `arr_range_low` | Salesforce | `accounts.arr_range_low` | Not yet added to schema |
| `arr_range_high` | Salesforce | `accounts.arr_range_high` | Not yet added to schema |
| `renewal_likelihood` | Salesforce | `accounts.renewal_likelihood` | Not yet added to schema |
| `renewal_model` | Salesforce | new column | Not yet added to schema |
| `renewal_pricing_method` | Salesforce | new column | Not yet added to schema |
| `growth_potential_score` | Glean inference | `accounts.growth_potential_score` | Not yet added to schema |
| `icp_fit_score` | Glean inference | `accounts.icp_fit_score` | Not yet added to schema |
| `active_subscription_count` | Derived from `account_products` | new column or computed | Not yet extracted |
| `primary_product` | Derived from `account_products` | new column or computed | Not yet extracted |
| `customer_status` | Salesforce | maps to `accounts.lifecycle` | Not yet extracted directly |
| `source_coverage` | Computed | new assessment field | Not yet extracted |
| `current_themes` | Glean synthesis | new assessment field | Not yet extracted |
| `recent_incidents` | Zendesk | new assessment field | Not yet extracted |
| `compliance_context` | Glean synthesis | new assessment field | Not yet extracted |
| `relationship_assessment` | Glean synthesis | new assessment field | Not yet extracted |

---

## Already Implemented (migration 076 + I649 + I651 + I652)

| Migration | What it added | Status |
|-----------|--------------|--------|
| 076 `source_aware_account_truth` | `account_source_refs` table + `accounts.commercial_stage` column | Shipped |
| I649 | `account_technical_footprints` table (support_tier, csat_score, open_tickets, usage_tier, adoption_score, active_users, services_stage, integrations_json) | Shipped |
| I651 | `account_products` extended with product_type, tier, billing_terms, arr, last_verified_at, data_source | Shipped |
| I652 | `account_stakeholders` + `account_stakeholder_roles` + `stakeholder_suggestions` with per-role provenance | Shipped |

---

## Source Reference Model (`account_source_refs`)

From migration 076:

```sql
CREATE TABLE account_source_refs (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  field TEXT NOT NULL,           -- "arr", "renewal_date", "lifecycle", "nps", "champion"
  source_system TEXT NOT NULL,   -- "salesforce", "zendesk", "gong", "user"
  source_kind TEXT NOT NULL DEFAULT 'inference',  -- "fact", "signal", "inference"
  source_value TEXT,             -- the actual value
  observed_at TEXT NOT NULL,
  source_record_ref TEXT,        -- optional: opportunity ID, ticket ID, etc.
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

The I644 spec proposes a richer `entity_source_references` table with `reference_type`, `reference_title`, `reference_url`, `glean_locator_json`, `snippet`, `stale_after`, and `is_current` columns. The current `account_source_refs` is a first step toward that fuller model.

---

## Data Flow Summary

```
Glean chat (6 parallel dimensions)
  |
  v
parse_intelligence_response() --> IntelligenceJson
  |
  v
merge_dimension_into() (6x) --> combined IntelligenceJson
  |
  v
reconcile_enrichment() (I576) --> preserves user edits + tombstones
  |
  +---> entity_assessments table (narrative, dimensions_json, health)
  +---> account_products table (I651 product classification upsert)
  +---> account_source_refs table (fact-kind fields promoted)
  +---> account_technical_footprints table (I649 support/adoption)
  +---> account_stakeholders / stakeholder_suggestions (I652)
  +---> accounts table (arr, contract_end, lifecycle, nps, commercial_stage)
```
