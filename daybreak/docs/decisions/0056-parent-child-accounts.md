# ADR-0056: Parent-Child Accounts (Enterprise BU Hierarchy)

**Date:** 2026-02-08
**Status:** Accepted

## Context

Enterprise customers (Cox, Hilton, Salesforce, Intuit) have business units (BUs) that operate as semi-independent entities — their own contracts, renewal dates, health scores, stakeholders, meeting cadences — but roll up to a parent company relationship. The flat account structure couldn't represent this. Enterprise is land-and-expand for CSMs: they work from one BU to the next. Without hierarchy, the tool can't model an enterprise portfolio.

Real workspace pattern (explored `~/Documents/VIP/Accounts/`):

```
Accounts/Cox/
├── dashboard.json            ← parent-level data
├── dashboard.md
├── Consumer-Brands/          ← BU (human-readable name, no numeric prefix)
│   ├── 01-Customer-Information/
│   ├── 02-Meetings/
│   └── ...
├── Corporate-Services-B2B/   ← BU
├── Diversification/          ← BU
└── Enterprises/              ← BU
```

## Decision

### Data model

Self-referential `parent_id TEXT` on the `accounts` table. NULL = top-level account. One level deep (no grandchildren). Non-breaking migration — existing flat accounts unaffected.

### Child ID scheme

`{slugify(parent)}--{slugify(child)}` — the `--` separator is unambiguous because `slugify()` collapses consecutive dashes, so `--` cannot be produced by slugify alone.

### Path resolution

Use the existing `tracker_path` field on `DbAccount`. `workspace.join(tracker_path)` resolves both flat paths (`Accounts/Cox`) and nested paths (`Accounts/Cox/Consumer-Brands`) without hierarchy table lookups. All write functions (`write_account_json`, `write_account_markdown`, `update_account_field`, etc.) resolve directories via `tracker_path` with fallback to `account_dir()`.

### BU detection heuristic

`is_bu_directory(name)`: subdirectories that don't start with a digit, `_`, or `.`. Internal org folders (`01-Customer-Information`, `02-Meetings`) start with digits. Already skip `_`/`.`-prefixed dirs. This reliably distinguishes all real accounts across 4 enterprise customers with 24 total BUs.

### Parent accounts are full accounts

Parents have their own `dashboard.json`, own data, AND aggregate child signals. This matches the real workspace where parent-level dashboards carry company-level context.

### UI

- **AccountsPage**: expandable parent rows with disclosure chevron, lazy-loaded children with indent
- **AccountDetailPage**: breadcrumb for children (`Accounts > Cox > Consumer Brands`), BU list section for parents, portfolio aggregate row (total ARR, worst health, nearest renewal, BU count)
- **Markdown writeback**: parent account markdown includes "Business Units" section listing children with health badges and ARR

### Intelligence aggregation

`get_parent_aggregate()` SQL query computes across children: `COUNT(*)`, `SUM(arr)`, `MIN(health)` (worst), `MIN(contract_end)` (nearest renewal). Displayed on parent detail page.

## Consequences

**Easier:**
- Enterprise portfolios are visible and navigable — parent shows the forest, children show the trees
- BU discovery is automatic from workspace structure — no manual configuration
- Existing flat accounts work identically (additive migration)
- Path resolution via `tracker_path` means all file I/O works for nested directories without special-casing

**Harder:**
- `guess_account_name()` in meeting prep only searches top-level Accounts/ directories — doesn't discover nested BU directories (tracked as I117)
- ActionsPage shows raw `account_id` which includes `--` separator for child accounts instead of human-readable name (tracked as I116)

**Trade-offs:**
- One level deep only (no grandchildren) — sufficient for all known enterprise patterns
- BU detection heuristic depends on naming conventions (no digit prefix = BU) — documented and verified across all real accounts
- Child count queries are N+1 per parent on AccountsPage — dozens of accounts, not thousands, so negligible
