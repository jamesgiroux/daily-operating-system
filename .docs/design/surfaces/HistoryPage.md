# HistoryPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `HistoryPage`
**`data-ds-spec`:** `surfaces/HistoryPage.md`
**Source files:**
- `src/pages/HistoryPage.tsx`
- `src/pages/HistoryPage.module.css`
**Routes:** `/history`

## Job

HistoryPage shows the user what DailyOS processed, how each file was classified, whether processing succeeded, where the result landed, and when it happened.

## Layout Regions

1. Folio chrome labeled Processing History.
2. Editorial page header with entry count.
3. Fixed five-column table header.
4. Processing rows with filename, class, status, destination, and timestamp.
5. Error detail row when a processing attempt fails.
6. Empty state with return-to-inbox action.
7. Finis marker when entries exist.

## Patterns And Primitives

Consumes `EditorialPageHeader`, `EditorialLoading`, `EditorialError`, `EmptyState`, `FinisMarker`, mono cells, status dots, and truncation/no-wrap utilities.

## States

Supports loading, error, empty, success row, error row, destination present, missing destination, and mixed history lists.

