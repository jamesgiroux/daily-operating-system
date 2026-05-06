# DOS-413 — BriefingViewModel contract — L0 plan

**Wave:** W0 (single-ticket wave; blocks all of W1–W6)
**Status:** Draft, awaiting L0 unanimous approval
**Reviewers required (independent):**
1. `/codex` challenge against this plan
2. `architect-reviewer` (this is a service-shape contract, not a UI ticket — architect over design-consultation)
3. `/codex` independent consult

## 1. Ticket reference and acceptance summary

[DOS-413](https://linear.app/a8c/issue/DOS-413) — Define BriefingViewModel contract.

Acceptance criteria from ticket:

- [ ] `BriefingViewModel` type exported from `src/types/briefing.ts`
- [ ] One variant per section (Lead, Schedule, Predictions, Moving, Watch)
- [ ] All five states (loading / error / empty / cached-stale / present) modeled per section
- [ ] No business logic in the type — pure shape
- [ ] ADR or design note documenting the contract

## 2. What I'm building

- `src/types/briefing.ts` — exports `BriefingViewModel` and the per-section discriminated unions: `LeadSection`, `ScheduleSection`, `PredictionsSection`, `MovingSection`, `WatchSection`. Each section is a `discriminator` union over `state ∈ {loading, error, empty, cachedStale, present}` with a `data` payload only on `present` and `cachedStale` (and an `error` field on `error`).
- `.docs/adr/ADR-NNNN-briefing-view-model-contract.md` — design note explaining the shape, the rationale for "view consumes pass-through," and the relationship to the W2 services that produce slices.
- Spec linkage: update `.docs/design/surfaces/DailyBriefingRedesign.md` to reference the contract.

Concrete shapes (sketch — final will be in code):

```ts
type SectionState<TData> =
  | { state: "loading" }
  | { state: "error"; error: { code: string; message: string } }
  | { state: "empty" }
  | { state: "cachedStale"; data: TData; cachedAt: string; reason: string }
  | { state: "present"; data: TData };

type LeadSection = SectionState<{
  headline: string;
  punchLine?: string;          // emphasis span in the headline
  focusCapacity: string;       // mono line: "3h available · 2 deep work blocks · light afternoon"
}>;

type ScheduleSection = SectionState<{
  todayCount: number;
  meetings: ScheduleMeeting[]; // each rendered via MeetingSpineItem
  dayChart: DayChartViewModel; // bars + nowLine
}>;

type PredictionsSection = SectionState<{
  count: number;
  predictions: PredictionViewModel[]; // populated only when count > 0
}>;

type MovingSection = SectionState<{
  entities: MovingEntityViewModel[]; // ≤3, service enforces cap
}>;

type WatchSection = SectionState<{
  rows: WatchRowViewModel[];
}>;

interface BriefingViewModel {
  date: string;            // "Thursday, April 23, 2026"
  lead: LeadSection;
  schedule: ScheduleSection;
  predictions: PredictionsSection;
  moving: MovingSection;
  watch: WatchSection;
}
```

The downstream view models (`MovingEntityViewModel`, `WatchRowViewModel`, `MeetingSpineViewModel`, etc.) are sketched in the ADR and finalized when each W2 service ticket lands.

## 3. What I'm NOT building

- Service implementations (W2 — DOS-414..419)
- Frontend hook to consume the contract (W5 — DOS-429)
- View component (W5 — DOS-429)
- Mock fixtures beyond what the ADR examples need

Files explicitly off-limits in this ticket:
- Anything under `src/components/`
- Anything under `src-tauri/src/services/`
- The reference HTML

## 4. Reuse audit

This is a service-shape contract, not a UI ticket — no design tokens, primitives, or patterns to reuse. The TypeScript shape composes existing types where they exist:

- Reuse `EntityRef` (existing — `src/types/entities.ts`) for the `entity` field on Moving rows and Watch rows.
- Reuse `HealthBand` / `HealthScore` types for provenance stats.
- Reuse `ClaimRef` (if present in current substrate) so trust-band integration in W4 has a hook.
- Anywhere existing types are insufficient, reference them as TODOs in the ADR rather than expand them in this ticket.

## 5. Service / view-model contract surface

This ticket IS the contract surface. Downstream:

- `MovingService` (DOS-414) produces `MovingSection["data"]`.
- `WatchService` (DOS-415) produces `WatchSection["data"]`.
- `BriefingScheduleService` (DOS-417) produces `ScheduleSection["data"]`.
- `PredictionsService` (DOS-418) produces `PredictionsSection["data"]`.
- Email lift (DOS-416) consumes inside `MovingService`'s email-source code path; doesn't affect this contract.
- `useBriefingViewModel()` hook (DOS-429) composes Tauri command results into the full `BriefingViewModel`.

Upstream (consumed by this ticket): nothing. This is the foundation.

## 6. Display-layer purity

N/A — no view code in this ticket. The contract is the *enforcement mechanism* for display-layer purity downstream: by giving the view a finalized shape per state, there's nothing left for the view to filter, sort, or rank.

## 7. Test plan

- `pnpm tsc --noEmit` — zero type errors after the new types ship
- Compile-check fixture: a `mockBriefingViewModel` constant constructed inline in a test file proves all five states can be expressed for every section
- Type-narrowing test: a discriminated-union exhaustiveness assertion in test code (using TypeScript's `never` exhaustion pattern) proves every state is handled

Concrete commands:

```sh
pnpm tsc --noEmit
pnpm test src/types/briefing.test.ts
```

## 8. Risk + rollback

**Risks:**

- **Over-fitting the contract to today's reference HTML** — risk that W3-W5 implementation discovers fields the contract didn't anticipate. Mitigation: the ADR documents extension points; downstream tickets can extend the per-section data type without breaking the section discriminator.
- **Premature trust-band coupling** — DOS-427 wires trust bands. If this contract over-specifies trust shape now, it boxes in DOS-320 (parent v1.4.0 W6). Mitigation: this ticket includes only `claimRefs?: ClaimRef[]` on data types that need it; trust band shape arrives via DOS-427 layered on top.
- **Discriminated union ergonomics** — `state: "present"` vs `data` access can be verbose in JSX. Mitigation: ship a small `select()` helper alongside the types so view components write `select(model.lead, { present: ... })` not `model.lead.state === "present" && model.lead.data ...`.

**Rollback:** types-only ticket. Rollback = revert the commit. Zero runtime effect.

## 9. Wave dependencies

- **Consumes:** nothing (W0 is foundational).
- **Blocks:** DOS-414, 415, 416, 417, 418, 419 (all W2 services); DOS-420, 421, 422, 426 (W1 components that consume the section types); DOS-432, 433 (W5 surface uplifts that compose the contract); DOS-427, 428 (W4 wire-ins that extend section data).
- All redesign tickets W1+ have DOS-413 in their `blockedBy`.

## 10. Merge gate artifacts

Concrete artifacts the L2 reviewer + L3 wave gate look for:

- `pnpm tsc --noEmit` clean (no compile errors)
- `BriefingViewModel` exported and importable from `@/types/briefing`
- ADR at `.docs/adr/ADR-NNNN-briefing-view-model-contract.md` landed
- Type-narrowing exhaustiveness test passes
- Sketch of `select()` helper or equivalent ergonomics shipped with the types

## L0 reviewer dispatch

Once this plan lands on the branch, dispatch in parallel:

1. `/codex challenge "Review .docs/plans/daily-briefing-redesign-W0/DOS-413-plan.md adversarially. Question whether the discriminated-union shape over-engineers vs simpler shapes; question whether trust-band extension points are correctly placed; question whether section-level state granularity matches actual loading patterns."`

2. `architect-reviewer subagent` — read this plan, evaluate against DailyOS architecture conventions and the existing service/view boundary patterns. Output: approve / approve-with-revisions / reject with rationale.

3. `/codex consult --independent "Independent read: does this contract correctly cover the redesign reference at .docs/design/reference/surfaces/briefing-redesign.html? Cite any section the contract misses or over-specifies."`

Pass rule: unanimous approval. 2 revision cycles without convergence ⇒ L6 escalation.
