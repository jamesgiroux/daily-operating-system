# CompositionInserter

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-06-02
**`data-ds-name`:** `CompositionInserter`
**`data-ds-spec`:** `patterns/CompositionInserter.md`
**Variants:** hidden block; hidden section; empty non-core
**Design system version introduced:** 0.5.0

Composition inserter restores hidden projected blocks or sections. It does not create new authored content.

## Structure

- Appears at the end of edit mode sections when hidden items exist.
- Lists hidden non-core items by section/block label.
- Uses text buttons with a plus icon from the app icon set.

## Behavior

- Re-adding an item removes its hidden id from the layout overlay.
- Stale hidden ids from old projections are ignored.
- If every non-core item is hidden, the surface renders core content plus a quiet reset affordance.
