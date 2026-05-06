# Daily Briefing Redesign — Parallel Wave Plan

**Date:** 2026-05-06
**Target:** Cutover the routed `/` from current DailyBriefing to DailyBriefingRedesign with W6 cleanup landed
**Source of truth:** Linear project [Design System](https://linear.app/a8c/project/design-system-1bae2a9614cc) — tickets DOS-413..DOS-438
**Companion docs:**
- `.docs/plans/v1.4.0-daily-briefing-redesign-decisions.md` — locked decisions (D1-D4)
- `.docs/plans/v1.4.0-daily-briefing-redesign-tickets.md` — ticket-list draft and rationale
- `.docs/design/reference/surfaces/briefing-redesign.html` — canonical reference render
- `.docs/plans/v1.4.0-waves.md` — parent v1.4.0 wave plan (this redesign track is parallel and depends on its W6 outputs at DOS-320 and DOS-411)

## Product thesis alignment

Four invariants must hold across every wave:

1. **No logic in the display layer.** Services own data shape; the view consumes pass-through.
2. **Reuse before extraction.** New CSS only when no token, primitive, or pattern fits.
3. **Restraint.** Moving caps at 3 entities; lede caps at 2 sentences; signal feed caps at 5 items per row; Predictions defaults to a single line.
4. **Trust visible.** v1.4.0 W6 trust-band data and claim-lifecycle correction state appear in the rendered briefing, not buried.

The cutover gate (W6) passes only if the routed `/` runs `DailyBriefingRedesign.tsx`, the audit script returns clean for the new canonical reference, and the audit (DOS-438) confirms zero data-shaping logic in the view.

## Wave shape — seven waves

| Wave | Tickets | Parallel agents | Wall-clock |
|------|---------|----------------|------------|
| **W0** — Contract | DOS-413 | 1 | half-day |
| **W1** — Components | DOS-420 (DayStrip), DOS-421 (InferredActionSelector), DOS-422 (SignalDot), DOS-426 (Lead) | 4 | 1-2 days |
| **W2** — Services | DOS-414 (Moving), DOS-415 (Watch), DOS-416 (email lift), DOS-417 (calendar lift), DOS-418 (Predictions), DOS-419 (lifecycle adapter) | 6 | 2-3 days |
| **W3** — Patterns | DOS-423 (MovingRow), DOS-424 (WatchRow), DOS-425 (PredictionsSection) | 3 | 1-2 days |
| **W4** — Wire-ins | DOS-427 (trust band), DOS-428 (claim-lifecycle SignalDot), DOS-434 (MeetingDetailPage absorption) | 3 | 2 days |
| **W5** — Surface integration | DOS-429 (surface), DOS-430 (feature flag), DOS-432 (/emails uplift), DOS-433 (/actions uplift) | 4 | 2-3 days |
| **W6** — Cutover + cleanup | DOS-431 (canonical), DOS-435 (/week deprecation), DOS-436 (archive cards), DOS-437 (trim CSS), DOS-438 (view-purity audit) | 5 (sequential within: DOS-431 first) | 3-4 days |

Total: 26 tickets. **Cross-wave ordering is hard.** No wave starts until the prior wave clears L3 and L5 (where applicable).

## What "parallel" means here

Each agent in a wave gets:

1. **A frozen contract.** The Linear ticket text is the contract. Agents do not invent shapes.
2. **An exclusive file/dir allowlist.** Listed in the ticket's `Wave + agent contract` section.
3. **A deny list.** Files no other agent in the wave will touch.
4. **A merge gate.** Concrete artifact (test, lint, audit-reference.py output) proving done before the next wave starts.

Within a wave the agents fan out. Across waves the ordering is hard.

---

# Review system — L0-L6 governance

This track adopts the v1.4.0 ladder with two design-track-specific adaptations:

- **L0 plan-review reviewer matrix gains a design-consultation seat** for any ticket producing a user-facing primitive, pattern, or surface (DOS-420 onward; not for service-only tickets).
- **L4 surface QA gains accessibility-tester** as a mandatory seat for user-facing components (DOS-420, 421, 422, 423, 424, 425, 426, 429, 432, 433).

## Review ladder

| Layer | When | Reviewers (independent) | Pass rule |
|---|---|---|---|
| **L0 — Plan review** | Per agent, pre-code | (1) `/codex` challenge against plan (2) **design-consultation** for UI tickets / **architect-reviewer** for service tickets (3) `/codex` independent consult | Unanimous approval |
| **L1 — Self-validation** | Per agent, pre-PR | Implementing agent | Evidence artifacts in PR body |
| **L2 — Diff review** | Per PR, pre-merge | (1) `/codex review` (2) `code-reviewer` subagent (3) domain reviewer (design or backend per ticket) | All three approve |
| **L3 — Wave adversarial** | After all wave PRs merged | (1) `/codex` challenge mode against integrated wave (2) `architect-reviewer` on integrated diff (3) `audit-reference.py --enforce-baseline` against any reference HTML touched | All approve, audit clean |
| **L4 — Surface QA** | W3 onward | `/qa-only` first; `/qa` from W5; `accessibility-tester` mandatory for user-facing | Zero blockers |
| **L5 — Drift check** | After W3 and W6 | `/plan-eng-review` + `architect-reviewer` comparing integrated state to redesign end-state | No drift |
| **L6 — Human review** | Only when L0-L5 escalate | You | Tiebreaker / direction call |

**Pacing rule.** No wave starts until prior wave clears L3 and L5 (where applicable). No agent codes before L0 clears unanimously. **2 revision cycles** on the same plan or PR without convergence ⇒ L6 escalation.

## Plan-review template (mandatory L0 input)

Every agent produces this 1-page plan before any code. Save to `.docs/plans/daily-briefing-redesign-WN/<DOS-NNN>-plan.md`. All ten sections are mandatory; "none" is an acceptable answer where truthful.

1. **Ticket reference and acceptance summary.** Linear ID + the acceptance checklist verbatim.
2. **What you're building.** Files added/modified. Ties to the ticket's File allowlist.
3. **What you're NOT building.** Files in the ticket's Deny list, plus any out-of-scope clarifications.
4. **Reuse audit.** Existing tokens, primitives, patterns this work will compose. Required for UI tickets — naming each one prevents reinvention.
5. **Service / view-model contract surface.** What shape does this ticket consume from upstream services and produce for downstream consumers? Cite DOS-413 entries by name.
6. **Display-layer purity.** For UI tickets: confirm zero `useState` for data filtering, zero `.filter`/`.sort`/`.reduce` on view-model arrays, zero conditionals beyond render-state checks. List exceptions.
7. **Test plan.** Unit, integration, accessibility, audit-reference.py. Concrete commands.
8. **Risk + rollback.** What breaks if this lands wrong? What's the rollback?
9. **Wave dependencies.** Which prior-wave outputs am I consuming? Which downstream wave consumes me?
10. **Merge gate artifacts.** Concrete commands + expected output that prove done.

## Per-wave merge gates

| Wave | Gate artifacts (all required) |
|---|---|
| W0 | `pnpm tsc --noEmit` clean · ADR doc landed · `BriefingViewModel` exported and consumable |
| W1 | All 4 components ship: `pnpm tsc --noEmit` clean · audit-reference.py against `briefing-redesign.html` shows scoped class names land · accessibility-tester L4 pass on each component |
| W2 | All 6 services ship: `cargo test services::{moving,watch,briefing_schedule,predictions}` pass · `cargo clippy -- -D warnings` clean · Tauri commands exposed and callable |
| W3 | All 3 patterns ship: same gates as W1 |
| W4 | Trust band visible on MeetingSpineItem in reference render · SignalDot corrected variant visible · MeetingDetailPage absorbs former expansion-panel content (audit doc lands) |
| W5 | DailyBriefingRedesign.tsx renders end-to-end against MockBriefingViewModel · feature flag toggles dev-mode · /emails and /actions reference-audit clean |
| W6 | Routed `/` is DailyBriefingRedesign in production · `audit-reference.py` clean across all surfaces · view-purity audit (DOS-438) returns clean or with documented exceptions · `/week` returns 404/redirect · BriefingMeetingCard removed · editorial-briefing.module.css ≤400 lines |

## Cross-track dependencies (parent v1.4.0)

Two redesign tickets depend on parent v1.4.0 W6 outputs:

| Redesign ticket | v1.4.0 dependency | Parent ticket |
|---|---|---|
| DOS-427 (trust band) | per-claim trust-band data shape | DOS-320 |
| DOS-428 (corrected SignalDot) | claim-lifecycle correction record shape | DOS-411 |

If parent W6 slips past redesign W4 timing, redesign W4 stalls. Mitigation: W1, W2, W3 can complete in parallel without these dependencies; W4 wire-ins are the gate.

## Risk + sequencing notes

- **DOS-414 (MovingService) and DOS-419 (lifecycle adapter) share a file** (`services/moving.rs`). The lifecycle adapter is allowlisted only to the lifecycle code-path; coordination needed if both ship in the same merge cycle. Recommend: DOS-414 lands first, DOS-419 layers on.
- **DOS-417 (calendar grouping lift)** modifies BOTH `DailyBriefing.tsx` and `WeekPage.tsx`. /week deprecation (DOS-435) won't have happened yet at W2; the WeekPage edit must be a removal only, no behavioral change.
- **DOS-426 (Lead pattern)** verifies whether existing source already covers the spec. Possible early completion if spec matches current implementation.
- **DOS-427 backward-compat path is mandatory.** If DOS-320 hasn't shipped at W4 time, the trust-band variant gracefully falls back to existing fresh/developing/sparse rendering. No hard W4 stall.
- **W6 sequencing within wave** is sequential: DOS-431 first (cutover), then DOS-435 (/week deprecation), then DOS-436 (archive cards), then DOS-437 (CSS trim), then DOS-438 (audit). DOS-438 can run in parallel with DOS-435/436/437 if DOS-431 has merged.

## Out of scope for this track

- Switching the routed `/week` to anything new — straight deprecation only.
- Changing `/account/$id`, `/person/$id`, `/project/$id` detail pages beyond the MeetingDetailPage absorption (DOS-434).
- New ability runtime additions beyond consuming existing DOS-218/DOS-219 outputs.
- Mobile-specific layout work beyond the existing 720px breakpoint already in `briefing-redesign.html`.

## Reading order for agents

1. This doc — wave assignments, gates, governance.
2. `v1.4.0-daily-briefing-redesign-decisions.md` — D1-D4 + Shape A row decision.
3. The Linear ticket — frozen contract + acceptance criteria.
4. `briefing-redesign.html` — canonical reference render for UI tickets.
5. ADRs referenced in the ticket.
