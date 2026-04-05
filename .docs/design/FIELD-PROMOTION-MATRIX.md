# Field Promotion Matrix (I648)

Classifies every Glean-accessible field family by authority, freshness, promotion destination, and target surfaces.

## Promotion Rules

| Field | Authority Source | Freshness Window | Promotion Destination | Target Surfaces | Notes |
|-------|----------------|------------------|----------------------|-----------------|-------|
| ARR | Salesforce (fact) | 30 days | accounts.arr + account_source_refs | Hero vitals, reports, health scoring | User override always wins |
| Lifecycle | Salesforce/Glean CRM (fact) | 7 days | accounts.lifecycle + lifecycle_changes | Hero vitals, briefing attention | Auto-transition via I623 |
| Renewal Date | Salesforce (fact) | 30 days | accounts.contract_end + account_source_refs | Hero vitals, renewal countdown | User override always wins |
| NPS | Survey tool (fact) | 90 days | accounts.nps + account_source_refs | Hero vitals, health scoring | |
| Champion | User designation (fact) | Indefinite | account_stakeholders.role | Stakeholder gallery, health scoring, prompts | SACRED -- never overwritten |
| Products | Glean/AI (inference) | 14 days | account_products | Products chapter | User correction promotes to fact |
| Support Tier | Zendesk (fact) | 7 days | account_technical_footprint | State of Play, Outlook | |
| CSAT | Zendesk (signal) | 30 days | account_technical_footprint | State of Play | |
| Open Tickets | Zendesk (signal) | 1 day | account_technical_footprint | State of Play, meeting prep | |
| Usage/Adoption | Telemetry (signal) | 7 days | account_technical_footprint | Outlook, health scoring | |
| Services Stage | Internal (fact) | 30 days | account_technical_footprint | State of Play | |

## Source Priority (ADR-0098)

User (4) > Clay (3) > Glean/Google/Gravatar (2) > AI (1)

## Freshness Rules

- Facts within freshness window: treat as ground truth
- Facts beyond freshness window: flag as potentially stale, still use but note age
- Signals beyond freshness window: suppress from active surfaces, move to historical
- Inferences beyond freshness window: eligible for re-inference on next enrichment
