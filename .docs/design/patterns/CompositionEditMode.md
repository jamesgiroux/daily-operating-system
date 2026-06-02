# CompositionEditMode

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-06-02
**`data-ds-name`:** `CompositionEditMode`
**`data-ds-spec`:** `patterns/CompositionEditMode.md`
**Variants:** view; edit; locked core; reset
**Design system version introduced:** 0.5.0
**Source:** `.docs/plans/v1.5.0-w2-l0-packet.md`

Composition edit mode is a surface-local editing layer over a projected composition. It lets the user reorder, hide, restore, and choose renderer variants without mutating the substrate-authored `Composition`, claims, provenance, trust, or sensitivity state.

## Structure

- Entry lives on the rendered surface near folio actions as `Customize`.
- The page remains the primary canvas; controls appear in context on hover/focus and while edit mode is active.
- A compact status strip shows save state and Reset.
- Core masthead/lead content stays locked and visible.
- Hidden non-core blocks are restored through `CompositionInserter`.

## Behavior

- Live changes update the open page optimistically.
- Saves are latest-wins; stale responses cannot roll back a newer local layout.
- Reset restores the shipped default layout for the entity type/surface key.
- Claim-backed text edits use feedback/correction routes. Overlay JSON stores only presentation preferences.

## Accessibility

- Every edit control has an accessible name.
- Reorder instructions are available without hover.
- Focus remains on the moved item after drop and returns to the invoking control after reset/cancel.
- Locked controls explain why they are unavailable without relying on tooltip-only text.
