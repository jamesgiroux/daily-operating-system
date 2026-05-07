# BriefingEmptyState

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `BriefingEmptyState`
**`data-ds-spec`:** `patterns/BriefingEmptyState.md`
**Variants:** `default`
**Design system version introduced:** 0.6.0

## Job

Render an editorial-register cold-start frame when a surface has no data because the user has not yet connected enough sources. Left-aligned 640px column with eyebrow, serif headline, italic lede, optional connect-checklist, and a primary CTA. The pattern frames "what DailyOS needs" rather than "the surface failed" — these are opt-in connection prompts, not errors.

## When to use it

- Any editorial-register surface when the primary view-model is empty *because the user hasn't connected required sources*
- When there's a clear primary action (connect Google, connect Gong) that resolves the empty state
- When the surface should remain navigable (chrome interactive) so the user can explore other parts of the product

## When NOT to use it

- For an empty list / table where the user has connected sources but there's no data this week — use a small inline empty hint instead
- For a feature the user lacks permission for — use a permission-guard pattern
- For a transient empty (still loading) — use `BriefingLoadingState`
- For an authentication failure mid-session — use `BriefingErrorState` with `code="dependency_failed"`

## States / variants

Single variant. Content is fully driven by props; the checklist and CTA are optional.

## Composition

Left-aligned 640px column.

```
┌────────────────────────────────────────┐
│ DAILY BRIEFING                         │  ← mono 11px caps, tertiary (eyebrow)
│                                        │
│ Your day, when DailyOS                 │  ← serif 36px (headline)
│ can read it.                           │
│                                        │
│ The briefing is a synthesis of...      │  ← serif italic 19px (lede)
│                                        │
│ ○  Connect Google to bring in...       │  ← checklist (optional)
│ ○  Optional: Glean for cross-tool...   │
│ ○  Optional: Claude Code to enable...  │
│                                        │
│ [Connect Google]                       │  ← ui-button-lg (CTA, optional)
│                                        │
└────────────────────────────────────────┘
```

Checklist items use `○` / `●` glyphs based on `status` (`"todo"` / `"done"`). The CTA only renders if `cta` is passed.

## Tokens consumed

- `--color-text-tertiary` — eyebrow + checklist glyph
- `--color-text-primary` — headline
- `--color-text-secondary` — lede
- `--color-spice-saffron` — primary CTA accent
- `--font-mono` — eyebrow
- `--font-serif` — headline + lede
- `--font-sans` — checklist + CTA label
- `--space-lg`, `--space-xl` — vertical spacing

## API sketch

```tsx
<BriefingEmptyState
  eyebrow="DAILY BRIEFING"
  headline="Your day, when DailyOS can read it."
  lede="The briefing is a synthesis of your calendar, mail, and signal sources. Connect what you'd like; we'll start reading."
  checklistItems={[
    { label: "Connect Google to bring in calendar and mail", status: "todo" },
    { label: "Optional: Glean for cross-tool retrieval", status: "todo" },
    { label: "Optional: Claude Code to enable abilities", status: "todo" },
  ]}
  cta={{ label: "Connect Google", onClick: () => connectGoogleAuth() }}
/>
```

Contract type:

```ts
interface BriefingEmptyStateProps {
  eyebrow: string;
  headline: string;
  lede: string;
  checklistItems?: { label: string; status?: "todo" | "done" }[];
  cta?: { label: string; onClick: () => void };
}
```

The pattern does not detect or trigger auth — `cta.onClick` delegates to the consuming surface's connection hook (`connectGoogleAuth()` for the briefing case). It does not render the FolioBar's readiness pairs in cold-start; the folio is bare.

## Source

- **Code:** ships W5 (DOS-429) at `src/components/dashboard/BriefingEmptyState.tsx` + `src/components/dashboard/BriefingEmptyState.module.css`.
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign-empty.html`

## Surfaces that consume it

- DailyBriefing (via `BriefingLoadState.status === "empty"`)

## Naming notes

`BriefingEmptyState` is the canonical name. The Briefing prefix matches `NAMING.md`'s ✅ example `BriefingSpine` — patterns unique to the briefing carry the prefix. There is no generic `EmptyState` to shadow. Existing canonical precedent: `BriefingMeetingCard`, `DailyBriefingAttentionSection`.

The slot-based API (`eyebrow`, `headline`, `lede`, `checklistItems`, `cta`) keeps copy out of the component and makes the pattern trivial to test. The briefing surface owns the words. Distinct from a generic null-state placeholder — this pattern carries the implicit contract that the state is *opt-in cold-start* (user hasn't connected sources), not a technical failure.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W5 under DOS-429.
