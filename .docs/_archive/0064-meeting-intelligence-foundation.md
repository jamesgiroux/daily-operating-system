# ADR-0064: Meeting intelligence report layout

**Date:** 2026-02-12
**Status:** Accepted

## Context

Meeting prep is the product’s north-star experience, but the previous stack of talking points, duplicate sections, and unstructured output made it hard to surface directed guidance. The next phase of meeting intelligence needs a consistent report-grade layout (hero + agenda + outcomes), explicit data plumbing for agenda/wins, and an enrichment anchor so AI synthesis always scales with the user’s commitments.

## Decision

- Adopt a three-tier layout on `MeetingDetailPage` with (1) an executive hero (title, timeline, primary signals), (2) an agenda-first “report” surface (agenda/topic summary, intelligence summary, risks/actions/questions), and (3) a deep appendix for historical wins/outcomes/attachments.
- Split agenda/win semantics in the data model (`recentWins`, `recentWinSources`, `proposedAgenda`) and ensure enrichment parsers respect the distinction so prompts can anchor on “agenda-first” content before falling back to wins.
- Bake the agenda-anchoring logic into the enrichment flow so AI completions (I188) know which inputs are agenda, which are wins, and which are open questions, then pipe those outputs into the prep detail, the preview cards (ADR-0063), and the stored `prep_context`.

## Consequences

- The data layer now needs backfill scripts (`backfill_prep_semantics`) and migrations to populate the new agenda/wins fields, increasing the divergence between older prep files and the enriched model.
- The UI gains clarity but also requires careful layout choices (accordion/expansion states) to expose the three tiers without overwhelming the user.
- Enrichment prompts need guardrails that respect the agenda-first policy; we must monitor for regressions when the model drifts.

## Related issues

- [I187](../BACKLOG.md#i187) Prep page three-tier layout (ADR-0064 P3)
- [I188](../BACKLOG.md#i188) Agenda-anchored AI enrichment (ADR-0064 P4)
