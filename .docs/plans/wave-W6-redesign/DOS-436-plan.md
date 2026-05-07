# DOS-436 - Archive orphaned card patterns (W6 redesign)

**Status:** L0 draft  
**Scope:** Archive or delete the card-style Daily Briefing components left behind after the redesigned briefing becomes canonical.

## 1. Acceptance criteria

- [ ] DOS-431 has landed: routed `/` renders `DailyBriefingRedesign` by default, with no production fallback to old `DailyBriefing`.
- [ ] DOS-434 has landed: meeting prep detail content is owned by `MeetingDetailPage`, not by an inline briefing expansion.
- [ ] Orphan inventory is re-run at L1 with `rg` and recorded in the implementation notes.
- [ ] Every orphaned card-style source component is either deleted, moved to archive, or explicitly kept as a legacy reference with a live consumer.
- [ ] Components re-homed by DOS-434 are not deleted from their new MeetingDetailPage implementation.
- [ ] Tests, reference-system entries, and any Storybook stories for removed cards are deleted or archived.
- [ ] DOS-437 receives a precise CSS class-family handoff for remaining `editorial-briefing.module.css` cleanup.
- [ ] `pnpm tsc --noEmit` and targeted tests pass.
- [ ] No source-code comments added or retained in touched files contain `DOS-` ticket references.

## 2. Sequencing

Order is hard:

1. DOS-434 must complete first so prep detail is classified as re-homed, not orphaned.
2. DOS-431 must complete second so old `DailyBriefing` is no longer the production route.
3. DOS-435 should complete before archiving `MeetingCard`; otherwise `/week` remains a live consumer.
4. DOS-436 removes/archive source and docs entries.
5. DOS-437 trims the large shared CSS file using this ticket's handoff list.

If L1 finds a live routed consumer for an item, that item moves from "orphan" to "legacy reference" and must not be deleted.

## 3. Inventory and decisions

| Item | Current source | Current consumer | Post-cutover status | Decision |
|---|---|---|---|---|
| Old Daily Briefing surface | `src/components/dashboard/DailyBriefing.tsx` | `src/router.tsx` via `DailyBriefingRouteGate` until DOS-431 | Orphan if no fallback remains | Delete source after route import is gone; keep historical reference only in docs/archive. |
| `BriefingMeetingCard` row shell | `src/components/dashboard/BriefingMeetingCard.tsx` | Old `DailyBriefing` schedule | Orphan after DOS-431 | Archive/delete source; do not keep runtime export. |
| Inline expansion panel | `BriefingMeetingCard.tsx` (`isExpanded`, measured height, bridge/collapse) | Old schedule rows | Orphan after DOS-434 | Delete; MeetingDetailPage owns depth. |
| `KeyPeopleFlow` | `BriefingMeetingCard.tsx` | Expansion panel | Re-homed by DOS-434 if route uses `UnifiedAttendeeList`/route-native room | Do not archive the concept; delete old helper once no imports remain. |
| `PrepGrid` + `parsePrepGridItem` | `BriefingMeetingCard.tsx` | Expansion panel and current tests | Re-homed by DOS-434 into meeting route helpers or route-native sections | Move helper tests to meeting helper tests if still used; delete dashboard export. |
| `MeetingActionChecklist` | `BriefingMeetingCard.tsx` | Expansion panel | Re-homed by DOS-434 into MeetingDetailPage action section | Delete dashboard helper after action detail coverage exists. |
| `MeetingCard` | `src/components/shared/MeetingCard.tsx` + module CSS | `BriefingMeetingCard`, `WeekPage` | Orphan only after DOS-435 removes `/week` | Archive/delete if `rg "<MeetingCard|from .*MeetingCard"` finds no live consumers; otherwise keep as legacy reference until `/week` is gone. |
| `DailyBriefingAttentionSection` local pattern | `AttentionSection` inside `DailyBriefing.tsx` | Old `DailyBriefing` | Orphan after DOS-431 | Delete with old surface; archive spec/reference entry. |
| Lifecycle confirm/correct card rows | `LifecycleUpdateItem` in `DailyBriefing.tsx` + `DailyBriefing.module.css` | Old Attention section | Orphan; lifecycle is Moving signal now | Delete old row/modal glue unless another live surface imports it. |
| Priority action card rows | raw action JSX + `PrioritizedActionItem` in `DailyBriefing.tsx` | Old Attention section | Orphan; Watch and `/actions` own action triage | Delete. |
| Priority email card rows | `PriorityEmailItem` in `DailyBriefing.tsx` | Old Attention section | Orphan; email signals move to Moving and `/emails` owns raw inbox | Delete local row. |
| Aging notice row | `DailyBriefing.tsx` + `DailyBriefing.module.css` | Old Attention section | Orphan; Watch has aging variant | Delete local row; CSS class goes to DOS-437 cleanup. |
| `EmailEntityChip` component | `src/components/ui/email-entity-chip.tsx` | `DailyBriefing`, `EmailsPage` | Not an orphan if `/emails` still consumes it after DOS-432 | Keep as legacy reference when `EmailsPage` still imports it; archive only if DOS-432 replaced it with `EntityChip`. |
| `EntityPortraitCard` | `src/components/dashboard/EntityPortraitCard.tsx` + module CSS | No routed consumer; spec already archived | Source-only orphan from abandoned D-spine card direction | Move to archive or delete runtime source; remove reference-system demo if still listed. |
| `DayChart` | `src/components/dashboard/DayChart.tsx` | Redesign reference/system; not card-style | Not in DOS-436 scope | Keep unless a separate design decision archives schedule chart. |
| `AskAnythingDock` / `ThreadMark` | dashboard/ui prototype components | No current routed briefing consumer | Not card-cleanup scope unless referenced only by `EntityPortraitCard` | Delete only if orphaned solely through `EntityPortraitCard`; otherwise leave for separate prototype cleanup. |

