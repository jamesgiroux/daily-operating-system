# Patterns

Composed, opinionated, named after the job they do. **This is where drift happens** — when two surfaces re-implement nearly-identical UI with small variations, that's a missing pattern. Promote it.

A pattern knows about a domain concept (a claim, a trust state, a briefing, a meeting). A primitive doesn't.

## Index

_(populated as patterns are promoted)_

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| _(awaiting Audit 03 + Audit 04 findings)_ | | | |

Strong promotion candidates the audits will likely surface (verify before adding):

- `TrustBand` — render claim trust state with appropriate freshness/confidence affordances
- `ClaimRow` — single-claim display with provenance, value, source
- `BriefingSpine` — vertical structural rail of the daily/meeting briefing
- `LocalNavIsland` — floating local-nav for surfaces (current candidate from `_shared/chrome.js`)
- `AskAnythingDock` — query input from D-spine briefing
- `ReceiptCallout` — expandable receipt for a claim (v1.4.4)
- `FreshnessChip` — compact freshness indicator
- `ProvenancePill` — source attribution

## Conventions

- **Named after the job, not the surface.** `TrustBand` not `BriefingTrustBand`. If a pattern is unique to one surface, it's probably surface-internal and doesn't need promotion yet.
- **PascalCase.** No suffixes like `Component`, `Container`, `Wrapper`.
- **Composes primitives.** A pattern that doesn't compose primitives is suspicious — it might be a primitive itself.
- **Has a clear API.** Pattern specs document the input/output/customization surface. Avoid "it works one way at one place."
- **Variants are first-class.** A pattern with 4 variants is fine. A pattern with 4 forks across 4 surfaces is a bug.

## Adding a pattern

1. Confirm: does this appear (or will it appear) in 2+ surfaces? If only one surface uses it, it's surface-internal.
2. Copy `../_TEMPLATE-entry.md` here.
3. Fill out **Composition** (which primitives) and **Variants** carefully — these are the contract.
4. List every consuming surface in **Surfaces that consume it**.
5. If you're consolidating drift, note the previous variants and where they lived in **History**.

## Reviewing for drift

If you're auditing or reviewing a PR, ask:

1. Is this UI also rendered somewhere else with small differences? → likely a missing pattern.
2. Does this pattern's spec match every consumer's actual usage? → if not, the spec is stale or the consumers drifted.
3. Did a recent PR add a "private" component that should be a pattern? → promote it.
