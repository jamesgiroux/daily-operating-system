# DOS-435 - /week deprecation - L0 plan

**Wave:** W6 redesign  
**Status:** L0 draft  
**Scope:** Remove the standalone `/week` route once the redesigned briefing owns
the week-shaped schedule experience through the briefing service, `DayStrip`,
and same-source schedule data.

## 1. Acceptance criteria

- [ ] `/week` no longer renders `WeekPage`.
- [ ] External `/week` bookmarks redirect to `/`.
- [ ] If product/UX rejects redirect before implementation, the accepted
  alternative is a hard 404 with no `/week` navigation affordance.
- [ ] The routed `/` is already the production/default daily briefing redesign.
- [ ] The briefing schedule slice already exposes the week shape lifted by
  DOS-417; this ticket does not reimplement week grouping in React.
- [ ] `FloatingNavIsland` no longer includes `This Week`.
- [ ] Command menu and nav route maps no longer offer `/week`.
- [ ] Week-only page files and tests are deleted.
- [ ] No new `DOS-` references are added to code comments.

## 2. Hard dependencies

Do not start implementation until both dependencies are true.

- DOS-417 has landed and passed L1/L2: the +/-7 day shape is no longer owned
  exclusively by `WeekPage.tsx`; schedule-service parity tests cover
  local-date grouping, earlier/recent/today/future buckets, Mon-Fri density,
  and `daysBefore = 7`, `daysAfter = 7`; `DayStrip` can support the week scan
  from same-source schedule data.
- DOS-431 has landed and passed cutover validation: `/` routes to
  `DailyBriefingRedesign` by default in production, and legacy `DashboardPage`
  is no longer the normal daily briefing route.

If either dependency is missing, keep `/week` in place and mark this ticket
blocked, not partially complete.

## 3. Repo reality

- `src/router.tsx` imports `WeekPage`, marks `/week` as a magazine route, maps
  `week` to `/week`, defines `weekRoute`, and adds it to the route tree.
- `src/pages/WeekPage.tsx` calls `get_meeting_timeline` with `daysBefore: 7`,
  `daysAfter: 7`, then derives buckets, readiness stats, future-meeting counts,
  and shell config locally.
- `src/pages/weekPageViewModel.ts` owns week number, date range, Mon-Fri
  `DayShape[]`, density, and epigraph helpers.
- Route/nav references also exist in `FloatingNavIsland.tsx`,
  `DailyBriefingRedesign.tsx`, `CommandMenu.tsx`, and `useMagazineShell.ts`.

## 4. UX and backward compatibility

Decision: redirect `/week` to `/`.

Reasoning: bookmarked or recent `/week` links should land on the daily briefing
because the replacement week scan now lives there. A 404 is cleaner
mechanically, but it is less useful when `/` is the intended replacement.

Implementation notes:

- Prefer an explicit TanStack route redirect from `/week` to `/`.
- Do not keep `WeekPage.tsx` as a redirect component.
- No query/hash preservation is required; `/week` has no documented query
  contract.
- If UX chooses 404, remove the route and rely on app not-found behavior.

## 5. Files to delete

- `src/pages/WeekPage.tsx`
- `src/pages/WeekPage.module.css`
- `src/pages/weekPageViewModel.ts`
- `src/pages/WeekPage.test.tsx`
- `src/pages/weekPageViewModel.test.ts`

If a listed file is already absent at implementation time, record that in PR
notes and continue. Do not recreate missing tests just to delete them.

## 6. Files to edit

- `src/router.tsx`: remove `WeekPage` import, remove `/week` from
  `MAGAZINE_ROUTE_IDS`, remove `week: "/week"` from `handleNavNavigate`, and
  replace `weekRoute` with redirect-to-`/` or remove it for the 404 path.
- `src/components/layout/FloatingNavIsland.tsx`: remove `week` from nav item
  types, remove the `This Week` item, and drop unused `Calendar` import.
- `src/pages/DailyBriefingRedesign.tsx`: remove `week: "/week"` from
  `NAV_ROUTES`; keep `DayStrip` as the briefing's week affordance.
- `src/components/layout/CommandMenu.tsx`: remove the `This Week` command, or
  retarget it to `/` only if UX wants a command alias.
- `src/hooks/useMagazineShell.ts`: remove `week` from
  `MagazineShellConfig.activePage`.
- `src/hooks/useMagazineShell.test.tsx`: update expectations if they mention
  `week`.
- `src/components/layout/README.md`: update stale examples only if touched in
  the implementation PR.

## 7. Service cleanup

This is primarily frontend route cleanup. If DOS-417 introduced a temporary
command or service entrypoint used only by `/week`, remove it after verifying no
remaining consumer needs it. Do not delete reusable schedule-service helpers
still used by the briefing schedule slice or `DayStrip`. No new calendar
fetching behavior belongs in this ticket.

## 8. Coordination with DOS-431

Landing order:

1. DOS-417 lifts and tests the week shape in the schedule service.
2. DOS-431 flips the routed daily briefing default to `DailyBriefingRedesign`.
3. DOS-435 deletes `/week`.

Do not land DOS-435 before DOS-431. Before the routed redesign is default,
`/week` remains the only standalone week-shaped user surface.

## 9. Out of scope

- changing `ScheduleViewModel`, `DayStrip`, or calendar ingestion
- redesigning the daily briefing schedule section
- deleting unrelated weekly report routes such as `/me/reports/weekly_impact`
- broad CSS trimming owned by DOS-437
- view-purity audit owned by DOS-438

## 10. L1 self-validation gates

Static checks:

- `pnpm tsc --noEmit`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx`
- `pnpm test src/hooks/useMagazineShell.test.tsx`
- existing router/nav tests if present, otherwise add one focused test for the
  removed `/week` affordance

Search gates:

- `rg -n "WeekPage|weekPageViewModel|WeekPage.module.css" src`
- `rg -n 'to: "/week"|go\\("/week"\\)|"/week"|week: "/week"' src`
- `rg -n "id: 'week'|activePage.*week|This Week" src/components src/pages src/hooks`

Expected result: no user-visible `/week` navigation remains. One explicit
redirect route in `src/router.tsx` is allowed when redirect is chosen.

Runtime smoke:

- `/` renders `DailyBriefingRedesign` with `DayStrip`
- `/week` redirects to `/` with no console error
- command menu and `FloatingNavIsland` do not show `This Week`

Comment hygiene: grep touched source files for `DOS-[0-9]+` in comments and
remove any new or newly-touched issue-ticket comments.

## 11. L2 review gates

- **code-reviewer:** route cleanup cannot mount deleted `WeekPage`; imports are
  clean; tests cover route/nav behavior.
- **architect-reviewer:** DOS-417 and DOS-431 evidence is attached; any removed
  command/service code is truly `/week`-only.
- **design-reviewer:** removing `This Week` from global chrome does not strand
  the week affordance because `DayStrip` on `/` carries the schedule scan.

## 12. Implementation order

1. Verify DOS-417 and DOS-431 have landed.
2. Delete the week page, CSS module, helper, and tests.
3. Update `/week` route behavior using the accepted redirect/404 call.
4. Remove `/week` from nav chrome, command menu, redesign route mapping, and
   magazine shell active-page types.
5. Run grep cleanup, L1 checks, and runtime smoke.
6. Attach dependency evidence and redirect/404 decision to PR notes.
