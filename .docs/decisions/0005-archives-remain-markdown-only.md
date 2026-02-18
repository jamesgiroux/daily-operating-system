# ADR-0005: Archives remain markdown-only

**Date:** 2026-02
**Status:** Accepted

## Context

ADR-0004 introduced JSON for active `_today/` data. Question: should archived days also have JSON?

## Decision

No. Archives are markdown-only. JSON generation happens at runtime for active data only. Historical data is for human reference, not machine consumption.

## Consequences

- Simpler archive structure (just markdown files)
- Can't query historical data without re-parsing markdown
- If historical querying becomes important, a SQLite index over archives is the answer (not JSON files)
