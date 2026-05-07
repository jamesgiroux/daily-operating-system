# DOS-434 — MeetingDetailPage absorption (W4 redesign)

**Status:** L0 draft
**Scope:** Move prep-detail content out of `BriefingMeetingCard` inline expansion and into the routed `MeetingDetailPage`.

## Acceptance Criteria

- [ ] Redesigned schedule meeting clicks open the meeting detail route and do not expand inline.
- [ ] `MeetingDetailPage` renders the prep-detail content that currently lives inside `BriefingMeetingCard`'s expansion panel.
- [ ] Absorbed content appears as real detail-page sections, not as a pasted expansion panel.
- [ ] The page covers meeting context, The Room, Discuss, Watch, Wins, before-meeting actions, and no-prep empty copy.
- [ ] `BriefingMeetingCard` remains exported for old `/briefing`, but becomes a plain row with no local expansion state.
- [ ] `BriefingMeetingCard.tsx` no longer owns `PrepGrid` or `MeetingActionChecklist`.
- [ ] Shared prep parsing/render helpers are extracted before reuse.
- [ ] No inline CSS is introduced.
- [ ] No ephemeral ticket references are added to source comments.
- [ ] Route verification is explicit: current code registers `path: "/meeting/$meetingId"` in `src/router.tsx`; if `/meetings/{id}` is intended literally, L1 must require an alias or route correction.

## Current Code Facts

- `BriefingMeetingCard.tsx` currently owns row rendering plus expansion details: `isExpanded`, measured `maxHeight`, `KeyPeopleFlow`, `PrepGrid`, `MeetingActionChecklist`, empty copy, bridge link, and collapse button.
- `MeetingDetailPage.tsx` already loads `MeetingIntelligence` through `get_meeting_intelligence`, stores `intel.prep` as `FullMeetingPrep`, and renders hero, risks, recent wins, open items, The Room, plan, outcomes, and transcript actions.
- `MeetingDetailPage.tsx` already has trust evidence handling through `buildMeetingTrustPartition`, `trustItemsForSection`, and `MeetingTrustBackground`.
- The redesign contract already carries `ScheduleMeeting.href` and `briefingAction: { kind: "link"; href }`; tests currently use `/meeting/meeting-1`.

## Absorption Mapping

| Expansion content | Current source | MeetingDetailPage target | Implementation note |
|---|---|---|---|
| Calendar description | `stripHtml(meeting.calendarDescription)` | New visible context block near `#headline` | Do not hide the only copy inside `UnifiedPlanEditor` details. |
| AI context fallback | `meeting.prep?.context` | `data.meetingContext` after key insight | Keep key insight, but expose fuller context when available. |
| The Room | `KeyPeopleFlow` | Existing `#the-room` + `UnifiedAttendeeList` | Prefer richer route attendee data; fallback only if route has raw attendees. |
| Discuss | `PrepGrid` from `prep.actions` + `prep.questions` | `#your-plan` plus explicit talking-points/questions rows if needed | Do not assume `proposedAgenda` contains every Discuss item. |
| Watch | `PrepGrid` from `prep.risks` | Existing `#risks` | Include both `entityRisks` and `risks`. |
| Wins | `PrepGrid` from `prep.wins` | Existing Recent Wins section | Verify legacy `wins` maps to `recentWins`; add normalizer only if needed. |
| Trust background/show-all | `PrepGrid` trust controls | Existing `MeetingTrustBackground` | Use page trust styling, not dashboard expansion classes. |
| Before this meeting | `MeetingActionChecklist(meetingActions)` | New/verified action section from `MeetingIntelligence.actions` | Filter to pending current-meeting actions; reuse `ActionRow` if suitable. |
| No prep | `No prep available yet.` | Existing empty/not-ready state | Make the route copy explicit for meeting-without-prep. |
| Full briefing link | `Read full briefing ->` | Removed | Row navigation replaces this. |
| Collapse | `Collapse` button | Removed | Detail page has no expansion state. |

## Extraction Plan

Do not import `PrepGrid`, `KeyPeopleFlow`, or `MeetingActionChecklist` from `BriefingMeetingCard.tsx` into `MeetingDetailPage.tsx`.

Preferred extraction:

- `src/components/meeting/meeting-prep-utils.ts`
- `src/components/meeting/meeting-prep-utils.test.ts`
- Optional visual component only if needed: `src/components/meeting/MeetingPrepSections.tsx` + module CSS.

Move or recreate the reusable pieces outside the dashboard row:

- `parsePrepGridItem`
- impact tail parsing for `high | medium | low`
- duplicate Discuss/Wins normalization
- field-path aware prep evidence construction
- trust-band partition preparation where it is not already route-native

`MeetingDetailPage` should render with its existing page primitives where possible: `ChapterHeading`, `ClaimTextRenderer`, `TrustBandIndicator`, `MeetingTrustBackground`, `UnifiedAttendeeList`, `UnifiedPlanEditor`, and `ActionRow`.

## File Plan

### `src/pages/MeetingDetailPage.tsx`

