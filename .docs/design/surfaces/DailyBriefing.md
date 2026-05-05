# DailyBriefing

**Tier:** surface
**Status:** shipped surface + prototype roadmap separated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `DailyBriefing`
**`data-ds-spec`:** `surfaces/DailyBriefing.md`
**Canonical name:** `DailyBriefing`
**Current src name:** `DashboardPage` inline in `src/router.tsx` ← rename candidate (DOS-360, deferred to post-v1.4.3 per D7)
**Source files:**
- `src/router.tsx` (`DashboardPage` inline route component, to be renamed/extracted)
- `src/components/dashboard/DailyBriefing.tsx`
- `src/components/dashboard/DailyBriefing.test.tsx`
- `src/hooks/useDashboardData.ts`
- `src/styles/editorial-briefing.module.css`
- `src/components/onboarding/chapters/DashboardTour.tsx`
- Spine-D references remain roadmap material until adopted by routed UI.

**Design system version introduced:** 0.1.0

## Job

The morning chief-of-staff briefing — a single editorial page that tells the user what matters today, in one sentence at the top, then in narrative form below. Not a dashboard of metrics; not a calendar; an article you read.

## Layout regions

In the shipped source, reading order is:

1. **FolioBar / magazine shell actions** — Daily Briefing label, refresh action via `FolioRefreshButton`, and live/stale status copy.
2. **Surface-local hero** — lead-like editorial copy implemented with `editorial-briefing` classes, not the standalone `Lead` pattern.
3. **Today / schedule** — `MarginGrid` section containing `BriefingMeetingCard` rows. `BriefingMeetingCard` composes `MeetingCard`, `KeyPeopleFlow`, `PrepGrid`, and `MeetingActionChecklist`.
4. **Health / account context where available** — `HealthBadge` and entity chips in shipped briefing rows.
5. **Attention section** — shipped local rows for prioritized actions, priority email, and lifecycle updates.
6. **Finis** — `FinisMarker`.

`AtmosphereLayer` (turmeric tint) renders behind everything.

## Local nav approach

**Provides chapters to `FloatingNavIsland`** per D2 (synthesis). The chapters inventory:

- `hero` / lead context
- `schedule` / meetings
- `attention` / priorities

Local pill renders these via `FloatingNavIsland`'s chapters contract; click smooth-scrolls to the section. Active chapter highlights via scroll-spy.

**No `DayStrip`** — D-spine mockup invents `DayStrip` (Yesterday / Today / Tomorrow) that "replaces nav island"; this proposal is **rejected** per D2. App-level navigation must remain present on briefing — it's the user's home base. If time-scoped nav (Yesterday / Today / Tomorrow) becomes a real product need, it surfaces as a separate pattern with its own justification, not as an implicit replacement.

## Patterns consumed

- `FolioBar` (chrome)
- `FloatingNavIsland` (chrome) — receives `chapters` prop
- `AtmosphereLayer` (chrome, tint=turmeric)
- `MarginGrid` (every section)
- `MeetingCard`
- `BriefingMeetingCard`
- `DailyBriefingAttentionSection` (shipped-local)
- `FinisMarker`

Roadmap/prototype only:

- `Lead`
- `DayChart`
- `MeetingSpineItem`
- `EntityPortraitCard`
- `ThreadMark`
- `AskAnythingDock`

## Primitives consumed

- `Pill` (status pills, prep states)
- `IntelligenceQualityBadge` (per-meeting prep state — replacing the local `prep-state` chip from D-spine)
- `EntityChip` (entity references inside content)
- `HealthBadge`
- `FolioRefreshButton`

## Tokens

- Primary tint: `turmeric` (atmosphere, FloatingNavIsland active state when on this surface)
- All standard token families (color, typography, spacing, motion)

## Notable interactions

- **Live refresh**: surface refreshes every 2 minutes when on screen; FolioBar's pulsing brand mark indicates live status
- **Inline meeting expansion**: upcoming/in-progress rows expand in place to show prep, the room, and before-meeting actions.
- **Past meeting navigation**: past rows navigate to MeetingDetail.
- **Priority completion**: action rows can be completed from the briefing.

## Empty / loading / error states

- **Loading** — `EditorialLoading` skeleton; FolioBar shows "Building briefing…"
- **Error** — `EditorialError` (terracotta) with retry
- **Empty (no meetings, no movement, no watch)** — Lead sentence shifts to "quiet day" register; sections collapse to placeholder summaries
- **Cached / stale** — banner via FolioBar status text ("Cached briefing — refreshing…")

## Naming notes

Canonical name `DailyBriefing` per `NAMING.md`. Current route wrapper name `DashboardPage` is the legacy mismatch. The rename is tracked under DOS-360 (DS-XCUT-04), deferred to post-v1.4.3 to avoid bundling rename churn with the redesign.

The src component `src/components/dashboard/DailyBriefing.tsx` is already named correctly; only the inline route / page wrapper in `src/router.tsx` needs the rename or extraction.

## Naming bug to fix during v1.4.3

Per Audit 04: `DailyBriefing` accepts `freshness` prop but it's currently aliased `_freshness` (received but unused). v1.4.3 should consume the prop properly when `FreshnessIndicator` is wired in.

## History

- 2026-05-02 — Surface spec authored as part of Wave 1 (v1.4.3 substrate prep).
- 2026-05-05 — Corrected spec to shipped source. Spine-D components remain prototype/roadmap until routed UI consumes them.
- DOS-360 — `Dashboard` → `DailyBriefing` rename, deferred post-v1.4.3.
