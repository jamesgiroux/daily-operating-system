# BriefingErrorState

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `BriefingErrorState`
**`data-ds-spec`:** `patterns/BriefingErrorState.md`
**Variants:** `default`
**Design system version introduced:** 0.6.0

## Job

Render an editorial-register error frame when a surface's primary data assembly fails. Centered single-column stage with eyebrow, headline, optional detail sentence, recovery affordances ("Try again" + "Diagnostics"), and optional diagnostic meta line (`code`, `service`). The pattern carries enough context for the user to know what went wrong without exposing stack traces, and gives them a one-click recovery path.

## When to use it

- Any editorial-register surface when the primary view-model failed to assemble
- When the failure is recoverable (transient: retry; or scoped: navigate to diagnostics)
- When the surface's chrome should stay interactive (the user can still navigate away)

## When NOT to use it

- For per-section errors — surface those inline with the section, not via a full-frame error
- For destructive-mutation errors — use a toast or inline banner instead
- For terminal / non-recoverable errors (account suspended, license expired) — use a dedicated guard surface

## States / variants

Single variant. Content is fully driven by props; no per-state CSS variants.

## Composition

Centered max-width 640px column.

```
┌────────────────────────────────────┐
│                                    │
│       BRIEFING UNAVAILABLE         │  ← mono 11px caps, terracotta (eyebrow)
│                                    │
│   We couldn't load your briefing.  │  ← serif 28px (headline / message)
│                                    │
│   A signal source isn't            │  ← serif italic 17px (detailMessage, optional)
│   responding. Your day is still    │
│   on the calendar — we just        │
│   can't shape it into a briefing   │
│   right now.                       │
│                                    │
│   [Try again]  [Diagnostics]       │  ← ui-button-default + ui-button-secondary
│                                    │
│   code: dependency_failed ·        │  ← mono 10px, tertiary (optional)
│   service: predictions             │
│                                    │
└────────────────────────────────────┘
```

The "Try again" button calls `onRetry()`; "Diagnostics" routes to `/settings#diagnostics` (or a service-specific diagnostics path if `service` is provided).

## Tokens consumed

- `--color-spice-terracotta` — eyebrow
- `--color-text-primary` — headline
- `--color-text-secondary` — detail message
- `--color-text-tertiary` — meta line
- `--color-border-strong` — primary button border
- `--font-mono` — eyebrow + meta
- `--font-serif` — headline + detail
- `--font-sans` — button labels
- `--space-lg`, `--space-xl` — vertical spacing

## API sketch

```tsx
<BriefingErrorState
  eyebrow="BRIEFING UNAVAILABLE"
  message="We couldn't load your briefing."
  detailMessage="A signal source isn't responding. Your day is still on the calendar — we just can't shape it into a briefing right now."
  code="dependency_failed"
  service="predictions"
  onRetry={() => refresh()}
  onDiagnostics={() => navigate("/settings#diagnostics")}
/>
```

Contract type:

```ts
interface BriefingErrorStateProps {
  eyebrow: string;             // surface-specific (e.g. "BRIEFING UNAVAILABLE")
  message: string;             // primary headline
  detailMessage?: string;      // optional secondary sentence
  code?: string;               // optional meta
  service?: string;            // optional meta
  onRetry?: () => void;
  onDiagnostics?: () => void;
}
```

The pattern does not auto-retry. Stack-trace exposure is forbidden — only `message` + `detailMessage` + the typed `code` / `service` meta render.

## Source

- **Code:** ships W5 (DOS-429) at `src/components/dashboard/BriefingErrorState.tsx` + `src/components/dashboard/BriefingErrorState.module.css`.
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign-error.html`

## Surfaces that consume it

- DailyBriefing (via `BriefingLoadState.status === "error"`)

## Naming notes

`BriefingErrorState` is the canonical name. The Briefing prefix matches `NAMING.md`'s ✅ example `BriefingSpine` — patterns unique to the briefing carry the prefix. There is no generic `ErrorState` to shadow. Existing canonical precedent: `BriefingMeetingCard`, `DailyBriefingAttentionSection`.

The slot-based API (`eyebrow`, `message`, `detailMessage`, `code`, `service`, `onRetry`, `onDiagnostics`) keeps copy out of the component and makes the pattern trivial to test. The briefing surface owns the words.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W5 under DOS-429.
