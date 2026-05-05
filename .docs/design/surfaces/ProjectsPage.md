# ProjectsPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ProjectsPage`
**`data-ds-spec`:** `surfaces/ProjectsPage.md`
**Source files:**
- `src/pages/ProjectsPage.tsx`
- `src/components/entity/EntityListShell.tsx`
- `src/components/entity/EntityRow.tsx`

## Job

ProjectsPage is the project index surface. It lets the user scan active and archived projects, search them, and enter a project dossier.

## Layout regions

1. Folio chrome with project count and new-project action.
2. Entity list header with active/archive tabs and search.
3. Recursive project tree rows with owner/context metadata.
4. Empty, filtered-empty, and create states.

## Patterns and primitives

Consumes `EntityListShell`, `EntityRow`, `EditorialPageHeader`, list tabs, search input, status dots, and row metadata.

## States

Supports loading, active/archive filter, search, nested projects, no projects, and create-project flow states.