## 4. Files affected

Runtime candidates:

- `src/router.tsx`
- `src/components/dashboard/DailyBriefing.tsx`
- `src/components/dashboard/DailyBriefing.test.tsx`
- `src/components/dashboard/DailyBriefing.module.css`
- `src/components/dashboard/BriefingMeetingCard.tsx`
- `src/components/dashboard/BriefingMeetingCard.test.tsx`
- `src/components/dashboard/EntityPortraitCard.tsx`
- `src/components/dashboard/EntityPortraitCard.module.css`
- `src/components/shared/MeetingCard.tsx`
- `src/components/shared/MeetingCard.module.css`
- `src/components/ui/email-entity-chip.tsx` only if `/emails` no longer imports it
- `src/lib/email-ranking.test.ts` if it still exists only to mirror old DailyBriefing selection

Design/docs/reference candidates:

- `.docs/design/patterns/BriefingMeetingCard.md`
- `.docs/design/patterns/DailyBriefingAttentionSection.md`
- `.docs/design/patterns/MeetingCard.md`
- `.docs/design/patterns/README.md`
- `.docs/design/reference/system/patterns.html`
- `.docs/design/reference/surfaces/briefing.html`
- `.docs/design/reference/_shared/styles/MeetingCard.module.css`
- `.docs/design/reference/_shared/styles/EntityPortraitCard.module.css`
- `.docs/design/_audits/surface-manifest.json`
- `.docs/design/INVENTORY.md`

Archive target:

- Prefer `.docs/design/_archive/patterns/` for superseded specs.
- If source snapshots are required, use `_archive/source/daily-briefing-cards/`; otherwise rely on Git history and delete runtime code.

## 5. Tests and Storybook cleanup

Delete or rewrite:

- `src/components/dashboard/DailyBriefing.test.tsx` if old `DailyBriefing` is removed.
- `src/components/dashboard/BriefingMeetingCard.test.tsx` after prep helper coverage has moved to MeetingDetailPage or meeting helper tests.
- Any MeetingCard assertions tied only to `/week` after DOS-435.
- `src/lib/email-ranking.test.ts` if no source imports `compareEmailRank` and DOS-416 service parity owns ranking.

