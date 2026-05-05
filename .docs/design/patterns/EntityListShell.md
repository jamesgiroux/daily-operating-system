# EntityListShell

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `EntityListShell`
**`data-ds-spec`:** `patterns/EntityListShell.md`
**Variants:** header/search; filter tabs; People/Projects archive toggle; Accounts lifecycle filters; empty/loading/error; end marker
**Design system version introduced:** 0.5.0

## Job

Provide the shared list-surface shell for entity indexes: editorial header, search, tabs, state handling, and `FinisMarker`.

Archive behavior differs by surface: People and Projects render `ArchiveToggle` in the header; Accounts keeps archive controls in folio actions and uses header tabs for active lifecycle filters only.

## Source

- **Code:** `src/components/entity/EntityListShell.tsx`
- **Styles:** `src/components/entity/EntityListShell.module.css`

## Surfaces that consume it

AccountsPage, PeoplePage, and ProjectsPage.
