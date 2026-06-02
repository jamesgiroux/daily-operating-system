# CompositionInlineEdit

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-06-02
**`data-ds-name`:** `CompositionInlineEdit`
**`data-ds-spec`:** `patterns/CompositionInlineEdit.md`
**Variants:** claim-backed; display-only fallback; failed correction
**Design system version introduced:** 0.5.0

Composition inline edit wraps `EditableText` for projected composition text that has a feedback-allowed edit route.

## Eligibility

- The projected block must expose an `edit_routes` entry with `feedback_allowed: true`.
- The route must include at least one claim ref.
- Display-only, computed-only, or source-only routes do not expose inline edit.

## Behavior

- The visible text edits inline using `EditableText`.
- Commit submits a correction through the existing intelligence correction/feedback command path with entity id, entity type, field path, claim id, current value, and corrected value.
- Failed submissions revert the optimistic visible edit.
- Overlay JSON never stores claim text replacements.

## Presentation Labels

Section/display label overrides may live in the layout overlay when they are not claim text. They use the same inline editing affordance but save to overlay label preferences, not correction feedback.
