# ADR-0023: /wrap replaced by post-meeting capture

**Date:** 2026-02
**Status:** Accepted

## Context

The CLI had `/wrap` — a batch end-of-day closure ritual. In practice, batch closure at day's end is unnatural and easily skipped.

## Decision

Replace `/wrap` with per-meeting post-meeting capture. Most wrap functions (archive, reconciliation) happen automatically in the background. The interactive piece — capturing wins, risks, and actions — happens naturally after each meeting via a notification prompt.

## Consequences

- More natural timing (after each meeting vs. end of day)
- Capture is optional — missing it creates no debt (zero-guilt)
- Background archive (ADR-0017) handles the mechanical cleanup that /wrap used to do
- See PRD.md Appendix A for the full transition analysis
