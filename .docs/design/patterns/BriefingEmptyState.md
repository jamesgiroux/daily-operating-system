# BriefingEmptyState

**Tier:** pattern
**Status:** proposed
**Owner:** DOS-429 (W5, ships TSX as part of DailyBriefingRedesign.tsx)
**Last updated:** 2026-05-06
**`data-ds-name`:** `BriefingEmptyState`
**`data-ds-spec`:** `patterns/BriefingEmptyState.md`
**Reference render:** `.docs/design/reference/surfaces/briefing-redesign-empty.html`

## Job

Render the briefing when the user has not yet connected enough data sources to assemble one. Triggered when `BriefingLoadState.status === "empty"`. Editorial framing of "what DailyOS needs" with a primary connect-first action plus optional checklist.

## Anatomy

Left-aligned 640px column, editorial register:

```
┌────────────────────────────────────────┐
│ DAILY BRIEFING                         │  ← mono 11px caps, tertiary
│                                        │
│ Your day, when DailyOS                 │  ← serif 36px
│ can read it.                           │
│                                        │
│ The briefing is a synthesis of...      │  ← serif italic 19px lede
│                                        │
│ ○  Connect Google to bring in...       │  ← checklist (BriefingEmptyChecklistItem[])
│ ○  Optional: Glean for cross-tool...   │
│ ○  Optional: Claude Code to enable...  │
│                                        │
│ [Connect Google]                       │  ← ui-button-lg
│                                        │
└────────────────────────────────────────┘
```

## Variants

Single variant. Surface contents driven by `BriefingLoadState.empty` fields:
- `message` → lede paragraph (or eyebrow + headline + lede if message includes structure — view splits at first `\n\n`)
- `googleAuth` → presence triggers the "Connect Google" CTA
- `checklistItems[]` → optional checklist (each item: `label` + optional `status`)

## Composition rules

- Eyebrow + headline are pattern-fixed editorial copy ("DAILY BRIEFING" + "Your day, when DailyOS can read it."). Customizable via service in the future if more empty-state variants emerge.
- Checklist items use `○` / `●` glyphs based on `status` ("todo" / "done").
- "Connect Google" CTA only renders when `googleAuth` is present and not authenticated.

## What it doesn't do

- Detect or trigger Google auth. The button click delegates to the existing `connectGoogleAuth()` hook.
- Render the FolioBar's readiness pairs. In empty state, the folio is bare (no readiness data to show).

## Open questions

- Should partial-connection states (Google authed but no Glean) get a tailored empty surface, or fall through to success with degraded fields? Current contract leans toward success+degraded; this empty state is for the cold-start case only.

## Spec status

**proposed** — TSX ships in W5 as part of DOS-429. Reference HTML at `briefing-redesign-empty.html` is the canonical render today.
