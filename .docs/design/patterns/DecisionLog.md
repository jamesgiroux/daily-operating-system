# DecisionLog

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `DecisionLog`
**`data-ds-spec`:** `patterns/DecisionLog.md`
**Variants:** per-row `data-state="default | reversed"`
**Design system version introduced:** 0.6.0

## Job

Render an ordered list of project-level decisions on record: the durable "who decided what, when, and why" trail that survives revisions. Each row is a dated decision with serif body text (em-emphasized reasoning) and a mono source attribution (decider + venue).

Distinct from related row patterns:
- `CommitmentRow` — confirmed meeting commitments (YOURS / THEIRS ownership). Spec note line 26 ("use FindingsTriad for decisions") is misleading; it should disambiguate to: meeting findings → FindingsTriad, project decisions → DecisionLog. (Maintenance ticket filed.)
- `FindingsTriad` — meeting-derived findings in 3 fixed buckets (wins / risks / decisions). The "decisions" bucket there is for findings *about* decisions captured in a meeting, not the project-level record.
- `ActivityLogSection` — chronological audit log of system operations (settings audit). DecisionLog is editorial/narrative.

## When to use it

- On Project Detail's "Decisions on record" chapter
- On Account Detail when an account has a load-bearing decision history (renewal terms, expansion gates)
- In report surfaces (EBR/QBR, Risk Briefing) where the decision trail is the spine of the executive narrative
- When a reversed decision needs to stay visible alongside its replacement so the reasoning is preserved

## When NOT to use it

- For confirmed meeting commitments — use `CommitmentRow`
- For meeting-derived findings — use `FindingsTriad`
- For chronological system events / settings audit — use `ActivityLogSection`
- For a single decision in callout form — use `Callout` with `tone="info"` and inline body

## States / variants

- **default** — current decision on record, full opacity, dated and sourced
- **reversed** — superseded by a later decision but kept on record. Body text shifts to text-secondary; the date prefixes a `↺` glyph; the entry remains in place chronologically so the reader can see what was decided and then reversed

## Composition

This pattern does not compose other primitives. Each row is plain markup: timestamp + serif body (with `<em>` for reasoning emphasis) + source attribution. Sits inside a `ChapterHeading` + `FreshnessLine` chapter shell.

## Tokens consumed

- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`
- `--color-rule-light` (row dividers)
- `--font-mono` (date, source), `--font-serif` (decision text)
- `--space-md`

## API sketch

```html
<ol class="decision-log" data-ds-name="DecisionLog" data-ds-spec="patterns/DecisionLog.md">
  <li class="decision-row">
    <span class="decision-when">Apr 19</span>
    <p class="decision-text">
      Ship the second partner's pilot in EU-West, not US-East.
      <em>Forced by their residency requirement; adds 2 weeks to provisioning but unblocks contract.</em>
    </p>
    <span class="decision-source">D. Mitchell · Slack</span>
  </li>
  <li class="decision-row" data-state="reversed">
    <span class="decision-when">Mar 18</span>
    <p class="decision-text">
      Ship our own auth flow.
      <em>Reversed Apr 2 — kept here so the why is preserved.</em>
    </p>
    <span class="decision-source">Architecture review</span>
  </li>
</ol>
```

React form:

```tsx
<DecisionLog
  decisions={[
    { when: 'Apr 19', text: <>Ship the second partner's pilot in EU-West, not US-East. <em>Forced by their residency…</em></>, source: 'D. Mitchell · Slack' },
    { when: 'Mar 18', state: 'reversed', text: <>Ship our own auth flow. <em>Reversed Apr 2…</em></>, source: 'Architecture review' },
  ]}
/>
```

## Source

- **Spec:** new for v1.4.2 project-detail d-spine
- **Reference CSS:** `.docs/design/reference/_shared/styles/DecisionLog.module.css`
- **Code:** to be shipped at `src/components/entity/DecisionLog.tsx`
- **Mockup origin:** `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.decision-list`, `.decision-row`, `.decision-when`, `.decision-text`, `.decision-source`)

## Surfaces that consume it

- ProjectDetail (canonical consumer, v1.4.2)
- AccountDetail (when decision history is load-bearing)
- Reports (EBR/QBR, Risk Briefing) where decisions anchor the narrative

## Naming notes

`DecisionLog` — "decision" (the unit) + "log" (the durable trail). Not `DecisionList` because list implies ephemeral; log implies kept. Not `DecisionRow` (singular row pattern) because the contract is the ordered collection. Reverse-decision marker uses `↺` glyph (no extra primitive); the reasoning text is inline em-emphasis to preserve the editorial tone.

## History

- 2026-05-10 — Proposed for v1.4.2 project-detail d-spine. Initial chrome-overlap audit attempted to map this to `CommitmentRow` (rejected: that pattern's spec excludes decisions) and to `FindingsTriad` (rejected: that pattern's "decisions" bucket is for meeting-derived findings, not project records). DecisionLog is genuinely new. Maintenance ticket pending to disambiguate `CommitmentRow.md` line 26.
