# Callout

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `Callout`
**`data-ds-spec`:** `primitives/Callout.md`
**Variants:** `tone="success" | "caution" | "warning" | "info" | "neutral" | "trust-band-current" | "trust-band-caution" | "trust-band-verification"`; `border="full" | "left-accent" | "gradient-bg" | "none"`; `density="compact" | "default" | "expanded"`; `shape="rounded" | "angular" | "pill"`; `dismissible`
**Design system version introduced:** 0.6.0

## Job

The visual primitive for any tinted, bounded surface that draws attention to a unit of content. Frames an optional label, a required body, and optional footer/actions inside a tone-driven container with consistent border, density, and shape vocabulary.

Named patterns that carry semantic meaning (`ReceiptCallout`, `SuccessOutcome`, `StaleReportBanner`, `EscalationQuote`, `ConsistencyFindingBanner`, `DataGapNotice`, `AboutThisIntelligencePanel`, `DossierSourceCoveragePanel`) compose `Callout` underneath. They add domain semantics, content shape, and behavior; `Callout` provides the visual chrome.

## When to use it

- When a piece of content needs to be visually distinct from surrounding flow (a tinted box, a left-accent strip, a gradient lift)
- As the underlying chrome for any pattern in the callout family (banners, panels, receipts, notices, outcomes, escalations)
- When you need consistent tone, border, density, and shape vocabulary across surfaces

## When NOT to use it

- For inline status badges — use `Pill`
- For full-bleed page sections without a frame — use chapter primitives (`ChapterHeading` + body)
- For interactive cards that navigate or open detail — use `EntityRow`, `MeetingCard`, or a surface-specific row pattern
- For meeting-derived findings (wins/risks/decisions in a single block) — use `FindingsTriad`
- When the meaning is entity identity, freshness, quality, or trust — use the corresponding named primitive that *composes* Callout where the visual chrome is needed (`TrustBand`, `FreshnessIndicator`, `EntityChip`)

## States / variants

### Tone (semantic intent)

| Tone | Meaning | Token resolution |
|---|---|---|
| `success` | Goal achieved, healthy, ready, outcome statement | `--color-garden-sage-12` bg, `--color-garden-rosemary` accent |
| `caution` | Pay attention, soft warning, stale | `--color-spice-saffron-12` bg, darkened saffron accent |
| `warning` | Hard warning, error, blocker, overdue | `--color-spice-terracotta-12` bg, `--color-spice-chili` accent |
| `info` | Informational, contextual, calm | `--color-garden-larkspur-12` bg, darkened larkspur accent |
| `neutral` | Generic frame, no semantic charge | `--color-desk-charcoal-4` bg, `--color-text-secondary` accent |
| `trust-band-current` | Trust receipt — likely current | alias of `success` (`--color-trust-likely-current-12`) |
| `trust-band-caution` | Trust receipt — use with caution | alias of `caution` (`--color-trust-use-with-caution-12`) |
| `trust-band-verification` | Trust receipt — needs verification | alias of `warning` (`--color-trust-needs-verification-12`) |

Trust-band aliases exist so trust-receipt patterns can express their intent semantically without coupling to the success/caution/warning vocabulary. Visually identical to their alias targets.

### Border

| Value | Treatment |
|---|---|
| `full` | 1px border on all sides, color matches tone accent |
| `left-accent` | 3px left border only, no other borders, used for inline expansion / drill-in receipts |
| `gradient-bg` | No border; a tone-tinted vertical gradient background instead (used for editorial pull-quote-style emphasis) |
| `none` | No border, only background tint (used for full-width banners) |

Default: `full`.

### Density

| Value | Padding | Use |
|---|---|---|
| `compact` | `--space-sm` vertical, `--space-md` horizontal | Inline notices, pill-shaped status |
| `default` | `--space-lg` all sides | Standard tinted boxes |
| `expanded` | `--space-xl` all sides | Receipts, outcomes, panels with multiple internal sections |

Default: `default`.

### Shape

| Value | Border-radius |
|---|---|
| `rounded` | `--radius-sm` (4px) |
| `angular` | 0 |
| `pill` | 999px (only valid with `density="compact"`; the body must fit on one line) |

Default: `rounded`.

### Dismissible

When set, renders a small dismiss affordance in the top-right of the callout. Consumers handle the click to remove or hide. Off by default.

