# BookOfBusinessPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `BookOfBusinessPage`
**`data-ds-spec`:** `surfaces/BookOfBusinessPage.md`
**Source files:**
- `src/pages/BookOfBusinessPage.tsx`
- `src/pages/report-slides.module.css`

## Job

BookOfBusinessPage renders a leadership-ready portfolio review for the user's customer book. It summarizes account health, retention risk, expansion, save motions, themes, quarterly focus, and leadership asks.

## Layout regions

1. Report slide shell with portfolio title and regeneration controls.
2. Executive summary and health overview.
3. Risk table, retention deep dives, and save motions.
4. Expansion, year-end outlook, landing scenarios, and account focus.
5. Quarterly focus, key themes, and leadership asks.

## Patterns and primitives

Consumes the shared report-slide module, book-of-business slide components, feedback controls, table-like report sections, status badges, and `FinisMarker`.

## States

Supports loading, generating, cached report, stale report, schema normalization, empty portfolio, error, regeneration, save feedback, and completed report states.
