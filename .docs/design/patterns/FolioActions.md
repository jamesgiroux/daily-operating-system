# FolioActions

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `FolioActions`
**`data-ds-spec`:** `patterns/FolioActions.md`
**Variants:** `meeting-detail`
**Design system version introduced:** `0.4.0`

## Job

Provide a compact action row directly beneath FolioBar so users can copy, share, send, or re-extract a generated surface without confusing those commands with chapter navigation.

## When to use it

- Beneath `FolioBar` when the surface has document-level actions.
- On MeetingDetail for Copy, Share, Send Recap, and Re-extract.
- When actions should remain visually subordinate to the folio chrome but above the surface masthead.

## When NOT to use it

- For section navigation; use `FloatingNavIsland`.
- For inline row actions such as Accept, Dismiss, or Mark complete; use local `Button` instances in that row.
- For primary body actions that belong inside a chapter rather than the folio area.

## States / variants

- `meeting-detail` — icon buttons for Copy, Share, Re-extract, plus primary turmeric Send Recap button.
- Hover — each button shows the standard button hover surface without shifting the row.
- Loading — Re-extract may show progress and become disabled while extraction runs.
- Disabled — unavailable commands dim and remain focusable only when they can explain why.

## Composition

Composes `Button` primitive instances in a horizontal sub-row: icon-only buttons for Copy, Share, and Re-extract; text primary button for Send Recap.

## Tokens consumed

- `--color-spice-turmeric` — primary Send Recap action.
- `--color-text-primary` — action foreground.
- `--color-text-tertiary` — subdued icon action foreground.
- `--border-subtle` — button boundary when present.
- `--space-xs`, `--space-sm`, `--space-md` — row gap and button padding.

## API sketch

```tsx
<FolioActions
  actions={[
    { id: "copy", icon: "copy", label: "Copy" },
    { id: "share", icon: "share", label: "Share" },
    { id: "send-recap", label: "Send Recap", tone: "primary" },
    { id: "re-extract", icon: "refresh-cw", label: "Re-extract" },
  ]}
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/FolioActions.tsx`
- **Mockup origin:** `.docs/mockups/claude-design-project/mockups/meeting/current/after.html` lines 29-34

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) canonical

## Naming notes

Canonical name is `FolioActions`. It is separate from `FloatingNavIsland`: FolioActions performs commands, while FloatingNavIsland navigates chapters.

## History

- 2026-05-03 — Proposed for Wave 4.
