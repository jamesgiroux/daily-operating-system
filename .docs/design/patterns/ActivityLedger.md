# ActivityLedger

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `ActivityLedger`
**`data-ds-spec`:** `patterns/ActivityLedger.md`
**Variants:** per-row `data-kind="meeting | decision | email | action | event | capture"` (dot color); optional `.activity-ledger-filters` chip row above the list
**Design system version introduced:** 0.6.0

## Job

Render the editorial weekly digest of "what's moved" on a project, account, or person — the synthesized subset of activity events that materially changed or confirmed the working understanding this week. Each row pairs a precise timestamp with a tone-coded type dot, a bold-prose body that names the event in editorial voice (with inline entity chips for the people / accounts / actions involved), and a meta line carrying attribution and entity references.

This is editorial synthesis, not chronological record-keeping. The producer curates which events are worth surfacing; the unit of work is the weekly digest, not the full activity log.

## When to use it

- On Project Detail's "What's moved this week" chapter
- On Account Detail's weekly digest section when a similar editorial summary is warranted
- On Person Detail's "Recent activity" section when the editorial weekly framing fits
- In reports (Risk Briefing, Account Health) when a curated activity digest anchors a narrative chapter

## When NOT to use it

- For the full chronological activity log — use `TimelineEntry` (used by project.html's "The Record" chapter and account.html equivalents)
- For the settings security/audit log — use `ActivityLogSection` (different domain: Security / Anomaly / Config / System categories, filter chips, day-grouping, integrity-verification footer)
- For the durable project decisions trail — use `DecisionLog` (decisions only, with reversal handling)
- For meeting-derived findings (wins/risks/decisions in the same block) — use `FindingsTriad`

## States / variants

- **type kind** — `data-kind="meeting | decision | email | action | event | capture"` controls the dot color via existing palette tokens (turmeric / rosemary / larkspur / terracotta / olive / sage). Default tertiary for unknown kinds
- **filter chips** — optional `.activity-ledger-filters` line ABOVE the list with inline links (Filter by type · person · account). Surface-controlled; the pattern doesn't manage filter state
- **bold prose** — body sentence uses `<strong>` for the primary action description; `<em>` for italicized reasoning
- **inline entity chips** — `EntityChip` primitives appear inline in the body and meta line for people / accounts / actions / projects

## Composition

Composes:
- `EntityChip` primitive — for inline entity references in body and meta
- (Optionally) `Pill` primitive — for filter chip variants if the surface promotes the filter row to a control

Self-contained otherwise (timestamp column, dot, body, meta are owned by the pattern).

## Tokens consumed

- `--color-spice-turmeric`, `--color-garden-rosemary`, `--color-garden-larkspur`, `--color-spice-terracotta`, `--color-garden-olive`, `--color-garden-sage` (dot kind colors)
- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`
- `--color-rule-light` (row dividers)
- `--font-mono` (timestamp, meta), `--font-serif` (body)
- `--space-xs | sm | md`

## API sketch

```html
<ol class="activity-ledger"
    data-ds-name="ActivityLedger"
    data-ds-spec="patterns/ActivityLedger.md">
  <li class="activity-ledger-row">
    <span class="activity-ledger-when">Today &middot; 9:14a</span>
    <span class="activity-ledger-dot" data-kind="action" aria-hidden="true"></span>
    <div class="activity-ledger-body">
      <p>
        <strong>Residency remediation draft</strong> sent to legal —
        owed to <a href="#" class="EntityChip_chip EntityChip_chip-person">Marco Devine</a> by Friday.
      </p>
      <div class="activity-ledger-meta">
        <span>S. Wu</span>
        <span class="activity-ledger-meta-sep">&middot;</span>
        <a href="#" class="EntityChip_chip EntityChip_chip-project">Q2 Launch Program</a>
        <a href="#" class="EntityChip_chip">Action #41</a>
      </div>
    </div>
  </li>
</ol>
```

React form:

```tsx
<ActivityLedger
  rows={[
    {
      when: "Today · 9:14a",
      kind: "action",
      body: <><strong>Residency remediation draft</strong> sent to legal — owed to <EntityChip kind="person" name="Marco Devine"/> by Friday.</>,
      meta: ["S. Wu", <EntityChip kind="project" name="Q2 Launch Program"/>, <EntityChip name="Action #41"/>],
    },
  ]}
/>
```

## Source

- **Spec:** new for v1.4.2 project-detail d-spine
- **Reference CSS:** `.docs/design/reference/_shared/styles/ActivityLedger.module.css`
- **Code:** to be shipped at `src/components/entity/ActivityLedger.tsx`
- **Mockup origin:** `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.vD-activity`)

## Surfaces that consume it

- ProjectDetail (canonical consumer, v1.4.2 — "What's moved this week" chapter)
- AccountDetail (potential consumer for weekly digest section)
- PersonDetail (potential consumer for recent activity section)
- Reports (Risk Briefing, Account Health) when a curated activity digest anchors a chapter

## Naming notes

`ActivityLedger` — "activity" (what happened) + "ledger" (curated record, weekly granularity). Not `WeeklyDigest` (too time-scoped — pattern works for any synthesized subset, not strictly weekly). Not `ActivityFeed` (feed implies firehose / chronological / unfiltered). Not `Timeline` (collides with `TimelineEntry` and `UnifiedTimeline` which are chronological log patterns). The "ledger" framing carries the editorial-synthesis intent.

The chrome-overlap audit on 2026-05-10 initially marked this as COMPOSE-via-`ActivityLogSection`, then COMPOSE-via-`TimelineEntry`. Both rejected on second pass: `ActivityLogSection` is settings-domain (Security/Anomaly categories), and `TimelineEntry` is chronological audit log. ActivityLedger is genuinely net new — same family, different intent.

## History

- 2026-05-10 — Proposed for v1.4.2 project-detail d-spine. Promoted from variation D's `.vD-activity` mockup CSS after two prior chrome-overlap audit cycles incorrectly mapped it to existing patterns. The editorial-prose body + multi-chip meta line + precise-timestamp left column distinguish it from `TimelineEntry` (date-only left column + plain title + plain detail).
