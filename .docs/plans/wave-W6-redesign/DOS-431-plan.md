# DOS-431 - Daily Briefing redesign cutover - L0 plan

**Wave:** W6 redesign
**Status:** L0 draft
**Scope:** Make `DailyBriefingRedesign` the canonical `/` route after
live-data and acceptance gates pass, then retire the legacy dashboard branch.

## 1. Acceptance criteria

- [ ] `/` renders `src/pages/DailyBriefingRedesign.tsx` without checking
  `daily_briefing_redesign_enabled`.
- [ ] The legacy `DashboardPage` route branch in `src/router.tsx` is removed.
- [ ] `daily_briefing_redesign_enabled` is no longer part of the public
  `FeatureFlags` wire shape after the final cutover step.
- [ ] Legacy `DailyBriefing.tsx` and `DailyBriefing.module.css` are deleted
  once no production route imports them.
- [ ] `BriefingMeetingCard` is deleted if DOS-434 has fully absorbed its prep
  detail content into `MeetingDetailPage`; otherwise DOS-431 leaves it in place
  and names the blocking residual usage.
- [ ] Config examples and docs no longer tell users to set the redesign flag.
- [ ] Backward compatibility is intentionally not preserved. This ticket is the
  migration boundary from legacy briefing to redesigned briefing.
- [ ] No new or touched source comments contain ephemeral `DOS-` references.
- [ ] Typecheck, targeted tests, Rust flag tests, and real-data smoke pass.

## 2. Production acceptance gates

DOS-431 cannot land on intent alone. It lands only after these gates are
evidenced in the PR:

- **Real-data smoke test:** run against a real Google Calendar/Gmail workspace,
  navigate to `/` with no flag override, verify loading/success/refresh/empty/
  upstream-error states, verify route links, and attach screenshot or notes.
- **Dev-mode trial period:** run the redesigned route for at least one release
  candidate or agreed internal trial window after section 3 passes. Blockers
  reopen their owning section ticket.
- **Design sign-off:** designer reviews desktop and narrow widths with real
  data for hierarchy, spacing, truncation, empty branches, and absence of
  legacy card composition. Accepted visual debt must be assigned to DOS-437 or
  a new follow-up before closure.

## 3. Pre-cutover validation

Every section composer must be live-data-backed before `/` becomes canonical.
The cutover PR must include a checklist with links to passing tests or merged
PRs for each item.

| Section | Required state before DOS-431 final cutover |
|---|---|
| Lead | `compose_lead` emits real day framing from the briefing model inputs, not mock/demo copy. Focus/capacity/readiness text must come from service data or render absent. |
| Schedule | Full DOS-417 is complete: temporal classification, ISO start/end data, duration labels, day chart data, and week-shape lift are service-owned and tested. |
| Predictions | A live producer exists for today's forward-looking predictions across entities. Empty predictions are allowed only when the live query returns none, not because the producer is unwired. |
| Moving | DOS-414 aggregation and DOS-419 lifecycle adapter are complete: meeting/action/email/lifecycle signals group by entity, rank by change magnitude, carry trust/provenance where available, and degrade gracefully. |
| Watch | Full DOS-415 is complete: suggestedAction, openAction, parked, and aging rows are backed by real actions/claims and every visible affordance has a mutation path. |

- `useBriefingViewModel` must call `get_briefing_view_model` and receive the
  four-state `BriefingLoadState` envelope without frontend shaping.
- `DailyBriefingRedesign.tsx` must remain a renderer. Any new data derivation
  belongs in Rust composers or typed helpers, not the route component.
- Existing source comments in touched briefing files that mention ticket IDs
  must be rewritten as durable product/architecture comments.

## 4. Migration strategy

Preferred final state is one PR with the route gate removed and legacy deleted.
If acceptance wants a softer deployment, use this two-step release window:

1. **Transition release, optional.**
   - Flip the redesign default to true in `get_feature_flags`.
   - Keep the route gate for one release so an explicit user override can force
     the legacy branch during internal trial.
   - Mark this as temporary and create the deletion PR before DOS-431 closes.
2. **Final cutover.**
   - Replace `DailyBriefingRouteGate` with `DailyBriefingRedesign` on the index
     route.
   - Remove `daily_briefing_redesign_enabled` from Rust and TS feature-flag
     types.
   - Delete the legacy route component and unreachable legacy files.
   - Remove config examples/docs that mention the flag.

