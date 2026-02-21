# ADR-0087: Entity Hierarchy Intelligence — Portfolio Surfaces, Partner Type, Bidirectional Propagation

**Date:** 2026-02-21
**Status:** Accepted
**Extends:** ADR-0086 (Intelligence as Shared Service)

**Extends:** ADR-0079 (Role Presets) — the `entityModeDefault` field in each preset determines which entity type receives the portfolio surface as primary. This ADR implements the intelligence architecture; ADR-0079 governs which entity type it applies to for a given role.

---

## Context

The current entity model treats all accounts as structurally equivalent: a parent account is a container that holds child accounts. It has no intelligence of its own beyond what's manually entered. A user working across 10 Salesforce BUs under one parent company has no surface that shows them the portfolio view — what's happening across all 10 at once, whether a pattern appearing in three BUs is worth escalating, which BUs need attention today vs. which can wait.

Additionally, DailyOS has no concept of a partner entity — agencies, SIs, channel partners, and consulting firms are currently lumped with customer accounts or left untyped. The intelligence prompt shape for a customer (health, spend, renewal risk) is wrong for a partner (alignment, joint deliverables, communication cadence). The AccountsPage gives no visual separation between the accounts you sell to, the internal teams you coordinate with, and the agencies you work alongside.

Parent-level meetings compound the problem. A meeting tagged to "Cox Enterprises" might contain signals about Cox B2B, Cox Retail, and the parent entity simultaneously. Currently the signal from that meeting lands on one entity (whichever the meeting is tagged to) and stays there.

Signal flow is also currently one-directional at the hierarchy level: person signals propagate to their linked accounts, but BU signals don't propagate to parent accounts, and parent-level intelligence doesn't cascade down to BUs.

---

## Decisions

### 1. Partner as a first-class entity type

`partner` joins `customer` and `internal` as a valid `account_type`. A partner is an entity you coordinate *with* but don't sell *to*: agencies, SIs, consulting firms, channel partners. The distinction matters because:

- Partners carry different intelligence signals (alignment, deliverable tracking, communication cadence) vs. customer health/renewal signals
- Partners should be visually distinct on account surfaces — badged similarly to internal accounts but with their own treatment
- Meeting classification can recognize partner domains and contextualize attendees correctly

### 2. AccountsPage grouped into three surfaces

The AccountsPage splits into three named groups matching the three entity types:

- **Your Book** — customer accounts (what you sell to and are responsible for)
- **Your Team** — internal accounts (cross-functional relationships, internal stakeholders)
- **Your Partners** — partner accounts (agencies, SIs, channel partners)

Empty groups do not render. Each group preserves the existing parent/child hierarchy within it. Search applies across all groups simultaneously.

### 3. Parent account intelligence is a two-layer portfolio surface

A parent account (any account with child accounts) has a qualitatively different `intelligence.json` from a leaf-node account:

**Layer 1 — Portfolio synthesis (from children):** The AI enrichment prompt for a parent includes the current `intelligence.json` of every direct child. The resulting intelligence includes:
- Portfolio health assessment (which children are healthy, at-risk, expanding)
- Hotspot accounts (children with active risk or opportunity signals, sorted by urgency)
- Cross-BU patterns (signal types or topics appearing in 2+ children — these are portfolio-level observations the user can't see from any individual BU page)
- Portfolio narrative (AI-synthesized executive view of the relationship)

**Layer 2 — Own signals (from parent-tagged meetings and user edits):** Meetings tagged directly to the parent entity, and any user edits on the parent entity itself, produce signals that belong at the parent level. These feed parent enrichment as own context, separate from the portfolio synthesis.

Leaf-node (child) accounts are unaffected. Their `intelligence.json` shape and detail page layout remain unchanged. The portfolio section only appears on accounts with children.

### 4. Bidirectional signal propagation in the entity hierarchy

The existing signal propagation model (person → linked account) is extended to the entity hierarchy:

**Upward (BU → parent):**
Child account signals propagate to their parent at 60% of the original confidence. This means:
- A single routine child signal (low confidence) propagates to the parent below the enrichment threshold — no parent refresh triggered
- A single high-significance child signal (high confidence) crosses the threshold alone — parent refreshes
- Multiple child signals of the same type accumulate at the parent via Bayesian fusion — the "multiple BUs" behavior emerges naturally from confidence accumulation, not a hard counter

The parent's intel_queue trigger is the same mechanism as any entity: new signals since last enrichment. No special threshold logic needed.

**Downward (parent → BU):**
Parent signals above a confidence threshold (default: 0.7) fan out to all direct children at 50% of the parent confidence. This ensures significant parent-level events (account-wide strategy shift, new executive sponsor, spend freeze) appear in every BU's intelligence without dominating it.

Downward propagation stops at direct children by default (does not cascade to grandchildren). This is a configurable propagation rule, not hardcoded.

Propagation loops are prevented by the existing mechanism: derived signals are not re-propagated in the same cycle.

### 5. The portfolio model generalizes across entity types via ADR-0079

The two-layer parent intelligence model (own signals + portfolio synthesis) and bidirectional propagation apply to **any entity type with a parent/child relationship**, not only accounts. The active role preset's `entityModeDefault` determines which entity type receives the portfolio surface as primary:

| Entity mode | Primary portfolio surface | Example preset |
|-------------|--------------------------|---------------|
| `account` | Parent account → BU hierarchy | Customer Success, Sales |
| `project` | Parent project → campaign/workstream hierarchy | Marketing, Product |
| `both` | Both account and project hierarchies | Partnerships, Leadership, Agency |

The v0.13.3 implementation focuses on accounts (the dominant current use case). Project hierarchy intelligence follows in v0.13.4 using the identical architecture with project-appropriate vocabulary and prompt shape. People are explicitly excluded from this model — people don't have a tree hierarchy in DailyOS, they have a relationship network, which requires a different architectural approach not covered by this ADR.

### 6. Multi-entity signal extraction is a future concern

When a parent-level meeting contains signals for multiple BUs ("for Cox B2B we're doing X, for Cox Retail we're doing Y"), the correct behavior is to extract and route signals to the appropriate child entities. This requires content-level entity resolution in the transcript processor — a meaningful pipeline addition.

This is explicitly deferred. The expected user behavior is to tag meetings at the parent level rather than tagging every relevant child. The bidirectional propagation model (decision 4) partially addresses this: parent-tagged meetings produce parent-level signals that cascade down to children via fan-out. Content-level multi-entity extraction is a refinement, not a prerequisite.

---

## Consequences

- Parent accounts with many children incur higher enrichment cost: the AI prompt must include all children's intelligence. This is bounded — a parent with 20 children still produces one AI call, not 20.
- Downward propagation means a significant parent-level signal will appear in all BU intelligence. Users may occasionally see parent context in a BU where it's not highly relevant. The 50% confidence decay and the 0.7 threshold gate should keep noise low.
- The "Your Book / Your Team / Your Partners" grouping requires all existing accounts to have an explicit `account_type`. Accounts created before this change without a type default to `customer`. Migration handles this.
- Partner intelligence prompts need to be written separately from customer and internal prompts. This is new prompt engineering work but follows the existing preset/prompt pattern.
