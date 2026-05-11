# Project D-Spine Chrome Overlap Audit

**Stress test:** 6 proposed patterns from `/Downloads/DailyOS Design System (1)/mockups/surfaces/project-detail/variations/D-composite.html` against the existing DailyOS DS inventory to identify composition vs creation opportunities.

**Principle:** A previous pass identified 6 "new" patterns, but many CSS rules add visual chrome that overlaps with existing primitives already doing the same job. Default to COMPOSE.

---

## Audit Findings

**1. MeterCluster** — verdict: COMPOSE
- **Existing match:** `VitalsStrip` (patterns/VitalsStrip.md) + `SignalGrid` (patterns/SignalGrid.md)
- **Composition strategy:** Each meter is a `VitalDisplay` within a horizontal vitals strip. Label + large serif value + monospace tone-tinted bar + trend arrow = VitalsStrip rhythm with bar-fill visual from the `meter-bar` CSS. The 4-meter grid layout mirrors SignalGrid's 2x2 density but at larger scale.
- **Chrome to drop:** The CSS `.meter-trend` arrow (`↑`/`↓`) is redundant; text ("from steady" / "since Mon") suffices. Color override for trend direction is already handled by VitalsStrip highlight tokens.
- **Confidence:** high — VitalsStrip + bar primitives fully cover this without needing a distinct pattern.

**2. PhaseTimeline** — verdict: CREATE
- **Why no composition works:** This is a specialized horizontal project timeline with overlaid milestone markers, phase-bar coloring, a "now" line, and month-tick legend. No existing pattern handles the time-axis layout or phase-to-milestone spatial relationship. `DayChart` (proposed, D-spine only) is close but targets day-scale; this is month/week scale and tied to project phase semantics, not briefing shape.
- **Minimal API:** `PhaseTimeline` accepts `phases: {start%, width%, color, label}[]`, `milestones: {position%, status, label}[]`, `nowLine: position%`, `legend: {phase, color}[]`. Renders as a positioned container with phase bars, overlaid milestone diamonds, and a now-line connector.
- **Chrome that should drop:** The `.phase-now-line::before` hardcoded `'NOW · WK 14'` label — make it a configurable prop or default to "NOW" if week number is unavailable. The month-tick grid (`grid-template-columns: repeat(8, 1fr)`) is project-specific; generalize to a `tickCount` prop.
- **Confidence:** high — genuine new pattern; no existing primitive or pattern covers the project timeline job.

**3. FindingsGrid** — verdict: COMPOSE
- **Existing match:** `FindingsTriad` (patterns/FindingsTriad.md) — already the canonical pattern for exactly this job.
- **Composition strategy:** The D-spine mockup renders "What's working / What's not / What's unclear / What we need" in a 2x2 layout. `FindingsTriad` is fixed three columns (wins, risks, decisions). Use `FindingsTriad` for the first three quadrants and add a fourth "Needs" column via composition: render `FindingsTriad` + a parallel "Needs" cell using the same label/body/serif typography and token rhythm.
- **Chrome that should drop:** The serif body text + mono label in each quadrant is already FindingsTriad's contract. No new visual chrome needed.
- **Confidence:** high — FindingsTriad is the direct ancestor; extend, don't create.

**4. ActivityLedger** — verdict: COMPOSE
- **Existing match:** `ActivityLogSection` (patterns/ActivityLogSection.md) + `EntityChip` (primitives/EntityChip.md) + `EntityRow` (patterns/EntityRow.md)
- **Composition strategy:** Each activity entry is a row with: mono timestamp (`when`), small colored dot (`typedot`), and a body cell containing serif text + meta chips. This mirrors `ActivityLogSection`'s audit log rows (timestamp + icon + body + meta), plus `EntityChip` for the account/person/action references. The grid layout (92px / 22px / 1fr) is ActivityLogSection's row pattern.
- **Chrome to drop:** The 7px colored dot (`.typedot`) can be a styled `StatusDot` primitive with `size="xs"` instead of custom CSS. The colored variants (`.t-meeting`, `.t-decision`, etc.) map to existing tone tokens (turmeric, rosemary, larkspur, terracotta, olive).
- **Confidence:** high — ActivityLogSection already exists; this is a reuse with dot-dot variant added.

**5. SuccessOutcome** — verdict: COMPOSE
- **Existing match:** `ReceiptCallout` (patterns/ReceiptCallout.md) — the callout/tinted-box pattern; also shares shape with the `.vD-outcome` CSS (olive tint, label, serif body, signoff).
- **Composition strategy:** Render as a composed `Callout` primitive (tinted box, border per tone) containing: mono label + serif paragraph + mono signoff. This is directly parallel to `ReceiptCallout`'s structure (collapsed form = parent box with label + body + meta). Reuse the olive-tinted background (`--color-garden-olive-10`) and border token.
- **Chrome to drop:** The `.signoff` text is just a meta line; it's already accounted for in `ReceiptCallout`'s "actions" footer slot. Don't add a distinct "signoff" visual — render as trailing attribution.
- **Confidence:** high — `ReceiptCallout` is the proven pattern; this is a direct reuse with no new chrome.

**6. DecisionLog** — verdict: COMPOSE
- **Existing match:** `CommitmentRow` (patterns/CommitmentRow.md) + `EntityChip` (for source attribution)
- **Composition strategy:** Each decision is a row with: mono when (date) + serif decision text with em-emphasized reasoning + mono source (person/doc). This is exactly `CommitmentRow`'s API: when + body + source. The em-emphasis inside the serif text is already handled by standard markdown/HTML (`<em>`) in the body slot.
- **Chrome to drop:** None — the CSS is already minimal and matches CommitmentRow's rhythm.
- **Confidence:** high — CommitmentRow is the existing pattern; this is a direct reuse.

---

## Summary

**Result: 5 of 6 patterns collapse to COMPOSE or minimal EXTEND; 1 is genuinely CREATE.**

- **MeterCluster, FindingsGrid, ActivityLedger, SuccessOutcome, DecisionLog** all have existing pattern analogs (VitalsStrip, FindingsTriad, ActivityLogSection, ReceiptCallout, CommitmentRow) that cover their semantic jobs. Compose them by reusing the existing patterns + dropping the redundant CSS chrome (arrows, hardcoded labels, custom dots).
- **PhaseTimeline** is the only genuine new pattern — no existing primitive or pattern handles the time-axis project timeline with phase bars, milestone markers, and a now-line. Create it with a clean, configurable API.

The mockup's CSS adds visual distinction through layout and tone, not through new primitives. Preserve the tone and typography; reuse the pattern contracts. Estimated rework: consolidate 5 into existing patterns (lightweight documentation updates to their specs), ship 1 new pattern with a minimal API and no hardcoded content.
