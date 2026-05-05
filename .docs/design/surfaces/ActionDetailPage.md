# ActionDetailPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ActionDetailPage`
**`data-ds-spec`:** `surfaces/ActionDetailPage.md`
**Source files:**
- `src/pages/ActionDetailPage.tsx`
- `src/pages/ActionDetailPage.module.css`
**Routes:** `/actions/$actionId`

## Job

ActionDetailPage is the single-commitment inspection and editing surface. It lets the user change priority, status, context, due date, linked account, source, and Linear push state without leaving the action workflow.

## Layout Regions

1. Folio chrome with action breadcrumb and saving status.
2. Title band with complete/reopen control and editable serif title.
3. Status strip for priority, open/completed state, waiting-on, and source badge.
4. Priority picker row and separator.
5. Context editor with auto-generated note when applicable.
6. Reference rows for account, due date, created/completed dates, and source.
7. Optional Linear section for issue linking or push.
8. Bottom action bar with save feedback and status toggle.

## Patterns And Primitives

Consumes `EditableText`, `EditableTextarea`, `EditableInline`, `EditableDate`, `EntityPicker`, `PriorityPicker`, mono badges, account chip, and the terracotta action color system.

## States

Supports loading skeleton, error retry, open, completed, auto-generated, account-linked, account-empty, source-linked, source-manual, Linear disabled, Linear push-ready, Linear linked, saving, and saved states.

