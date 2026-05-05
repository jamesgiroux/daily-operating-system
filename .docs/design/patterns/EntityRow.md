# EntityRow

**Tier:** pattern
**Status:** canonical/shipped
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `EntityRow`
**`data-ds-spec`:** `patterns/EntityRow.md`
**Variants:** accent dot; avatar; nested depth; border/no border; nameSuffix; subtitle; right meta slot
**Design system version introduced:** 0.5.0

## Job

Render an entity list item with a scannable title, optional `nameSuffix`, optional subtitle, accent/avatar identity, nesting, and right-aligned metadata. Accounts can use `HealthBadge` as the avatar when intelligence health exists; People rows use avatar rings; nested account/project rows use indentation.

## Source

- **Code:** `src/components/entity/EntityRow.tsx`
- **Styles:** `src/components/entity/EntityRow.module.css`

## Surfaces that consume it

AccountsPage, PeoplePage, and ProjectsPage.
