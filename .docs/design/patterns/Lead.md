# Lead

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `Lead`
**`data-ds-spec`:** `patterns/Lead.md`
**Variants:** editorial register in source copy (`calm` | `sharp` | `quiet`); optional `headline.punchLine` emphasis
**Design system version introduced:** 0.1.0

## Job

The single-sentence headline that opens DailyBriefing — large serif type that sums up "what matters today" in one breath. The lead is always present and always one sentence; if you can't say it in one sentence, the briefing is the wrong shape.

## When to use it

- Standalone component ships at `src/components/dashboard/Lead.tsx`. Shipped DailyBriefing currently uses local `heroHeadline`, `heroNarrative`, `focusBlock`, and `focusCapacity` classes instead.
- Roadmap target: top of the pure `DailyBriefingRedesign.tsx` source component.
- Top of any future surface where a single-sentence summary should establish the day / state / theme
- Pair with an eyebrow ("Today, Thursday April 23") above

## When NOT to use it

- Multi-sentence summaries — those are paragraphs (use serif body or a `Lede` paragraph below the hero, not Lead)
- CTAs, headlines with action verbs, banner copy — Lead is editorial voice, not marketing voice

## Composition

```
[Lead sentence — serif 52px (default), weight 400, line-height 1.1, max-width 980px]
   Lead phrase + optional `headline.punchLine` highlight (subtle turmeric-15 underline)
[Focus capacity — mono 12px, color text-tertiary]
[Optional focus block — serif 16px, color text-secondary]
```

## Variants

- **calm** — observational summary ("Four meetings today, two with customers. Light afternoon after 2:00.")
- **sharp** — calls out the one thing that matters ("The Acme renewal at 10:00 is the one to nail — legal needs the final terms before the MSA review.")
- **quiet** — light-day register ("A quiet day — one customer call, two internal syncs. Room to think.")

The optional `headline.punchLine` highlight (subtle turmeric underline gradient) marks the most-important clause and survives across all registers.

## Tokens consumed

- `--font-serif` (sentence, focus block), `--font-mono` (focus capacity)
- `--color-text-primary` (sentence), `--color-text-secondary` (focus block), `--color-text-tertiary` (focus capacity), `--color-spice-turmeric-15` (highlight)
- `--space-xs`, `--space-md`, `--space-3xl` (vertical rhythm)

## API sketch

```tsx
<Lead
  lead={{
    headline: {
      lead: "Four meetings today, two with customers —",
      punchLine: "the Acme renewal at 10:00 is the one to nail.",
    },
    focusCapacity: "4h 30m available · 2 deep work blocks · 4 meetings",
    focusBlock: "Legal needs the final terms before the MSA review.",
  }}
/>
```

Contract type (from `BriefingViewModel`):

```ts
interface LeadViewModel {
  headline: { lead: string; punchLine?: string };
  focusCapacity: string;
  focusBlock?: string;
}
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/Daily Briefing redesign.html` (`.lead`, `.lead-sentence`, `.lead-eyebrow`)
- **Code:** ships W1 (DOS-426) at `src/components/dashboard/Lead.tsx` + `src/components/dashboard/Lead.module.css`
- **Roadmap target:** top of the pure `DailyBriefingRedesign.tsx` source component

## Surfaces that consume it

No shipped routed consumers. DailyBriefing has a local hero treatment today. Potential extension: the "lead summary" spot at the top of high-information surfaces if this pattern is promoted from roadmap.

## Naming notes

`Lead` — newspaper-editorial term for the opening sentence. Consider whether to rename to `BriefingLead` if the pattern becomes briefing-specific in practice; current spec assumes it's reusable.

## History

- 2026-05-06 — Integrated W1 `Lead` component against `LeadViewModel`.
- 2026-05-06 — Removed the pricing-memo blocker sentence from the sharp variant.
- 2026-05-02 — Proposed pattern for v1.4.3 from Daily Briefing redesign mockup.
