# View Purity Audit - 2026-05-07

## Scope

This audit covers the DOS-438 L0 plan scope:

- `src/pages/DailyBriefingRedesign.tsx`
- `src/components/dashboard/*`
- `src/pages/EmailsPage.tsx`
- `src/pages/ActionsPage.tsx`
- `src/pages/MeetingDetailPage.tsx`
- `src/components/meeting/*` per the M7 meeting-prep expansion

Rule checked: display code may own render state, event wiring, local affordance state, and presentation-only formatting. It must not own ranking, grouping, filtering, thresholding, data eligibility, service contract adaptation, or direct command orchestration that belongs in hooks/services.

`src/router.tsx:422`-`426` confirms `/` now renders `DailyBriefingRedesign`. Directory-wide dashboard matches from unrouted components are recorded as residual cleanup, not as routed `DailyBriefingRedesign` behavior.

## Commands Run

Plan read:

```sh
sed -n '1,260p' .docs/plans/wave-W6-redesign/DOS-438-plan.md
```

Candidate sweeps, with `src/components/meeting` added to the L0 plan commands:

```sh
rg -n "useState|useMemo|useCallback|useRef|useTransition" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
rg -n "\.filter\(|\.sort\(|\.reduce\(|\.slice\(|new Set\(|new Map\(|new Date\(" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
rg -n "invoke<|invoke\(" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
rg -n ">=|<=|>|<|threshold|score|priority|rank|overdue|noise|cadence|usually|days|slice\(0" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
rg -n "to[A-Z]|format[A-Z]|group[A-Z]|build[A-Z]|normalize[A-Z]|sanitize[A-Z]" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
```

Refinement checks:

```sh
rg -n "DailyBriefing\b|<DailyBriefing" src --glob '!src/components/dashboard/DailyBriefing.tsx' --glob '!src/components/dashboard/DailyBriefing.test.tsx'
rg -n "export .*DailyBriefing|from \"./DailyBriefing\"|DailyBriefing" src/components/dashboard/index.ts src/components/dashboard --glob '!*.test.tsx'
rg -n "style=\{\{|style=\{|style=\"" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
rg -n "DOS-[0-9]+|cycle-[0-9]+|cycle-" src/pages/DailyBriefingRedesign.tsx src/pages/EmailsPage.tsx src/pages/ActionsPage.tsx src/pages/MeetingDetailPage.tsx src/components/dashboard src/components/meeting
```

Validation:

```sh
pnpm tsc --noEmit
```

## Summary

Counts are semantic finding groups after manual inspection, not raw grep matches.

| Surface | Pass | Exceptions | Violations | Verdict | Required notes |
|---|---:|---:|---:|---|---|
| DailyBriefingRedesign | 6 | 5 | 0 | Pass with residual cleanup | `SurfaceFolio`, section wrappers, nav handlers, and row keys are render-only. Unrouted dashboard grep matches are outside `/`. |
| EmailsPage | 5 | 6 | 7 | Violation | Ranking, score bands, dismissal state, sync controls, and direct invokes need hook/service extraction. |
| ActionsPage | 4 | 5 | 2 | Violation | Filters are mostly in `useActions`; counts and grouping still live in the page. |
| MeetingDetailPage | 4 | 6 | 11 | Violation | Data loads, mutations, transcript/prep flows, trust helpers, attendee filtering, and `src/components/meeting/*` transforms need extraction. |

## DailyBriefingRedesign

### Passes

- `src/pages/DailyBriefingRedesign.tsx:48` consumes `useBriefingViewModel` and branches on the load-state union without reshaping the service contract.
- `src/pages/DailyBriefingRedesign.tsx:148`-`175` renders lead, schedule, predictions, moving, and watch sections from the view model.
- `src/pages/DailyBriefingRedesign.tsx:178`-`201`, `259`-`315` map already-shaped arrays to rows without filtering, sorting, or ranking.
- `src/components/dashboard/Lead.tsx:18`-`45`, `DayStrip.tsx:49`-`72`, `MovingRow.tsx:46`-`133`, and `WatchRow.tsx:31`-`129` consume contract props directly.
- `src/components/dashboard/PredictionsSection.tsx:61`-`66` renders contract-ordered predictions.
- `src/router.tsx:422`-`426` proves the canonical `/` route is the redesigned page.

### Exceptions

- `src/pages/DailyBriefingRedesign.tsx:31`-`42` owns local nav route lookup only.
- `src/pages/DailyBriefingRedesign.tsx:43`-`44`, `209`-`210`, and `321`-`329` adapt primitives for display without changing eligibility/order.
- `src/pages/DailyBriefingRedesign.tsx:238`-`241` and `331`-`389` are navigation/event wiring.
- `src/pages/DailyBriefingRedesign.tsx:317`-`319` and `367`-`380` build breadcrumbs and stable React keys only.
- `src/components/dashboard/PredictionsSection.tsx:34`-`35` owns expand/collapse and accessibility IDs.

