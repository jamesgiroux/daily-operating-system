# MeetingDetail

**Tier:** surface
**Status:** shipped surface + extraction targets
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `MeetingDetail`
**`data-ds-spec`:** `surfaces/MeetingDetail.md`
**Canonical name:** `MeetingDetail`
**Source files:**
- Existing: `src/pages/MeetingDetailPage.tsx` (canonical route)
- Legacy route wrapper: `src/pages/MeetingHistoryDetailPage.tsx` (covered by `MeetingDetailPage`)
- CSS module: `src/pages/meeting-intel.module.css`
- Mockup: `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html`

**Design system version introduced:** 0.4.0

## Job

The post-meeting recap surface — what happened in the room, how predictions held up, what was committed, who's where in the relationship arc. Reads like an after-action review, not a transcript dump. Generated immediately after the meeting; user can act on commitments + corrections from inside.

## Layout regions

In reading order:

1. **FolioBar / magazine shell actions** — meeting label, crumbs, refresh action, and status text.
2. **Inline folio actions** — source-local action buttons in `MeetingDetailPage.tsx`; no exported `FolioActions` component yet.
3. **Meeting hero** — current source uses `MeetingDetailPage.module.css` hero classes rather than `SurfaceMasthead`.
4. **Editable meeting intelligence** — `EditableText`, `IntelligenceFeedback`, entity chips, health badges, and refresh controls.
5. **Post-meeting intelligence** — `PostMeetingIntelligence` renders the real agenda threads, predictions, conversation signals, escalation quote, findings, champion health, commitments, action rows, and role transitions.
6. **Outcomes/actions** — `ActionRow` outcome variant.
7. **Finis** — `FinisMarker`.

`AtmosphereLayer` (turmeric default; may inherit primary entity tint).

## Local nav approach

**Provides chapters to `FloatingNavIsland`** per D2. Seven significant sections warrant chapter nav (synthesis Part 4 noted MeetingDetail might be "no chapters" for short surfaces — this surface is long enough that chapters help):

- `plan` → "What Happened"
- `predictions` → "Predictions"
- `conversation` → "Conversation"
- `findings` → "Findings"
- `champion` → "Champion"
- `commitments` → "Commitments"
- `roles` → "Role Changes"

Local pill renders these via FloatingNavIsland's chapters contract; scroll-spy highlights active.

**`FolioActions` is separate from FloatingNavIsland** — it's the action toolbar (Copy / Share / Send Recap / Re-extract), not navigation. Sub-row below FolioBar; coexists with FloatingNavIsland.

## Patterns consumed

- `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer` (chrome)
- `PostMeetingIntelligence`
- `TalkBalanceBar`
- `ActionRow`
- `IntelligenceFeedback`
- `FinisMarker`

Shipped as local `PostMeetingIntelligence` class families, not exported components:

- `AgendaThreadList`
- `PredictionsVsRealityGrid`
- `SignalGrid`
- `EscalationQuote`
- `FindingsTriad`
- `ChampionHealthBlock`
- `CommitmentRow`
- `RoleTransitionRow`

Proposed:

- `FolioActions` as an extracted toolbar pattern.
- `SurfaceMasthead` replacement for the current inline hero.

## Primitives consumed

- `Pill` (commitment YOURS/THEIRS tags, finding dots)
- `EntityChip` (attendee references, account references)
- `HealthBadge`
- `EditableText`
- `FolioRefreshButton`

## Notable interactions

- **Send Recap** (primary FolioAction, turmeric) — composes a recap email/Slack message and sends to attendees
- **Re-extract** — reruns intelligence extraction on the transcript; shows progress; updates findings inline
- **Accept / Dismiss on suggested actions** — promotes to commitment OR records as user feedback (dismissal is signal, not a void)
- **Mark complete on pending commitments** — settles open commitments inline
- **Click any finding's evidence quote** — opens transcript at that timestamp (deep link)

## Empty / loading / error states

- **Loading** (recap generating) — `GeneratingProgress` with phase steps ("Transcribing" → "Extracting" → "Analyzing" → "Generating recap")
- **Error** — `EditorialError` with re-extract action
- **No transcript** — empty state with manual entry option
- **Partial extraction** (some sections couldn't be derived) — section-level placeholders with explanation

## Naming notes

Canonical name `MeetingDetail`. Verify current src naming during implementation; if `MeetingDetailPage.tsx` or similar, rename to `MeetingDetail` to match (NAMING.md track).

The mockup uses `cur-pm-*` and `cur-folio-*` prefixed classes (current/post-meeting) — these are mockup-internal naming and do **not** propagate to canonical patterns; per synthesis Part 6, all `cur-*` classes consolidate into the named patterns above.

## History

- 2026-05-03 — Surface spec authored as part of Wave 4 (MeetingDetail redesign substrate prep).
- 2026-05-05 — Corrected spec to shipped source. Wave 4 child patterns are real shipped UI where they are local `PostMeetingIntelligence` class families; `FolioActions` and `SurfaceMasthead` remain extraction targets.