- Add visible pre-meeting context using `stripHtml(data.calendarNotes)` first, then `data.meetingContext`.
- Avoid duplicate visible text when the key insight is the first sentence of the same context.
- Ensure Discuss content includes `data.talkingPoints`, `data.questions`, and `data.proposedAgenda`.
- Ensure Watch content includes `data.entityRisks` and `data.risks`.
- Ensure Wins content includes `data.recentWins`, with compatibility handling only if legacy prep still arrives as `wins`.
- Add or verify a before-meeting actions section from `intel.actions`, filtered to this meeting's pending actions.
- Preserve completion/reopen mutations and page refresh behavior for action rows.
- Update empty states so a meeting with no prep renders clear no-prep copy.
- Verify direct render at `/meeting/$meetingId`.

### `src/components/dashboard/BriefingMeetingCard.tsx`

- Keep the exported component and temporal helpers.
- Remove `isExpanded`, `setIsExpanded`, `innerRef`, `measuredHeight`, `useLayoutEffect`, and max-height panel logic.
- Remove expansion-only calculations: `hasPrepContent`, `canExpand`, `prepDiscuss`, `prepWatch`, `prepWins`, `getExpansionTintClass`.
- Remove inline expansion JSX, bridge link, and collapse button.
- Remove title "expand/collapse" hint.
- Make non-cancelled rows navigate to detail; cancelled rows stay inert.
- Keep row display: time, duration, title, entity byline, attendee count, intelligence quality, current/up-next/past/cancelled styling.
- Keep old caller compatibility by either leaving now-unused props optional for one release or updating `DailyBriefing.tsx` in the same implementation.

### Optional New Files

- `src/components/meeting/meeting-prep-utils.ts`
- `src/components/meeting/meeting-prep-utils.test.ts`
- `src/components/meeting/MeetingPrepSections.tsx`
- `src/components/meeting/MeetingPrepSections.module.css`
- `src/components/meeting/MeetingPrepSections.test.tsx`

Only create visual files if the route needs reusable section components. Prefer route-native rendering if fewer abstractions are needed.

## Backward Compatibility

`BriefingMeetingCard` still ships for the old `/briefing` DailyBriefing. Do not delete it, rename it, or break its import. The legacy route may lose inline prep depth after this ticket, but it must still render rows and navigate to the routed briefing.

If old `/briefing` still passes `meetingActions`, `onComplete`, `completedIds`, `isUpNext`, or `userDomain`, either keep them as optional unused props or remove the call-site props in the same implementation. Avoid a broad legacy route refactor.

## Test Impact

Keep or move:

- `parsePrepGridItem` impact-tail parsing.
- duplicate Discuss/Wins normalization.
- trust partition behavior for use-with-caution and needs-verification evidence.

Remove or rewrite:

- any `BriefingMeetingCard` expand/collapse assertions.
- any assertions for measured height, `Collapse`, `Read full briefing`, `PrepGrid` inside the card, or `MeetingActionChecklist` inside the card.

Add/update:

- `MeetingDetailPage` test with mocked `get_meeting_intelligence` payload asserting context, room, discuss/watch/wins, and before-meeting actions render.
- `BriefingMeetingCard` row test asserting non-cancelled click navigates to detail and cancelled row does not.
- `DailyBriefingRedesign` test asserting meeting link/href targets the canonical meeting route and has no inline expansion behavior.

## Out Of Scope

- Redesign surface composition; DOS-429 owns that.
- Navigation wiring beyond verifying the existing `href` / `briefingAction.kind="link"` contract.
- Backend service redesign.
- Canonical route rename unless L1 confirms plural `/meetings/{id}` is required.
- Broad `MeetingDetailPage` decomposition.
- New design-system primitives.

## Risks

- **Route spelling:** shipped route is singular `/meeting/$meetingId`, while ticket prose says `/meetings/{id}`.
- **Duplicate context:** route already has key insight and calendar notes; absorption must not show the same paragraph twice.
- **Action source:** old card receives filtered `meetingActions`; route gets `MeetingIntelligence.actions`.
- **CSS drift:** dashboard expansion styles live in `editorial-briefing.module.css`; route styles live in `meeting-intel.module.css`.
- **Legacy route blast radius:** old `/briefing` still imports `BriefingMeetingCard`.

## L1 Gates

- `pnpm tsc --noEmit`
- `pnpm test src/components/dashboard/BriefingMeetingCard.test.tsx`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx`
- Run extracted helper/component tests if files are created.
- Run or add `MeetingDetailPage` targeted tests for absorbed sections.
- Manual browser check:
  - open redesigned briefing
  - click an upcoming meeting
  - confirm canonical meeting detail URL
  - confirm context, room, discuss/watch/wins, and before-meeting actions render
  - confirm old `/briefing` still renders rows
- Static checks:
  - `rg "style=\\{\\{" src/pages/MeetingDetailPage.tsx src/components/dashboard/BriefingMeetingCard.tsx`
  - `rg "DOS-" src/pages/MeetingDetailPage.tsx src/components/dashboard/BriefingMeetingCard.tsx`

## L2 Gates

- Code review: `BriefingMeetingCard` is row-only, extracted helpers are route-safe, and `MeetingDetailPage` owns detail composition.
- Design review: absorbed content reads as page sections, with no pasted expansion panel and no duplicate context.
- Test review: removed expansion coverage is replaced by route/detail coverage, legacy `/briefing` remains safe, and canonical route behavior is covered.
