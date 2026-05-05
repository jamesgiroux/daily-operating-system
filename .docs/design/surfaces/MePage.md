# MePage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `MePage`
**`data-ds-spec`:** `surfaces/MePage.md`
**Source files:**
- `src/pages/MePage.tsx`
- `src/pages/MePage.module.css`

## Job

MePage is the user's self context surface. It captures role, priorities, operating context, attachments, and profile metadata that shape briefings and recommendations.

## Layout regions

1. Folio chrome and local chapter navigation.
2. Editorial profile header.
3. About Me fields.
4. Priorities and context entries.
5. Attachments and supporting personal context.

## Patterns and primitives

Consumes `ChapterHeading`, `EditableText`, `ContextEntryList`, field rows, attachment rows, and report actions. Field editing remains surface-local until generalized.

## States

Supports loading, no-profile, empty priorities, empty attachments, edit, save-in-progress, and save-error states.
