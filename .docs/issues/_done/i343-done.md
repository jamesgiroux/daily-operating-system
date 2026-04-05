# I343 — Inline Editing — Field Drawers Replaced

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0 (partial work in v0.12.1, completed in v0.13.0)
**Area:** UX

## Summary

Account and project detail pages previously used drawer components (`AccountFieldsDrawer`, `ProjectFieldsDrawer`) for editing entity fields — tapping a field would open a side drawer with a form, requiring multiple interactions to edit a single value. This issue replaced all field drawers with inline editing: clicking a field makes it editable in place, with Tab/Enter navigation between fields. This aligns with the editorial magazine aesthetic where the page itself is the editor, not a separate form.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Open any account detail page. Click an editable field (name, health status, notes, any custom metadata field). A drawer does not open. The field becomes editable inline.
2. Same for any project detail page.
3. `AccountFieldsDrawer` and `ProjectFieldsDrawer` do not exist anywhere in the codebase — confirm with `grep -r "AccountFieldsDrawer\|ProjectFieldsDrawer" src/`.
4. Tab and Enter keyboard navigation work within inline fields.

## Dependencies

- `EditableText` and `EditableList` components (introduced in v0.8.0 for the risk briefing) were the foundation for this work.
- Partial work done in v0.12.1; completed in v0.13.0.

## Notes / Rationale

Drawers are a form pattern. The editorial magazine aesthetic doesn't use forms — it uses click-to-edit prose. The inline editing model matches how a user thinks about editing a report: you click the word, change it, and move on. No drawer, no Save button, no context switch.
