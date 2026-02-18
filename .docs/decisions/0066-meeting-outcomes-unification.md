# ADR-0066: Meeting outcomes are integrated with prep

**Date:** 2026-02-12
**Status:** Accepted

## Context

Outcomes used to live in a separate post-meeting bucket, making follow-up tracking and dashboard summarization harder. To close the loop, the meeting detail view should treat outcomes as part of the same card as the prep, surfacing the most recent stakes and progress markers up top so the intelligence narrative stays cohesive.

## Decision

- Treat outcomes as first-class data alongside the prep context and render them in the shared MeetingDetail layout (`MeetingOutcomes` component sits below the prep hero but remains visually connected via spacing/SectionLabel).
- Ensure outcomes refresh whenever the prep is retrieved so we persist a single cohesive payload (no more separate “wrap” step).
- Surface outcomes inside the account detail meeting preview and archive history, so they are visible even when the prep is collapsed.

## Consequences

- Meeting detail now has two entry points for future-meeting context (prep) and past successes (outcomes), which means the UI needs to manage dual refresh flows.
- Since outcomes are now present in the preview cards and hero, we must ensure they still respect access controls (e.g., transcripts remain gated until the meeting occurs).
- The unified view simplifies follow-ups and dashboard cards because Outcome data flows through the same `MeetingDetailPage` pipeline.

## Related issues

- [I191](../BACKLOG.md#i191) Card-detail unification (ADR-0066 P2-3)
- [I195](../CHANGELOG.md#i195) Outcomes surface inside prep/outcomes section