Keep or add:

- `src/pages/DailyBriefingRedesign.test.tsx` coverage proving the canonical route renders the new surface.
- MeetingDetailPage tests from DOS-434 proving context, room, discuss/watch/wins, and before-meeting actions render.
- WatchRow and MovingRow tests proving lifecycle/action/email/aging replacements still cover the old jobs.

Storybook status:

- Current repo scan shows no `.stories.*` or Storybook config.
- L1 must re-run `rg --files | rg '(stories|storybook)'`.
- If stories have been added, remove/archive stories for `BriefingMeetingCard`, `MeetingCard`, `DailyBriefingAttentionSection`, `EntityPortraitCard`, and old `DailyBriefing`.

## 6. CSS cleanup coordination with DOS-437

DOS-436 removes imports and component-owned module files when their components go away. DOS-437 owns broad trimming of `src/styles/editorial-briefing.module.css`.

Handoff class families for DOS-437:

- Schedule/old card: `briefingCardOverride`, `scheduleRow*`, `expansionPanel*`, `expansionInner`, `expandHint`
- Prep detail: `theRoom*`, `prepGrid`, `prepSection`, `prepLabel*`, `prepItem*`, `prepDot*`, `trustBackground*`, `trustShowAll*`, `meetingActions*`, `meetingLinks`, `capturedSummary*`
- Attention rows: `prioritiesSection`, `priorityGroupLabel*`, `priorityItems`, `priorityItem*`, `priorityCheck*`, `priorityDot*`, `priorityContent`, `priorityTitle`, `priorityContext`, `priorityWhy`, `replyMeta`, `emailScoreReason`
- Old schedule flags: `prepFlag`, `healthBadgeRow`
- Old notice/modal helpers in `DailyBriefing.module.css`: lifecycle/correction/callout/email/aging helpers

Do not delete `editorial-briefing.module.css` wholesale in DOS-436: `EmailsPage` and `DashboardSkeleton` currently import it, and DOS-432/DOS-437 decide the remaining shared use.

## 7. Out of scope

- Reworking `/emails` or `/actions`; DOS-432 and DOS-433 own those surfaces.
- Trimming every unused CSS selector from `editorial-briefing.module.css`; DOS-437 owns that.
- Deleting MeetingDetailPage prep sections absorbed by DOS-434.
- Changing briefing service contracts or `BriefingViewModel`.
- Replacing WatchRow, MovingRow, MeetingSpineItem, DayStrip, Lead, or PredictionsSection.

## 8. L1 self-validation gates

- `rg -n "DailyBriefing|BriefingMeetingCard|MeetingCard|EntityPortraitCard|DailyBriefingAttentionSection|EmailEntityChip" src .docs/design --glob '!**/_archive/**'`
- `rg -n "from \\\"@/components/shared/MeetingCard\\\"|<MeetingCard|BriefingMeetingCard" src`
- `rg -n "from \\\"@/components/ui/email-entity-chip\\\"|EmailEntityChip" src`
- `rg --files | rg '(stories|storybook)'`
- `pnpm tsc --noEmit`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx`
- `pnpm test src/components/dashboard/WatchRow.test.tsx src/components/dashboard/MovingRow.test.tsx`
- Run MeetingDetailPage targeted tests added by DOS-434.
- `rg -n "DOS-"` on every touched source file returns no matches.

## 9. L2 review gates

- Code review confirms no production route imports the archived cards.
- Design review confirms the design-system index no longer presents removed card-heavy patterns as integrated.
- Regression review confirms MeetingDetailPage owns former prep depth and Watch/Moving own old attention jobs.
- CSS review confirms DOS-437 received the exact leftover selector list and DOS-436 did not leave dead component imports.
- Archive review confirms historical docs moved to `_archive` with replacement pointers, while runtime source is removed unless an approved legacy consumer remains.