## Tokens consumed

- Tone backgrounds: `--color-garden-sage-12`, `--color-spice-saffron-12`, `--color-spice-terracotta-12`, `--color-garden-larkspur-12`, `--color-desk-charcoal-4`
- Trust band aliases: `--color-trust-likely-current-12`, `--color-trust-use-with-caution-12`, `--color-trust-needs-verification-12`
- Tone accents (border + label): `--color-garden-rosemary`, `--color-spice-chili`, darkened saffron/larkspur (resolved via existing palette)
- Typography: `--font-mono` (label, footer), `--font-sans` (body), `--font-serif` (body when used inside editorial patterns like SuccessOutcome)
- Spacing: `--space-sm`, `--space-md`, `--space-lg`, `--space-xl`
- Radius: `--radius-sm`
- Rule: `--color-rule-light` (internal section dividers when callout has multiple slots)

## Composition slots

A Callout has four optional content regions:

```
.callout
├── .callout-label    — mono kicker, uppercase 10px (optional)
├── .callout-body     — primary content; sans by default, serif-eligible (required)
├── .callout-footer   — secondary attribution / signoff / metadata (optional)
└── .callout-actions  — button group, right-aligned (optional)
```

The body is the only required slot. Patterns that compose Callout decide which other slots to use and what content to put in them.

## API sketch

DOM / HTML form (canonical):

```html
<aside class="callout"
       data-ds-name="Callout"
       data-ds-spec="primitives/Callout.md"
       data-tone="success"
       data-border="full"
       data-density="expanded"
       data-shape="rounded">
  <div class="callout-label">By GA · Jul 15</div>
  <div class="callout-body">
    Two design-partner tenants live and billable by July 15.
  </div>
  <div class="callout-footer">Defined Mar 4 at kickoff · Last revised Apr 8 by J. Park</div>
</aside>
```

React form:

```tsx
<Callout tone="success" border="full" density="expanded" shape="rounded">
  <Callout.Label>By GA · Jul 15</Callout.Label>
  <Callout.Body>Two design-partner tenants live and billable by July 15.</Callout.Body>
  <Callout.Footer>Defined Mar 4 at kickoff · Last revised Apr 8 by J. Park</Callout.Footer>
</Callout>
```

## Source

- **Spec:** new for v1.4.x design system 0.6.0
- **Reference CSS:** `.docs/design/reference/_shared/styles/Callout.module.css` (canonical implementation)
- **Reference shared CSS:** `.docs/design/reference/_shared/primitives.css` (will absorb the `.callout` rules in the next consolidation pass)
- **Code:** to be shipped at `src/components/ui/Callout.tsx`
- **Replaces / consolidates:** the ad-hoc tinted-box treatments cataloged in `.docs/design/_audits/callout-usage-survey.md`

## Surfaces that consume it

Callout is composed by patterns, not consumed directly by surfaces. The patterns that compose Callout (today or after migration) are:

- `ReceiptCallout` — trust receipt drill-in
- `SuccessOutcome` — outcome statement (primary v1.4.2 consumer for the new primitive)
- `StaleReportBanner`, `ConsistencyFindingBanner`, `EscalationQuote`, `TemplateSuggestionBanner`, `DataGapNotice`, `StateBlock`, `AboutThisIntelligencePanel`, `DossierSourceCoveragePanel` — existing patterns that should compose Callout in a future consolidation pass; no migration required for v1.4.2.

## Naming notes

`Callout` is the umbrella primitive name because the existing pattern family (ReceiptCallout, ConsistencyFindingBanner, etc.) already uses callout/banner/notice vocabulary interchangeably. Resist renaming to `TintedBox`, `Frame`, or `Panel` — those add a new vocabulary without retiring the existing one. `Banner` and `Notice` remain valid pattern-level names for full-width and inline variants respectively; both compose Callout underneath.

## History

- 2026-05-10 — Proposed primitive informed by `_audits/callout-usage-survey.md` (12–15 callout-shaped surfaces in the codebase). Establishes a single substrate for tinted-box chrome with semantic tone, border, density, and shape vocabulary. Initial consumer is `SuccessOutcome` for the v1.4.2 project-detail d-spine; existing callout-family patterns continue with their current implementations and migrate in a separate maintenance pass.
