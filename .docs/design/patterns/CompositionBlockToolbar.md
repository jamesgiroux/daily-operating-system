# CompositionBlockToolbar

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-06-02
**`data-ds-name`:** `CompositionBlockToolbar`
**`data-ds-spec`:** `patterns/CompositionBlockToolbar.md`
**Variants:** movable; locked; variant; hidden
**Design system version introduced:** 0.5.0

Composition block toolbar is the lightweight control rail shown for a projected block while composition edit mode is active.

## Anatomy

- Reorder handle (`CompositionReorderHandle`) when the block is movable.
- Visibility `Switch` for eligible non-core blocks.
- Variant `Segmented` control when the renderer supports more than the default presentation.
- Inline edit affordance only when the block exposes a feedback-allowed edit route.

## Rules

- The toolbar is visually quiet: mono labels, editorial rules, no decorative color.
- Controls reveal on hover/focus but remain reachable by keyboard.
- Disabled locked controls keep their accessible explanation in the DOM.
- Toolbar actions write layout overlay preferences only, except inline claim text edits, which route through feedback/correction.
