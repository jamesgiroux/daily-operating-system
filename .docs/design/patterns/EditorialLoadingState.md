# EditorialLoadingState

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `EditorialLoadingState`
**`data-ds-spec`:** `patterns/EditorialLoadingState.md`
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

Pulsing dot animation: `editorial-loading-pulse 1.4s ease-in-out infinite`. Pure CSS, no skeleton-content motion. The surface's chrome (FolioBar, FloatingNavIsland) remains rendered and interactive.

## Tokens consumed

- `--color-text-tertiary` — headline + eyebrow
- `--color-spice-turmeric` — pulsing dot
- `--font-serif` — headline
- `--font-mono` — eyebrow
- `--space-lg`, `--space-xl` — vertical spacing between elements

## API sketch

```tsx
<EditorialLoadingState
  headline="Reading your day…"
  eyebrow="GATHERING TODAY'S SIGNALS"
  withPulse
/>

<EditorialLoadingState
  headline="Loading account…"
  eyebrow="ASSEMBLING DOSSIER"
/>
```

Contract type:

```ts
interface EditorialLoadingStateProps {
  headline: string;        // surface-specific copy
  eyebrow: string;         // surface-specific copy
  withPulse?: boolean;     // default true
}
```

Copy is **always** passed in by the consuming surface — the pattern owns shape and motion, not editorial register words.

## Source

- **Code:** ships W5 (DOS-429) at `src/components/dashboard/EditorialLoadingState.tsx` + `src/components/dashboard/EditorialLoadingState.module.css` (initial). May lift to `src/components/shared/` once a second consumer adopts it.
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign-loading.html` (DailyBriefing instance with copy "Reading your day…" / "GATHERING TODAY'S SIGNALS")

## Surfaces that consume it

- DailyBriefing (via `BriefingLoadState.status === "loading"`)
- (future) AccountDetail, ProjectDetail when their loading states adopt the editorial register

## Naming notes

`EditorialLoadingState` is the canonical name. Earlier draft used `BriefingLoadingState`, which violated the `NAMING.md` rule "patterns are named for the pattern, not the surface." The pattern is briefing-resident today but is structurally generic — the briefing-specific copy lives in the consuming surface, not the pattern.

Distinct from a generic loading spinner. The `Editorial` prefix marks this as the editorial-register variant — calm, serif, no skeleton motion.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. Renamed from `BriefingLoadingState` per `NAMING.md` policy. TSX ships W5 under DOS-429.
