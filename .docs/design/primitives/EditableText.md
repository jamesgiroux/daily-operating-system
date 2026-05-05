# EditableText

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `EditableText`
**`data-ds-spec`:** `primitives/EditableText.md`
**Variants:** display tag; `multiline=true | false`; placeholder; hover/editing states
**Design system version introduced:** 0.5.0

## Job

Render shipped text as normal editorial copy until the user clicks it, then swap to a matching text input or textarea and commit on blur, Tab, or Enter in single-line mode.

## When to use it

- Generated or user-editable prose on meeting, entity, and report surfaces.
- Editable titles, one-liners, findings, commitments, report rows, and evidence copy.
- Text that should remain visually calm until the user intends to edit.

## When NOT to use it

- Full form settings rows; use `FormRow` with a normal input.
- Planned pencil-style inline fields; that is the future `InlineInput` roadmap primitive.
- Rich text or markdown editing.

## Source

- **Code:** `src/components/ui/EditableText.tsx`
- **Styles:** `src/components/ui/EditableText.module.css`

## Surfaces that consume it

MeetingDetail, AccountDetail, ProjectDetail, PersonDetail, report pages, WorkSurface, and several slide/report components.

