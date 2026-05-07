# DOS-437 - CSS trim after redesign cutover - L0 plan

**Status:** L0 draft  
**Scope:** Remove orphaned CSS from the legacy editorial briefing module and dashboard CSS modules after the redesigned briefing is canonical.

## 1. Ticket reference and acceptance summary

DOS-437 trims the orphaned CSS left behind by the daily briefing redesign cutover.

Acceptance criteria:

- [ ] `src/styles/editorial-briefing.module.css` is at or below 400 total lines.
- [ ] Every class remaining in `src/styles/editorial-briefing.module.css` has a live production consumer or a synchronized reference HTML consumer that still mirrors production.
- [ ] Every `.module.css` file under `src/components/dashboard/` has zero orphaned classes after the cutover.
- [ ] Any orphan-only dashboard CSS module is deleted with its import removed.
- [ ] `.docs/design/reference/surfaces/` HTML and mirrored `_shared/styles/*.module.css` files stay synchronized with the source class names.
- [ ] No source-code comments contain ticket IDs.

## 2. Current code facts

- `src/styles/editorial-briefing.module.css` is currently 1,490 lines.
- Current production imports include `DailyBriefing.tsx`, `BriefingMeetingCard.tsx`, and `DashboardSkeleton.tsx`.
- `src/components/dashboard/` currently has component-scoped CSS modules for state stubs, DayStrip, DayChart, Lead, MovingRow, WatchRow, PredictionsSection, SignalDot, ProvenanceStat, MeetingSpineItem, InferredActionSelector, EntityPortraitCard, AskAnythingDock, and DailyBriefing local extraction styles.
- DOS-431 plan is present under `.docs/plans/wave-W6-redesign/DOS-431-plan.md`; it makes `DailyBriefingRedesign` canonical, deletes legacy `DailyBriefing`, and deletes `BriefingMeetingCard` if DOS-434 has fully absorbed prep details.
- DOS-434 plan is present under `.docs/plans/wave-W4-redesign/DOS-434-plan.md`; it removes `BriefingMeetingCard` inline expansion ownership and moves prep-detail content to `MeetingDetailPage`.
- DOS-436 plan is not present in this checkout yet. Implementation must read its landed plan or diff before deleting dashboard component CSS modules.

## 3. What I'm building

Files modified:

- `src/styles/editorial-briefing.module.css`
- any orphan-only `src/components/dashboard/*.module.css`
- affected dashboard TSX imports only where required by deleted orphan-only CSS modules
- `.docs/design/reference/surfaces/*.html` that still mention removed classes
- `.docs/design/reference/_shared/styles/*.module.css` mirrors for any source CSS changed

Expected CSS families to audit for deletion after predecessors land:

- `BriefingMeetingCard` expansion families: `theRoom*`, `prepGrid*`, `meetingActions*`, `capturedSummary*`, expansion-panel hints/state classes.
- Legacy action/email/watch families: `priorityItem*`, `priorityCheck*`, `replyMeta`, `emailScoreReason`, `agingNotice`, lifecycle-group support.
- Deprecated lead/schedule residue: `heroNarrative`, `staleness`, `prepFlag`, old `scheduleRow*` classes no longer consumed by the canonical surface.
- Any dashboard component CSS module whose TSX component was archived by DOS-436.

The final retained `editorial-briefing.module.css` should be only shared editorial scaffolding still used by the active briefing surface or skeleton: margin grid, labels, section rules, and any proven live transitional layout rules.

## 4. What I'm NOT building

- No routed cutover work.
- No MeetingDetailPage absorption work.
- No card/archive component deletion beyond CSS modules proven orphan-only.
- No new visual design, new component abstractions, or class renames for style preference.
- No new inline CSS.
- No source comments that mention ticket IDs.

## 5. Dependencies and ordering

Hard dependency: DOS-431, DOS-434, and DOS-436 must land first.

- DOS-431 establishes the canonical routed briefing surface and removes the legacy route/component branch. Before it lands, old `/` consumers make many classes appear live.
- DOS-434 moves inline meeting expansion detail out of `BriefingMeetingCard`; until it lands, expansion CSS is still consumed.
- DOS-436 archives old cards/components; until it lands, dashboard CSS modules may still have legitimate consumers.

Implementation must start from the merged state of those three predecessors, not from this draft's current-code inventory. If any predecessor changes scope, this plan narrows or expands only to remove classes made orphaned by the merged code.

## 6. Reuse audit

Reuse before deleting or moving:

