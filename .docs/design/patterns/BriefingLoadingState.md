# BriefingLoadingState

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `BriefingLoadingState`
**`data-ds-spec`:** `patterns/BriefingLoadingState.md`
**Variants:** `default` (with optional `withPulse` toggle)
**Design system version introduced:** 0.6.0

## Job

Render an editorial-register holding state while a surface's primary data assembles. A centered single-column stage with a serif headline and an optional pulsing-dot mark. The pattern is honest — "Reading your day…" rather than fake content shapes — and shared across surfaces that prefer editorial calm to skeleton screens.

## When to use it

- Any editorial-register surface (DailyBriefing, AccountDetail, ProjectDetail) while the primary view-model loads
- When the surface's chrome (FolioBar, FloatingNavIsland) should remain interactive but the body slot should hold
- When the data shape is too varied to skeleton honestly (predictions count varies, sections are conditional)

## When NOT to use it

- For a list / table that has a known row count — use a row-skeleton pattern instead (TBD)
- For a non-editorial register surface (Settings, Inbox) — use a default loading spinner
- When the load is expected to be <300ms — avoid flashing the state

## States / variants

Single variant; the pulsing dot is a property toggle, not a separate variant:

- **Default** — eyebrow + serif headline + optional pulsing dot.
- **`withPulse={false}`** — pulsing dot suppressed (used when the parent prefers stillness).

## Composition

Pattern — no sub-primitives. Centered max-width 640px column.

```
┌────────────────────────────────────┐
│                                    │
│      Reading your day…             │  ← serif italic 32px, tertiary (headline)
│                                    │
│              •                     │  ← 8px pulsing dot, turmeric (optional)
│                                    │
│      GATHERING TODAY'S SIGNALS     │  ← mono 11px caps, tertiary (eyebrow)
│                                    │
└────────────────────────────────────┘
```

Pulsing dot animation: `briefing-loading-pulse 1.4s ease-in-out infinite`. Pure CSS, no skeleton-content motion. The surface's chrome (FolioBar, FloatingNavIsland) remains rendered and interactive.

## Tokens consumed

- `--color-text-tertiary` — headline + eyebrow
- `--color-spice-turmeric` — pulsing dot
- `--font-serif` — headline
- `--font-mono` — eyebrow
- `--space-lg`, `--space-xl` — vertical spacing between elements

## API sketch

```tsx
<BriefingLoadingState
  headline="Reading your day…"
  eyebrow="GATHERING TODAY'S SIGNALS"
  withPulse
/>

<BriefingLoadingState
  headline="Loading account…"
  eyebrow="ASSEMBLING DOSSIER"
/>
```

Contract type:

```ts
interface BriefingLoadingStateProps {
  headline: string;        // surface-specific copy
  eyebrow: string;         // surface-specific copy
  withPulse?: boolean;     // default true
}
```

Copy is **always** passed in by the consuming surface — the pattern owns shape and motion, not editorial register words.

## Source

- **Code:** ships W5 (DOS-429) at `src/components/dashboard/BriefingLoadingState.tsx` + `src/components/dashboard/BriefingLoadingState.module.css`.
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign-loading.html` (DailyBriefing instance with copy "Reading your day…" / "GATHERING TODAY'S SIGNALS")

## Surfaces that consume it

- DailyBriefing (via `BriefingLoadState.status === "loading"`)

## Naming notes

`BriefingLoadingState` is the canonical name. The Briefing prefix matches `NAMING.md`'s ✅ example `BriefingSpine` — patterns unique to the briefing surface carry the prefix. The anti-example (`❌ BriefingTrustBand`) only applies when the unprefixed pattern (`TrustBand`) already exists generically; there is no generic `LoadingState` to shadow. Existing canonical precedent: `BriefingMeetingCard`, `DailyBriefingAttentionSection`.

The slot-based API (`headline`, `eyebrow`, `withPulse`) keeps the editorial copy out of the component file and makes the pattern trivial to test in isolation. The briefing surface owns the words.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W5 under DOS-429.
