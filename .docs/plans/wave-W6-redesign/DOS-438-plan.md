# DOS-438 - View-purity audit - L0 plan

**Wave:** W6 redesign  
**Status:** L0 draft  
**Scope:** Post-cutover audit proving redesigned briefing surfaces did not move business logic into display-layer code.

## 1. Acceptance criteria

- [ ] Audit report covers `DailyBriefingRedesign`, `EmailsPage`, `ActionsPage`, and `MeetingDetailPage`.
- [ ] Each surface has a pass count, exception count, violation count, and verdict.
- [ ] Every candidate finding is classified as pass, allowed exception, or violation.
- [ ] Zero unaccounted-for violations remain at close.
- [ ] Confirmed violations are fixed in the owning layer or explicitly tracked as accepted follow-up work.
- [ ] `.docs/design/_audits/view-purity-2026.md` lands with commands, evidence, findings, exceptions, and fix references.
- [ ] No touched source comments contain ephemeral `DOS-` references.

## 2. Cardinal rule

Enforce the redesign rule: no business logic in the display layer; services own data shape and views consume pass-through contracts.

Display code may own render state, event wiring, local affordance state, and presentation-only formatting. It must not own ranking, grouping, filtering, thresholding, data eligibility, service contract adaptation, or direct command orchestration that belongs in hooks/services.

## 3. Surfaces in scope

| Surface | Primary files | Notes |
|---|---|---|
| DailyBriefingRedesign | `src/pages/DailyBriefingRedesign.tsx`, `src/components/dashboard/*` | Canonical `/` after DOS-431 cutover. |
| EmailsPage | `src/pages/EmailsPage.tsx` | Include ranking, score bands, dismissal, sync controls, direct invokes. |
| ActionsPage | `src/pages/ActionsPage.tsx` | Include tab/filter state, grouping, priority/due-date logic, hook usage. |
| MeetingDetailPage | `src/pages/MeetingDetailPage.tsx` | Include direct invokes, transcript/prep orchestration, trust helpers, attendee filtering. |

Out of scope: unrelated account/project/person/report surfaces and historical docs unless imported behavior directly affects one of the four audited pages.

## 4. Audit method

Run grep to generate candidates, then inspect manually. Grep output is not itself a verdict.

```sh
rg -n "useState|useMemo|useCallback|useRef|useTransition" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard
rg -n "\\.filter\\(|\\.sort\\(|\\.reduce\\(|\\.slice\\(|new Set\\(|new Map\\(|new Date\\(" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard
rg -n "invoke<|invoke\\(" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard
rg -n ">=|<=|>|<|threshold|score|priority|rank|overdue|noise|cadence|usually|days|slice\\(0" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard
rg -n "to[A-Z]|format[A-Z]|group[A-Z]|build[A-Z]|normalize[A-Z]|sanitize[A-Z]" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard
```

Anti-pattern buckets:

- `useState` / `useMemo` filtering of contract data.
- Inline `.sort`, `.filter`, `.reduce`, score bands, or thresholds.
- Hardcoded business rules: priority weights, stale/overdue windows, score cutoffs, cadence labels, noise lists, service-owned caps.
- Direct Tauri `invoke` calls outside hooks or service adapters.
- Transformation layers between contract and component: local `toX`, `formatX`, `groupX`, `normalizeX`, `sanitizeX`, or `buildX` helpers that alter domain meaning.

## 5. Classification rules

**Pass:** candidate is render-only or already-shaped contract consumption.

**Exception:** legitimate local UI ownership. Allowed examples: nav state, route lookup, click handlers, active tabs, local search input, form draft state, expand/collapse, show/hide details, dialog open state, hover/focus/accessibility IDs, and local in-flight flags.

**Violation:** the view interprets domain data, reshapes a service contract, ranks/groups/filters data, computes business windows, or owns command orchestration that should live in a hook/service.

Exceptions must distinguish UI-state concerns from contract-shape violations. "Small helper" is not a valid exception if it changes data eligibility, order, grouping, or semantics.

## 6. Per-surface checklist

The audit report must include this table with real counts:

| Surface | Pass | Exceptions | Violations | Verdict | Required notes |
|---|---:|---:|---:|---|---|
| DailyBriefingRedesign | TBD | TBD | TBD | TBD | `SurfaceFolio`, section wrappers, nav handlers, row keys. |
| EmailsPage | TBD | TBD | TBD | TBD | ranking, score bands, dismissal state, sync controls, direct invokes. |
| ActionsPage | TBD | TBD | TBD | TBD | filters, counts, grouping, due-date formatting, action hooks. |
| MeetingDetailPage | TBD | TBD | TBD | TBD | data loads, mutations, transcript/prep flows, trust helpers, attendee filtering. |

Each surface section must list candidate line refs under `Passes`, `Exceptions`, `Violations`, and `Residual risk`.

## 7. Output deliverable

Primary artifact:

- `.docs/design/_audits/view-purity-2026.md`

Required contents:

- audit date and scope
- exact commands run
- summary table from section 6
- detailed findings per surface
- accepted exceptions with rationale
- fixed violations with commit/PR references
- unfixed violations only if accepted and linked to follow-up work

The PR body can summarize the report, but the markdown audit doc is the closure source of truth.

## 8. Files affected

Planned edit:

- `.docs/design/_audits/view-purity-2026.md`

Allowed only if confirmed violations surface:

- minimal source fixes in the owning page, component, hook, or service adapter
- focused tests for moved logic

Do not edit unrelated surfaces, tokens, reference HTML, or historical docs.

## 9. Fix policy

- View-model shaping moves to an existing hook or a narrow hook beside the data source.
- Domain classification moves to the service/composer that owns the source data.
- Mutation orchestration moves behind typed hook methods; component handlers stay thin.
- Presentation-only formatting can remain in view code only when it does not alter order, grouping, eligibility, or domain meaning.
- Fixes preserve existing user-visible behavior unless L2 explicitly accepts a correction.

## 10. Sequencing

DOS-438 lands last in W6.

Hard dependency: DOS-431 has merged and `/` is canonical.

DOS-438 can run in parallel with DOS-435, DOS-436, and DOS-437 only after DOS-431 has merged. If parallel cleanup touches audited files, rerun all grep passes against the final merged state before closing the report.

Implementation order:

1. Verify DOS-431 landed.
2. Run grep candidate sweep.
3. Classify each candidate per surface.
4. Fix confirmed violations or link accepted follow-ups.
5. Write `.docs/design/_audits/view-purity-2026.md`.
6. Rerun grep checks and update final counts.

## 11. L1 self-validation gates

- `pnpm tsc --noEmit`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx`
- `pnpm test src/hooks/useBriefingViewModel.test.ts`
- Targeted tests for any source file changed by fixes.
- Rerun all grep commands from section 4; every remaining match is represented in the audit report.
- `rg -n "DOS-[0-9]+"` on touched source files returns no ticket-comment violations.
- `test -f .docs/design/_audits/view-purity-2026.md`
- Audit report has no `TBD` values and zero unaccounted-for violations.

If Rust service/composer code changes, also run:

- `cargo test --manifest-path src-tauri/Cargo.toml briefing`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins -- -D warnings`

## 12. L2 review gates

- **Architecture review:** classifications honor service-owned data shape and do not hide display-layer adapters as helpers.
- **Code review:** fixes are minimal, typed, tested, and behavior-preserving.
- **Design review:** accepted exceptions are UI-state only and do not weaken the redesign contract.
- **Test review:** moved logic is covered at its new ownership layer.
- **Release review:** DOS-438 is the final W6 cleanup artifact, with any accepted follow-up tracked outside the closed audit.
