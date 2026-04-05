# ADR-0017: Pure Rust archive (no three-phase)

**Date:** 2026-02
**Status:** Accepted

## Context

The nightly archive moves `_today/` files to `_archive/YYYY-MM-DD/`. This is a file operation — no AI enrichment needed.

## Decision

Archive is pure Rust. No Python, no three-phase pattern. The Rust backend handles file moves directly.

## Consequences

- Simpler and faster than invoking Python or Claude
- No dependency on Python for a core daily operation
- Archive is the only workflow that breaks the three-phase pattern — justified because it doesn't need AI
