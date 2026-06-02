# CompositionReorderHandle

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-06-02
**`data-ds-name`:** `CompositionReorderHandle`
**`data-ds-spec`:** `patterns/CompositionReorderHandle.md`
**Variants:** mouse; keyboard; disabled locked
**Design system version introduced:** 0.5.0

Composition reorder handle is the drag target for changing block or section order in edit mode.

## Keyboard Contract

- Space grabs the focused item.
- Arrow keys move it within the current sortable group.
- Space or Enter drops it.
- Escape cancels and restores focus.

## Accessibility

- The handle names the item it moves.
- Screen-reader announcements describe start, move, drop, and cancel.
- Instructions are available through visible helper text or `aria-describedby`, not hover-only tooltips.
- Locked masthead/lead content renders a disabled handle with a non-hover explanation.
