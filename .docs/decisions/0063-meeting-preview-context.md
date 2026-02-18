# ADR-0063: Meeting preview cards carry prep context

**Date:** 2026-02-12
**Status:** Accepted

## Context

The account detail page lists upcoming and recent meetings with compact cards, but those cards previously showed only the basic title, type, and time. Analysts still need prep context before the meeting detail view loads, yet requesting the full prep for every listed meeting is expensive. We need a middle ground that surfaces the most actionable intelligence (agenda fragments, risks, open items) while leaving the deep dive for the dedicated prep page.

## Decision

- Extend `MeetingPreview` to optionally carry the trimmed `PrepContext` (intelligence summary, proposed agenda topics, risk/action/question counts) that already exists in `meetings_history` and prep JSON.
- When building the account detail payload, hydrate the preview cards with prep context for the most recent meetings. Reuse the `/meeting/$meetingId` data pipeline (I190) so the server can reuse the same disk/DB lookup that powers the meeting detail route.
- Render the prep context only when it exists and keep the layout compact: summary text, agenda chips, and signal counts appear beneath the header row and collapse naturally when the intelligence isn’t present.

## Consequences

- Account detail queries now read both meeting metadata and the associated prep context, so we throttle the materialized view to three meetings by default and ask for more only when users expand the list.
- Preview cards stay lightweight: missing prep context simply hides the extra section instead of showing placeholders.
- Because the preview cards link to the same meeting ID that the detail page uses, the routing migration from I190 ensures everything stays in sync.

## Related issues

- [I190](../BACKLOG.md#i190) Meeting route migration — `/meeting/$meetingId` plus the unified prep command are the data foundation for enriching previews.
