# Lead

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `Lead`
**`data-ds-spec`:** `patterns/Lead.md`
**Variants:** `register="calm" | "sharp" | "quiet"`; optional inline `.sharp` highlight marker
**Design system version introduced:** 0.1.0

## Job

The single-sentence headline that opens DailyBriefing — large serif type that sums up "what matters today" in one breath. The lead is always present and always one sentence; if you can't say it in one sentence, the briefing is the wrong shape.

## When to use it

- Top of DailyBriefing (always)
- Top of any future surface where a single-sentence summary should establish the day / state / theme
- Pair with an eyebrow ("Today, Thursday April 23") above

## When NOT to use it

- Multi-sentence summaries — those are paragraphs (use serif body or a `Lede` paragraph below the hero, not Lead)
- CTAs, headlines with action verbs, banner copy — Lead is editorial voice, not marketing voice

## Composition

```
[Eyebrow — mono uppercase 10px, color text-tertiary]
[Lead sentence — serif 52px (default), weight 400, line-height 1.1, max-width 980px]
   Optional inline `.sharp` highlighting a key clause (subtle turmeric-15 underline)
```

## Variants

- **calm** — observational summary ("Four meetings today, two with customers. Light afternoon after 2:00.")
- **sharp** — calls out the one thing that matters ("The Acme renewal at 10:00 is the one to nail — the pricing memo still hasn't gone.")
- **quiet** — light-day register ("A quiet day — one customer call, two internal syncs. Room to think.")

The `.sharp` inline highlight (subtle turmeric underline gradient) marks the most-important clause and survives across all registers.

## Tokens consumed

- `--font-mono` (eyebrow), `--font-serif` (sentence)
- `--color-text-tertiary` (eyebrow), `--color-text-primary` (sentence), `--color-spice-turmeric-15` (highlight)
- `--space-md`, `--space-3xl` (vertical rhythm)

## API sketch

```tsx
<Lead
  eyebrow="Today, Thursday April 23"
  sentence="Four meetings today, two with customers — the Acme renewal at 10:00 is the one to nail; the pricing memo still hasn't gone."
  sharpClause="the Acme renewal at 10:00 is the one to nail"
  register="sharp"
/>
```

## Source

- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` (`.lead`, `.lead-sentence`, `.lead-eyebrow`)
- **Code:** to be implemented in `src/components/dashboard/Lead.tsx` (Wave 1 follow-on for v1.4.3)

## Surfaces that consume it

DailyBriefing (canonical). Potential extension: the "lead summary" spot at the top of high-information surfaces (e.g., AccountDetail's hero lede could compose this with a different register).

## Naming notes

`Lead` — newspaper-editorial term for the opening sentence. Consider whether to rename to `BriefingLead` if the pattern becomes briefing-specific in practice; current spec assumes it's reusable.

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from D-spine mockup.
