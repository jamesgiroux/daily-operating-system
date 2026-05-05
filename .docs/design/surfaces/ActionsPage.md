# ActionsPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ActionsPage`
**`data-ds-spec`:** `surfaces/ActionsPage.md`
**Source files:**
- `src/pages/ActionsPage.tsx`
- `src/pages/ActionsPage.module.css`

## Job

ActionsPage is the user's commitment and follow-up command surface. It groups suggested, active, and completed actions by status, priority, source context, and due date.

## Layout regions

1. Folio chrome with action counts and create affordance.
2. Editorial page header with status tabs, priority tabs, and search.
3. Grouped action columns organized by source and date.
4. Action rows with completion controls, title/context, metadata, and priority.
5. Empty states for no suggested, active, completed, or matching search results.

## Patterns and primitives

Consumes `ChapterHeading`, action-row reference classes, button controls, priority badges, and search input. New action-list primitives should be promoted only when reused outside this surface.

## States

Supports loading, empty, filtered-empty, overdue, active, suggested, completed, and completion-updating states.
