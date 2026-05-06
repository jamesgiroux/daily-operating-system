# BriefingErrorState

**Tier:** pattern
**Status:** proposed
**Owner:** DOS-429 (W5, ships TSX as part of DailyBriefingRedesign.tsx)
**Last updated:** 2026-05-06
**`data-ds-name`:** `BriefingErrorState`
**`data-ds-spec`:** `patterns/BriefingErrorState.md`
**Reference render:** `.docs/design/reference/surfaces/briefing-redesign-error.html`

## Job

Render the briefing when data assembly failed. Triggered when `BriefingLoadState.status === "error"`. Centers an editorial error frame with recovery affordances and diagnostic meta.

## Anatomy

Centered single-column stage:

```
┌────────────────────────────────────┐
│                                    │
│       BRIEFING UNAVAILABLE         │  ← mono 11px caps, terracotta
│                                    │
│   We couldn't load your briefing.  │  ← serif 28px (BriefingLoadState.error.message)
│                                    │
│   A signal source isn't            │  ← serif italic 17px
│   responding. Your day is still    │     (BriefingLoadState.error.detailMessage)
│   on the calendar — we just        │
│   can't shape it into a briefing   │
│   right now.                       │
│                                    │
│   [Try again]  [Diagnostics]       │  ← ui-button-default + ui-button-secondary
│                                    │
│   code: dependency_failed ·        │  ← mono 10px, tertiary
│   service: predictions             │
│                                    │
└────────────────────────────────────┘
```

## Variants

Single variant. Surface contents driven by `BriefingLoadState.error` fields:
- `message` → primary headline
- `detailMessage` → secondary detail sentence (optional)
- `code` → meta line code segment (optional)
- `service` → meta line service segment (optional)

## Composition rules

- Centered max-width 640px column.
- Error eyebrow color tied to `--color-spice-terracotta`.
- "Try again" button triggers a refetch via the parent hook (DOS-429's `useBriefingViewModel().refresh()`).
- "Diagnostics" link points to `/settings#diagnostics` (existing surface).

## What it doesn't do

- Stack-trace exposure. The contract carries `code` + `service` as user-readable diagnostics; raw error strings beyond `message` and `detailMessage` are not rendered.
- Auto-retry. The user explicitly clicks "Try again."

## Open questions

- Should `service` link to a service-specific diagnostics page if available? Deferred until DOS-429 builds.

## Spec status

**proposed** — TSX ships in W5 as part of DOS-429. Reference HTML at `briefing-redesign-error.html` is the canonical render today.
