# ActionRow

**Tier:** pattern
**Status:** canonical/shipped
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ActionRow`
**`data-ds-spec`:** `patterns/ActionRow.md`
**Variants:** `compact`, `full`, `outcome`
**Design system version introduced:** 0.5.0

## Job

Render a work action at the density required by its surface: compact linked row, full action-list row, or meeting outcome triage row.

## Source

- **Code:** `src/components/shared/ActionRow.tsx`
- **Extraction note:** source is shared and shipped, but still carries inline styles. CSS extraction remains a cleanup target, not a reason to omit it from the design system.

## Surfaces that consume it

ActionsPage uses `variant="full"` for action lists. MeetingDetailPage uses `variant="outcome"` for meeting outcome triage. `src/components/entity/TheWork.tsx` uses `variant="compact"` for linked Work rows.
