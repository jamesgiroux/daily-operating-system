# MeetingDetail

**Tier:** surface
**Status:** redesigning (Wave 4 substrate prep)
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `MeetingDetail`
**`data-ds-spec`:** `surfaces/MeetingDetail.md`
**Canonical name:** `MeetingDetail`
**Source files:**
- Existing: `src/pages/MeetingDetail*` (verify path during implementation)
- Mockup: `.docs/mockups/claude-design-project/mockups/meeting/current/after.html`

**Design system version introduced:** 0.4.0

## Job

The post-meeting recap surface — what happened in the room, how predictions held up, what was committed, who's where in the relationship arc. Reads like an after-action review, not a transcript dump. Generated immediately after the meeting; user can act on commitments + corrections from inside.

## Layout regions

In reading order:

1. **FolioBar** — surface label "Meeting", crumbs ("Meetings / [Meeting name + date]"), center timestamp, right-side: status dot ("processed") + folio status text
2. **`FolioActions`** sub-row — Copy / Share / Send Recap (primary turmeric) / Re-extract toolbar
3. **`SurfaceMasthead`** (composed as MeetingHero per D-reconciliation):
   - accessory: `<MeetingStatusPill state="wrapped" duration="56 min" />`
   - eyebrow: "Meeting Recap · [day date · time range]"
   - title: meeting subject
   - lede: one-paragraph synthesis (the room's actual outcome, in plain prose)
4. **What Happened to Your Plan** chapter:
   - `AgendaThreadList` — predicted agenda items checked off (✓ confirmed / ○ open / + new attendee) with per-item time-spent metadata; carried-over items render with overdue affordance
5. **Predictions vs. Reality** chapter:
   - `PredictionsVsRealityGrid` — two-column risks vs wins comparison; each finding has dot + title + impact paragraph
6. **Conversation** chapter:
   - `TalkBalanceBar` (existing in `src/components/shared/`) — proportional segments by speaker
   - `SignalGrid` — 2x2 stats (Question density / Decision maker active / Forward-looking / Monologue risk)
   - `EscalationQuote` — highlighted attributed quote where the room turned
   - Competitor mentions — inline list
7. **Findings** chapter:
   - `FindingsTriad` — three-column Wins / Risks / Decisions, each with evidence quotes + attribution
8. **Champion Health** chapter:
   - `ChampionHealthBlock` — name + status arc + evidence quote + risk paragraph
9. **Commitments & Actions** chapter:
   - `CommitmentRow` instances (YOURS / THEIRS captured commitments)
   - `SuggestedActionRow` instances (AI-suggested follow-ups with Accept/Dismiss)
   - "Pending" rows for previously-committed but still-open items
10. **Role Changes** chapter:
    - `RoleTransitionRow` instances — name + before-status → after-status pill chain
11. **Finis** — `FinisMarker` + "Processed [timestamp] — from [transcript source]"

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
- `FolioActions` (Wave 4) — action toolbar sub-row
- `SurfaceMasthead` (Wave 3) — composed with MeetingStatusPill accessory; this IS the "MeetingHero"
- `AgendaThreadList`, `PredictionsVsRealityGrid`, `SignalGrid`, `EscalationQuote`, `FindingsTriad`, `ChampionHealthBlock`, `CommitmentRow`, `SuggestedActionRow`, `RoleTransitionRow` (all Wave 4)
- `TalkBalanceBar` (existing in `src/components/shared/`; not re-spec'd)
- `FinisMarker` (existing canonical from Wave 1)

## Primitives consumed

- `MeetingStatusPill` (Wave 4)
- `Pill` (commitment YOURS/THEIRS tags, finding dots)
- `EntityChip` (attendee references, account references)
- `TrustBandBadge`, `FreshnessIndicator`, `ProvenanceTag` (claim-level signals on findings)
- `Button` (Send Recap primary, Re-extract icon, Accept / Dismiss / Mark complete in actions)

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
- Wave 4 entry count clarification: original synthesis listed `MeetingHero` as a separate Wave 4 pattern; reconciled into `SurfaceMasthead` per D-series reconciliation. Wave 4 entry count: 13 → 12 (MeetingHero subsumed).
