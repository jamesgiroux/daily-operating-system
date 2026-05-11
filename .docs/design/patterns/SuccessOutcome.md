# SuccessOutcome

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `SuccessOutcome`
**`data-ds-spec`:** `patterns/SuccessOutcome.md`
**Variants:** entity tint via `data-entity="project | person | account | self | none"`; default tone is success
**Design system version introduced:** 0.6.0

## Job

Render the outcome statement that defines what success means for a project, account, person, or work program. A first-class, sticky callout that frames the user's commitment in editorial language: "this is what we're building toward and how we'll know it's done."

The outcome statement is not a status update, not a prediction, and not a list of milestones. It's the durable, revisited north star — usually defined at kickoff and revised when the underlying commitment changes.

## When to use it

- On Project Detail when the project has a defined goal/outcome captured at kickoff or in a steering doc
- On Account Detail when the account has an explicit success commitment (e.g. expansion target, retention plan)
- On Person Detail when there's a relationship goal worth surfacing (rarer; usually relationship arc lives in narrative chapters)
- In reports or executive briefings where the success contract needs to be visible before metrics

## When NOT to use it

- For ongoing status — use `MeterCluster` or chapter narrative instead
- For an upcoming milestone — use `PhaseTimeline` or a milestone callout
- For a finding/decision/risk — use `FindingsTriad` (meeting findings) or `DecisionLog` (project decisions)
- For trust receipts on a claim — use `ReceiptCallout`
- For warning that something is at risk — use `StaleReportBanner` or `ConsistencyFindingBanner`

## States / variants

- **default** — `tone="success"` rendering of the outcome statement with mono kicker, large serif body, mono signoff
- **entity-tinted** — when rendered on an entity surface (project/person/account/self), the underlying tone shifts from generic success-sage to the entity's color via `data-entity`. Example: `data-entity="project"` resolves to olive instead of sage on Project Detail. This keeps the editorial frame consistent with the entity's identity color.
- **revised** — when the outcome was revised after kickoff, the signoff includes the most recent revision attribution

## Composition

Composes the `Callout` primitive with these slot conventions:

- `tone="success"` (default) or entity-tinted when on an entity surface
- `border="full"`
- `density="expanded"`
- `shape="rounded"`
- `Callout.Label` carries the timeframe (e.g. "By GA · Jul 15 — and 90 days after")
- `Callout.Body` carries the outcome statement in serif typography (overrides Callout's default sans body via the SuccessOutcome CSS)
- `Callout.Footer` carries the signoff ("Defined Mar 4 at kickoff · Last revised Apr 8 by J. Park")

## Tokens consumed

Inherits Callout primitive tokens. Plus:

- `--font-serif` (body override — outcome statement reads as editorial prose, not sans UI text)
- `--color-garden-olive-12` (project entity tint)
- `--color-garden-larkspur-12` (person entity tint)
- `--color-spice-turmeric-12` (account entity tint)
- `--color-garden-eucalyptus-10` (self entity tint)

## API sketch

DOM / HTML form:

```html
<aside class="callout success-outcome"
       data-ds-name="SuccessOutcome"
       data-ds-spec="patterns/SuccessOutcome.md"
       data-tone="success"
       data-entity="project"
       data-border="full"
       data-density="expanded"
       data-shape="rounded">
  <div class="callout-label">By GA · Jul 15 — and 90 days after</div>
  <div class="callout-body">
    <p>Two design-partner tenants live and billable by July 15. Three pipeline
    logos shipped within 90 days. A partner-co-authored case study and
    featured AppExchange listing — so the integration sells itself.</p>
  </div>
  <div class="callout-footer">Defined Mar 4 at kickoff · Last revised Apr 8 by J. Park</div>
</aside>
```

React form:

```tsx
<SuccessOutcome
  entity="project"
  timeframe="By GA · Jul 15 — and 90 days after"
  defined="Defined Mar 4 at kickoff"
  revised="Last revised Apr 8 by J. Park"
>
  Two design-partner tenants live and billable by July 15. Three pipeline
  logos shipped within 90 days. A partner-co-authored case study and
  featured AppExchange listing — so the integration sells itself.
</SuccessOutcome>
```

## Source

- **Spec:** new for v1.4.2 project-detail d-spine
- **Reference CSS:** `.docs/design/reference/_shared/styles/SuccessOutcome.module.css` (overrides on top of `Callout.module.css`)
- **Code:** to be shipped at `src/components/entity/SuccessOutcome.tsx`
- **Mockup origin:** `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.vD-outcome` rules)

## Surfaces that consume it

- ProjectDetail (canonical consumer, v1.4.2)
- AccountDetail (when an explicit success commitment is captured)
- PersonDetail (rare; relationship arc usually lives in narrative chapters)
- Reports (Risk Briefing, EBR/QBR, Account Health) when the outcome contract needs to lead

## Naming notes

`SuccessOutcome` — "success" (the editorial frame: this is what success looks like) + "outcome" (the durable target). Not `OutcomeCallout` because the "Callout" suffix would collide with `ReceiptCallout`'s drill-in semantics. Not `SuccessBlock` because `StateBlock` already exists for left-bordered state lists with different shape.

## History

- 2026-05-10 — Proposed for v1.4.2 project-detail d-spine. Composes the new `Callout` primitive (introduced same release). Replaces the surface-local `.vD-outcome` styling from the D-composite mockup.