### Violations

- None in the routed redesigned surface.

### Residual Risk

- Final rerun against the current worktree no longer finds `src/components/dashboard/DailyBriefing.tsx`; remaining historical references are parity/docs metadata, not runtime imports into `/`.
- `src/components/dashboard/DashboardEmpty.tsx`, `DashboardSkeleton.tsx`, `DashboardError.tsx`, and `DayChart.tsx` still contain inline `style` matches. They are outside the routed redesign import set; follow-up should handle them under the CSS/cardinal-rule cleanup track.

## EmailsPage

### Passes

- `src/pages/EmailsPage.tsx:382`-`395` wraps page states without data shaping.
- `src/pages/EmailsPage.tsx:397`-`399` renders loading and error branches directly.
- `src/pages/EmailsPage.tsx:450`-`581` displays sync stats already returned by the service.
- `src/pages/EmailsPage.tsx:1077`-`1257` renders an email item and child controls from props.
- `src/pages/EmailsPage.tsx:1260`-`1294` is presentation-only date/time formatting.

### Exceptions

- `src/pages/EmailsPage.tsx:29`, `75`-`90`, `312`, `929`-`936`, and `1095` hold UI loading, local dismissal, expansion, form draft, archive visibility, and pin affordance state.
- `src/pages/EmailsPage.tsx:121`-`161` wires focus/visibility refresh events.
- `src/pages/EmailsPage.tsx:948`-`954` resets a local form draft.
- `src/pages/EmailsPage.tsx:1128`-`1134` opens Gmail externally; it does not reshape app data.
- `src/pages/EmailsPage.tsx:1182` truncates subject text for display.
- `src/pages/EmailsPage.tsx:1196`-`1201` strips duplicate entity text from a visible reason only.

### Violations

- `src/pages/EmailsPage.tsx:31`-`41`, `92`-`119`, `125`-`135`, `169`-`210`, `476`-`561`, `634`, `956`-`984`, `1101`-`1143`: direct Tauri command orchestration is in the page and child view components.
- `src/pages/EmailsPage.tsx:53`-`58`: cadence labels hardcode business windows.
- `src/pages/EmailsPage.tsx:212`-`294`: a page-local noise list, entity lookup map, entity/high-priority eligibility, and commitment/question aggregation reshape the email contract.
- `src/pages/EmailsPage.tsx:296`-`306`: "your move" ranking, unread eligibility, score threshold, sort, and cap happen in view code.
- `src/pages/EmailsPage.tsx:314`-`337`: risk-signal classification and gone-quiet split are business grouping/eligibility.
- `src/pages/EmailsPage.tsx:341`-`367`: archive filtering, ranking sort, `hasScores`, and score bands are view-owned classification.
- `src/pages/EmailsPage.tsx:742`-`758`: update threads are filtered/sliced in render using dismissal eligibility.

### Residual Risk

- No fix was made in DOS-438. Follow-up should introduce an email briefing view model/hook that owns IPC, ranking, bands, noise rules, dismissals, failure actions, and section-ready collections.

## ActionsPage

### Passes

- `src/pages/ActionsPage.tsx:146`-`164` consumes `useActions` and `useSuggestedActions`; direct IPC is not in the page.
- `src/pages/ActionsPage.tsx:176`-`183` delegates accept/reject to hooks.
- `src/pages/ActionsPage.tsx:261`-`389` renders rows from hook-owned `actions` and `suggestedActions`.
- `src/pages/ActionsPage.tsx:569`-`585` formats a due date label for display only.

### Exceptions

- `src/pages/ActionsPage.tsx:166`, `186`-`188`, and `440`-`447` are local modal/tab/form draft state.
- `src/pages/ActionsPage.tsx:190`-`212` auto-selects the UI tab based on suggestions.
- `src/pages/ActionsPage.tsx:140`-`143` are tab labels and presentation labels.
- `src/pages/ActionsPage.tsx:224`-`241` registers shell chrome.
- `src/pages/ActionsPage.tsx:466` computes a button class from form validity.

### Violations

- `src/pages/ActionsPage.tsx:35`-`128` and `401`: priority weights, meeting grouping, due-date sort, fallback grouping, and active-tab grouping happen in the view.
- `src/pages/ActionsPage.tsx:168`-`220`: active/completed/overdue counts and folio readiness stats are computed from raw actions in the page.

### Residual Risk

- `useActions` already owns status/priority/search filtering. Follow-up should extend that hook or a sibling view-model hook to return action counts and grouped sections so `ActionsPage` renders pass-through collections.

