# From markdown to JSON to SQLite to claims

**Status:** stub. Drafting TBD.

## Synopsis

DailyOS has had several "source of truth" eras. Markdown was right for human legibility. JSON was right for structured machine reads. SQLite became the working store when the system needed joins, freshness, and incremental updates. Claims emerged when even rows were too blunt to express provenance, supersession, and trust. This entry is the architecture story of that progression.

## Outline

- The lived moment: realising that human-readable files were useful but not enough for a system that needed to reason continuously.
- Markdown phase: great for authoring and reflection, weak as operational substrate.
- JSON phase: structure helped, but file-based state still made freshness and composition awkward.
- SQLite phase: the app needed a real working store, not a glorified cache.
- Claims phase: once provenance, supersession, and trust mattered, rows were not expressive enough on their own.
- What's still open: what should remain human-editable, what should remain query-native, and where claims should stop.

## Related ADRs

- ADR-0004 (markdown-first workflow roots)
- ADR-0048 (structured JSON backing for intelligence)
- ADR-0057 (entity intelligence JSON schema)
- ADR-0105 (SQLite as durable app substrate)
- ADR-0113 (claim ledger and supersede semantics)
