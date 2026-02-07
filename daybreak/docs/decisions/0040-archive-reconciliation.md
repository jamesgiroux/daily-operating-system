# ADR-0040: Archive reconciliation (end-of-day mechanical cleanup)

**Date:** 2026-02-06
**Status:** Accepted
**Evolves:** [ADR-0017](0017-pure-rust-archive.md) (pure Rust archive)

## Context

ADR-0023 decomposed `/wrap` into per-meeting capture (interactive) + background archive (mechanical). The archive workflow (ADR-0017) handles file moves, but several deterministic reconciliation steps from `/wrap` had no home:

1. **Transcript status check** — For each completed meeting with a recording, check whether the transcript was processed (in canonical location), still in `_inbox/`, or missing entirely.
2. **Prep status reconciliation** — Mark completed meetings as "Done" in the week overview. This is a file operation: calendar says meeting ended → update the prep status column.
3. **Daily summary generation** — `/wrap` generated `wrap-summary.md` in the archive directory: meetings completed, actions reconciled, gaps flagged. This is a synthesis of already-known data, not AI enrichment.
4. **Gap flagging** — Unprocessed transcripts and incomplete capture get surfaced in the next morning's briefing as attention items.

These are all deterministic Rust operations — no AI, no user interaction. They fit naturally in the archive workflow that already runs at end of day.

## Decision

The archive workflow gains mechanical reconciliation steps, executed in sequence:

1. **Reconcile completed meetings** — Query calendar for today's completed meetings. For each: check transcript status (canonical location vs. `_inbox/` vs. missing), update prep status in week overview to "Done."
2. **Generate daily summary** — Write `archive/YYYY-MM-DD/day-summary.json` with: meetings completed (with transcript/prep status), actions completed today, actions added today, unresolved gaps.
3. **Flag attention items** — Write unresolved items (missing transcripts, unprocessed inbox files) to a `next-morning-flags.json` that the next briefing's Phase 1 can read and surface.
4. **Archive files** — Existing file-move behavior (unchanged).

The reconciliation runs before file archival so it can read `_today/` state. All steps are optional — if calendar data is unavailable or week overview doesn't exist, skip gracefully.

Still pure Rust, no three-phase. ADR-0017's core constraint holds: the archive workflow doesn't need AI.

## Consequences

- Closes the gap between what ADR-0023 promised ("/wrap's mechanical parts happen automatically") and what was built (just file moves)
- Next-morning flags create a feedback loop: today's gaps surface in tomorrow's briefing
- Week overview stays accurate without manual reconciliation
- Archive directories contain structured summaries, not just raw files — useful for `/week` and `/month` rollups
- Adds complexity to the archive workflow, but all steps are independently skippable