## MeetingDetailPage

### Passes

- `src/pages/MeetingDetailPage.tsx:943`-`1008` registers shell chrome and buttons after local state is computed.
- `src/pages/MeetingDetailPage.tsx:1010`-`1031` renders loading, error, and empty states.
- `src/pages/MeetingDetailPage.tsx:1124`-`1907` mostly renders branches and child components from derived local variables.
- `src/pages/MeetingDetailPage.tsx:2195`-`2203`, `2520`-`2667`, and `3029`-`3032` are display formatting/truncation.

### Exceptions

- `src/pages/MeetingDetailPage.tsx:191`-`244` contains page lifecycle, dialog, copy, connector, and evidence-disclosure UI state.
- `src/pages/MeetingDetailPage.tsx:246`-`248`, `1057`-`1066`, and `1491`-`1496` persist the local show-all-evidence affordance.
- `src/pages/MeetingDetailPage.tsx:759`-`763` handles clipboard copied state.
- `src/pages/MeetingDetailPage.tsx:1837`-`1900` owns paste-dialog input state.
- `src/pages/MeetingDetailPage.tsx:1929` and `2242`-`2252` own local reveal/editing form state.
- `src/components/meeting/PostMeetingIntelligence.tsx:591`-`624`, `653`-`659`, and `725`-`734` are display-only date/text formatting.

### Violations

- `src/pages/MeetingDetailPage.tsx:250`-`358`, `360`-`706`, `847`-`887`, `1148`-`1178`, `2095`-`2301`, and `2608`-`2765`: direct Tauri command orchestration lives in the page and nested view helpers.
- `src/pages/MeetingDetailPage.tsx:281`-`304`: the page adapts `MeetingIntelligence` into `FullMeetingPrep`, including fallback shape construction.
- `src/pages/MeetingDetailPage.tsx:306`-`320`: post-meeting intelligence eligibility is computed in the page.
- `src/pages/MeetingDetailPage.tsx:100`-`103`, `382`-`437`, and `907`-`941`: Granola retry windows, intervals, and auto-refresh orchestration are view-owned business policy.
- `src/pages/MeetingDetailPage.tsx:650`-`681`, `765`-`815`, and `817`-`840`: prep prefill/share/request flows select, cap, sanitize, and compose domain content in the view.
- `src/pages/MeetingDetailPage.tsx:858`-`897` and `961`-`978`: minutes-until-meeting, three-days-out, ready/fresh collaboration gates, and transcript action visibility are hardcoded windows/rules.
- `src/pages/MeetingDetailPage.tsx:1057`-`1105`, `2675`-`2899`, and `2967`-`3027`: trust partitioning, agenda/discuss de-duplication, lifecycle lookup, normalization, and sanitization are transformation layers between contract and component.
- `src/pages/MeetingDetailPage.tsx:1930`-`1935` and `2902`-`2964`: attendee filtering and attendee-context/insight/signal merging happen in display code.
- `src/pages/MeetingDetailPage.tsx:2242`-`2290`, `2339`-`2362`, and `2440`-`2459`: the plan editor merges proposed/user agenda, parses persisted strings, overrides proposed items, and applies dismissal semantics locally.
- `src/components/meeting/meeting-prep-utils.ts:22`-`100`: DOS-434 extracted prep parsing/building under `components/meeting`, but it still performs normalization, de-duplication, section assignment, and trust partition construction in component-owned territory.
- `src/components/meeting/PostMeetingIntelligence.tsx:69`-`76`, `390`-`392`, `630`-`651`, `661`-`723`: post-meeting captures/actions are grouped, sorted, filtered, and classified by urgency/status/subtype in the component.

### Residual Risk

- Follow-up should extract a `useMeetingDetail`/meeting-detail view-model layer with typed command methods and pre-shaped sections. `src/components/meeting/meeting-prep-utils.ts` should move to a hook/lib/service-owned layer or be replaced by service output.

## Fixed Violations

- None. The task requested an audit and explicitly directed genuine violations to be reported for follow-up instead of fixed in DOS-438.

## Follow-Up Work

- Email surface: extract IPC, ranking, score bands, noise/cadence rules, failure actions, and dismissal projection from `EmailsPage`.
- Actions surface: move action counts and grouped active sections into `useActions` or a dedicated view-model hook.
- Meeting surface: extract meeting data load/mutations, transcript orchestration, trust/agenda/attendee shaping, and post-meeting grouping into hooks/services.
- Dashboard residuals: clean inline styles in unrouted dashboard leftovers.

## Gate Status

- `pnpm tsc --noEmit`: passed on 2026-05-07.
- Grep audit method: documented above.
- Source ticket-comment check: no source files were edited in this ticket.