- Keep existing tokens from `src/styles/design-tokens.css`; do not introduce replacement token aliases.
- Keep canonical W1/W3 dashboard modules (`Lead`, `MeetingSpineItem`, `MovingRow`, `WatchRow`, `PredictionsSection`, `SignalDot`, `ProvenanceStat`, `DayStrip`, `DayChart`) when they have live TSX consumers.
- Prefer existing component-owned modules over retaining broad `editorial-briefing.module.css` rules.
- Treat `.docs/design/reference/_shared/styles/*.module.css` as mirrors, not independent source CSS.

## 7. Audit method

Use three passes before deletion.

Git grep pass:

- `wc -l src/styles/editorial-briefing.module.css`
- `find src/components/dashboard -name '*.module.css' -print | sort`
- `git grep -n "editorial-briefing.module.css\\|@/styles/editorial-briefing.module.css" src .docs/design/reference`
- For each candidate class, run a targeted `git grep` for `styles.<class>`, `s.<class>`, bracket access, and scoped reference names under `src` and `.docs/design/reference/surfaces`.
- Explicitly check dynamic maps and `clsx` call sites where classes are assembled indirectly.

Automated unused-class detection:

- Run an implementation-time script against `src/styles/editorial-briefing.module.css` and every `src/components/dashboard/*.module.css`.
- The script must parse CSS class selectors, parse TS/TSX CSS-module imports, and count references through dot access, bracket access, lookup maps, and `clsx`.
- The script must report three buckets: live class, source-orphan/reference-live, and full orphan.
- Full-orphan is deletable. Source-orphan/reference-live requires reference HTML sync before deletion.

Manual review:

- Review each candidate deletion in the owning TSX file, colocated tests, test mocks, and `.docs/design/reference/surfaces/`.
- Check non-obvious surfaces such as `DashboardSkeleton`, legacy `/briefing` references if still present, and fixture/reference HTML that uses scoped class names directly.
- For any ambiguous class, keep it until a visual or snapshot check proves it is not needed.

## 8. Display-layer purity

This is CSS cleanup only. No data filtering, sorting, view-model shaping, or runtime display logic should be introduced. TSX edits are allowed only to remove imports for deleted orphan-only modules or to remove dead `className` references that no longer render.

## 9. Test plan

Static gates:

- `pnpm tsc --noEmit`
- `pnpm test` or the narrowed dashboard test set if the full suite is too slow for the PR
- `python3 .docs/design/_audits/audit-reference.py --surface briefing-redesign --strict`
- run additional `audit-reference.py --surface ... --strict` for any reference surface touched
- from `.docs/design/reference/_shared/`, run `python3 scope-modules.py` after source mirror changes

Verification for the main risk:

- Add or update a targeted snapshot/unit test if a previously styled branch still renders after cleanup.
- Run a visual diff or manual browser screenshot comparison for the canonical briefing, empty/error/loading states, and any touched actions/emails/meeting reference surface.

## 10. Risk and rollback

Primary risk: deleting a class still consumed by a non-obvious surface, test fixture, or reference HTML file. This is most likely for dynamic `clsx` combinations, lookup maps, scoped reference classes, and old compatibility surfaces.

Mitigation:

- Require the automated unused-class report before deletion.
- Require manual review of dynamic class assembly and reference HTML.
- Require snapshot or visual-diff evidence for any branch whose visual state depends on removed classes.

Rollback is mechanical: restore the removed CSS block and any removed import/class reference from the previous commit. Because this ticket should not change data flow or component APIs, rollback should not require service or route changes.

## 11. Wave dependencies

Consumes:

- DOS-431 canonical briefing cutover.
- DOS-434 MeetingDetailPage absorption.
- DOS-436 archive-card cleanup.

Feeds:

- DOS-438 view-purity audit, which should run after this trim so it audits the final surface and not legacy CSS residue.
- W6 merge gate requiring `editorial-briefing.module.css <= 400` lines.

## 12. L1 gates

The implementing agent must attach evidence for:

- `wc -l src/styles/editorial-briefing.module.css` showing `<= 400`.
- Automated unused-class detection report showing zero full-orphan classes in `src/styles/editorial-briefing.module.css` and `src/components/dashboard/*.module.css`.
- `pnpm tsc --noEmit` clean.
- Relevant `pnpm test ...` command output clean.
- `audit-reference.py --strict` clean for every touched reference surface.
- Visual diff, snapshot test, or manually captured before/after notes for the canonical briefing surface.
- `git grep -n "DOS-" <touched source files>` plus diff review showing no newly added source comments with ticket IDs.

## 13. L2 gates

- Code review confirms every deleted class was proven orphaned by grep, automation, and manual review.
- Design review confirms canonical briefing and touched reference surfaces did not visually regress.
- Test review confirms the deletion risk is covered by snapshot or visual-diff evidence, not just TypeScript compilation.
- Reference review confirms `.docs/design/reference/surfaces/` and `_shared/styles/` are synchronized with source modules.
