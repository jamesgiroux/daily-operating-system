# PhaseTimeline

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `PhaseTimeline`
**`data-ds-spec`:** `patterns/PhaseTimeline.md`
**Variants:** phase set `discover | build | beta | launch | warn`; milestone state `default | done | next`; bar state `past | future`; configurable `tickCount`
**Design system version introduced:** 0.6.0

## Job

Render a horizontal project timeline with overlaid phase bars, milestone diamonds, a "now" indicator, and a month-tick legend. Gives the user a single at-a-glance answer to "where is this project on its arc and what's coming next."

The timeline is project-shaped: phases (Discovery / Build / Beta / Launch / GA) span a multi-month window with milestones positioned along their length and a now-line showing today's position. It is not a Gantt chart — there are no individual workstream rows.

## When to use it

- On Project Detail's "Plan at a glance" chapter when the project has a defined phase plan and dated milestones
- On report surfaces (EBR/QBR, Monthly Wrapped) when a project arc needs to anchor an executive narrative
- For program-level surfaces where multiple project arcs share a common time axis (future use)

## When NOT to use it

- For day-scoped scheduling — use `DayChart`
- For per-meeting timelines — use `MeetingSpineItem` lists
- For a Gantt-style multi-row project plan — out of scope; use a dedicated planning surface
- For account-level lifecycle visualization — out of scope (consider a future `AccountLifecycle` pattern)

## States / variants

- **phase set** — phases are colored per `data-phase`: `discover` (paper-linen), `build` (olive), `beta` (turmeric), `launch` (rosemary), `warn` (terracotta)
- **bar state** — `past` (55% opacity), `future` (85% opacity), default current (100%)
- **milestone state** — `default` (outlined diamond), `done` (filled charcoal), `next` (filled terracotta — the next milestone, often slipping)
- **now-line** — vertical terracotta line with a configurable label (rendered via `data-label` attribute, default "NOW") and a small terminal dot
- **tick count** — `data-tick-count="6 | 8 | 12"` controls the month-tick grid columns to match the timeline window

## Composition

This pattern does not compose other primitives. It owns its bars, ticks, milestones, and now-line directly. Optionally pairs with a `FreshnessIndicator` or surface-level freshness line in the chapter heading above it.

## Tokens consumed

- `--color-paper-linen`, `--color-paper-cream` (discover phase, now-line label background)
- `--color-garden-olive`, `--color-garden-rosemary`, `--color-spice-turmeric`, `--color-spice-terracotta` (phase fills)
- `--color-rule-light`, `--color-rule-heavy` (track borders, discovery border)
- `--color-text-tertiary`, `--color-text-secondary` (labels, ticks)
- `--color-desk-charcoal` (milestone outlines + done fill)
- `--font-mono` (ticks, milestone labels, meta), `--font-sans` (bar title fallback), `--font-serif` (bar title)
- `--space-md`, `--space-lg`, `--space-2xl`
- `--shadow-md` (bar hover lift)
- `--transition-normal`

## API sketch

```html
<section class="phase-timeline" data-ds-name="PhaseTimeline" data-ds-spec="patterns/PhaseTimeline.md">
  <div class="phase-timeline-legend">
    <span><span class="phase-timeline-swatch" data-phase="discover"></span>Discovery</span>
    <span><span class="phase-timeline-swatch" data-phase="build"></span>Build</span>
    <span><span class="phase-timeline-swatch" data-phase="beta"></span>Beta</span>
    <span><span class="phase-timeline-swatch" data-phase="launch"></span>Launch / GA</span>
    <span><span class="phase-timeline-swatch" data-phase="warn"></span>At risk</span>
  </div>

  <div class="phase-timeline-ticks" data-tick-count="8">
    <span>Mar</span><span data-quarter-end="true">Q1 →</span>
    <span>Apr</span><span>May</span><span>Jun</span>
    <span data-quarter-end="true">Q2 →</span><span>Jul</span><span>Aug</span>
  </div>

  <div class="phase-timeline-track">
    <div class="phase-timeline-bar" data-phase="discover" data-state="past" style="left:0%; width:12.5%;">
      <span class="phase-timeline-bar-title">Discovery</span>
      <span class="phase-timeline-bar-meta">Mar 4 – Mar 18</span>
    </div>
    <!-- … more bars … -->

    <div class="phase-timeline-milestone" data-state="done" style="left: 6.25%;"></div>
    <div class="phase-timeline-milestone-label" style="left: 6.25%;">Kickoff · Mar 4</div>

    <div class="phase-timeline-now" data-label="NOW · WK 14" style="left: 44%;"></div>
  </div>
</section>
```

React form:

```tsx
<PhaseTimeline
  legend={[
    { phase: 'discover', label: 'Discovery' },
    { phase: 'build',    label: 'Build' },
    { phase: 'beta',     label: 'Beta' },
    { phase: 'launch',   label: 'Launch / GA' },
    { phase: 'warn',     label: 'At risk' },
  ]}
  ticks={['Mar', 'Q1 →', 'Apr', 'May', 'Jun', 'Q2 →', 'Jul', 'Aug']}
  phases={[
    { phase: 'discover', state: 'past',  start: 0,    width: 12.5, title: 'Discovery', meta: 'Mar 4 – Mar 18' },
    { phase: 'build',    state: 'past',  start: 12.5, width: 25,   title: 'Build',     meta: 'Mar 19 – Apr 22 · Closed' },
    { phase: 'beta',                       start: 37.5, width: 30, title: 'Beta · 2 partners', meta: 'Apr 23 – Jun 30' },
    { phase: 'launch',   state: 'future', start: 67.5, width: 20,  title: 'GA prep + launch',  meta: 'Jul 1 – Jul 22' },
  ]}
  milestones={[
    { state: 'done', position: 6.25, label: 'Kickoff · Mar 4' },
    { state: 'done', position: 12.5, label: 'Arch sign-off' },
    { state: 'done', position: 37.5, label: 'Beta start' },
    { state: 'next', position: 44,   label: 'Residency · slipping' },
    /* … */
  ]}
  now={{ position: 44, label: 'NOW · WK 14' }}
/>
```

## Source

- **Spec:** new for v1.4.2 project-detail d-spine
- **Reference CSS:** `.docs/design/reference/_shared/styles/PhaseTimeline.module.css`
- **Code:** to be shipped at `src/components/entity/PhaseTimeline.tsx`
- **Mockup origin:** `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.phase-strip`, `.phase-bar`, `.phase-milestone`, `.phase-now-line`)

## Surfaces that consume it

- ProjectDetail (canonical consumer, v1.4.2 — Plan at a glance chapter)
- Reports (EBR/QBR, Monthly Wrapped, Risk Briefing) when a project arc anchors the narrative

## Naming notes

`PhaseTimeline` — names the structural job (a timeline of project phases). Not `ProjectTimeline` because timelines may be valid on non-project surfaces (program rollups, account lifecycles); the constraint is "phases as the primary axis." Not `Roadmap` because roadmap implies forward-only intent and PhaseTimeline shows past + present + future symmetrically. The hardcoded "NOW · WK 14" label from the mockup is configurable via `data-label` so consumers can pass week numbers, ISO dates, or just "TODAY."

## History

- 2026-05-10 — Proposed for v1.4.2 project-detail d-spine. No existing pattern handled month-tick + phase bar + milestone + now-line composition; promoted from the D-composite mockup with the now-line label parameterized.