No database migration is required. Existing configs with the obsolete key are
harmless because `Config.features` is a map; after typed removal, the key is
ignored.

## 5. Files to edit

- `src/router.tsx`
  - Remove imports used only by the legacy route: `DailyBriefing`,
    `DashboardSkeleton`, `DashboardEmpty`, `DashboardError`,
    `useDashboardData`, `useWorkflow`, and `FeatureFlags` if unused.
  - Delete `DashboardPage`.
  - Delete `DailyBriefingRouteGate`.
  - Set `indexRoute.component` to `DailyBriefingRedesign`.
- `src-tauri/src/types.rs`
  - Remove `daily_briefing_redesign_enabled` from `FeatureFlags`.
  - Remove or rewrite tests that assert the old flag defaults/serializes.
  - Remove touched ticket-ID comments.
- `src/types/index.ts`
  - Remove `daily_briefing_redesign_enabled` from the TS `FeatureFlags`
    interface and update its comment.
- `src-tauri/src/commands/app_support.rs`
  - Stop populating the removed flag in `get_feature_flags`.
  - Update feature-flag docs to reflect remaining flags only.
- Config examples/docs
  - Remove `daily_briefing_redesign_enabled` from any `config.json`, template,
    proof-bundle, or setup instructions.
  - If no examples contain the flag, record the `rg` verification in the PR.

## 6. Files to delete

- `src/components/dashboard/DailyBriefing.tsx`
- `src/components/dashboard/DailyBriefing.module.css`
- `src/components/dashboard/DailyBriefing.test.tsx`, unless it still covers a
  shared helper that is extracted before deletion.
- The `DashboardPage` route component inside `src/router.tsx`.
- `src/components/dashboard/BriefingMeetingCard.tsx` and
  `BriefingMeetingCard.test.tsx` if DOS-434 has completed absorption and
  `rg "BriefingMeetingCard" src` shows no remaining production imports.
- Deletion audit: `rg "DailyBriefing|DailyBriefingRouteGate|daily_briefing_redesign_enabled|BriefingMeetingCard" src src-tauri .docs`.
  Remaining matches must be historical docs, unrelated symbols, or assigned
  follow-ups. No production code may keep the gate or deleted flag.

## 7. Out of scope

- Building missing section producers. DOS-431 blocks until those are done; it
  does not hide composer gaps.
- Redesigning `/week`, `/emails`, `/actions`, archive cards, or design tokens.
- Adding a compatibility route for the old briefing.
- Changing the locked `BriefingViewModel` contract unless L1 finds a concrete
  production blocker.

## 8. W6 sequencing

DOS-431 lands first in W6 redesign. Then DOS-435 deprecates `/week`, DOS-436
archives obsolete cards, DOS-437 trims CSS, and DOS-438 audits view purity.
DOS-438 may inspect DOS-431's final diff, but should not run before the route
cutover exists.

## 9. L1 gates

- `pnpm tsc --noEmit`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx`
- `pnpm test src/hooks/useBriefingViewModel.test.ts`
- `pnpm test src/components/dashboard/Lead.test.tsx src/components/dashboard/MovingRow.test.tsx src/components/dashboard/WatchRow.test.tsx src/components/dashboard/PredictionsSection.test.tsx`
- `cargo test --manifest-path src-tauri/Cargo.toml feature_flags`
- `cargo test --manifest-path src-tauri/Cargo.toml briefing`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins -- -D warnings`
- Static: `rg "daily_briefing_redesign_enabled|DailyBriefingRouteGate" src src-tauri`
  returns no production matches after final cutover.
- Static: `rg "DOS-" src/router.tsx src-tauri/src/types.rs src-tauri/src/commands/app_support.rs src/hooks/useBriefingViewModel.ts src/pages/DailyBriefingRedesign.tsx src-tauri/src/services/briefing`
  returns no touched source-comment violations.
- Manual real-data smoke test from section 2.

## 10. L2 gates

- Code review confirms the `/` route has no runtime feature gate and no legacy
  fallback.
- Architecture review confirms backward compatibility is intentionally closed
  and stale config keys are harmless.
- Product/design review confirms the real-data briefing is acceptable as the
  default daily surface.
- Test review confirms deleted legacy coverage is replaced by redesign route,
  composer, and mutation-path coverage.
- Release review confirms the optional transition window, if used, has been
  closed by a deletion PR before DOS-431 is marked done.
