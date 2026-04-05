# ADR-0028: Structured document schemas (JSON-first templates)

**Date:** 2026-02
**Status:** Accepted

## Context

Account dashboards, success plans, and other structured documents need mechanical updates (e.g., "Last Contact" date, "Recent Wins" list). Updating these via regex on markdown is fragile (per ADR-0004 rationale).

## Decision

Structured documents get JSON schemas. Rust reads JSON, applies structured updates, writes JSON back. Markdown is optionally regenerated from JSON for human readability. Extends ADR-0004 pattern from `_today/` ephemeral data to all persistent structured documents.

**Schema priority:**
- Account dashboard — High (updated after every meeting)
- Action items per account — High (bidirectional sync)
- Success plans — Medium (quarterly reviews)
- Impact capture — Medium (weekly roll-up)

**Not schematized:** raw transcripts, meeting summaries (prose), archive files (read-only), user notes (freeform).

**File layout:**
```
Accounts/Heroku/01-Customer-Information/
├── dashboard.json   # Machine-readable (app updates)
└── dashboard.md     # Human-readable (generated from JSON)
```

## Consequences

- Mechanical updates are reliable and testable
- Two files per document (JSON + MD) — the Phase 3 sync pattern
- Migration needed: existing markdown dashboards → JSON via one-time script
- Schemas live in `~/.dailyos/schemas/` or bundled with the CS extension
