# DailyBriefing

**Tier:** surface
**Status:** redesigning (v1.4.3)
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `DailyBriefing`
**`data-ds-spec`:** `surfaces/DailyBriefing.md`
**Canonical name:** `DailyBriefing`
**Current src name:** `Dashboard.tsx` ← rename candidate (DOS-360, deferred to post-v1.4.3 per D7)
**Source files:**
- `src/pages/Dashboard.tsx` (main route component, to be renamed)
- `src/components/dashboard/DailyBriefing.tsx`
- `src/components/dashboard/DailyBriefing.test.tsx`
- `src/hooks/useDashboardData.ts`
- `src/styles/editorial-briefing.module.css`
- `src/components/onboarding/chapters/DashboardTour.tsx`
- v1.4.3 redesign references `.docs/mockups/claude-design-project/mockups/briefing/variations/D-spine.html`

**Design system version introduced:** 0.1.0

## Job

The morning chief-of-staff briefing — a single editorial page that tells the user what matters today, in one sentence at the top, then in narrative form below. Not a dashboard of metrics; not a calendar; an article you read.

## Layout regions

In reading order:

1. **FolioBar** — surface label "Daily Briefing", center timestamp ("THU · APR 23 · LIVE"), right action set (refresh + Ask anything ⌘K), pulsing brand mark for live status
2. **The Lead** (`Lead` pattern) — single sentence, optional inline `.sharp` highlight; eyebrow ("Today, Thursday April 23")
3. **Today** section (in `MarginGrid`):
   - `ChapterHeading`-style section heading + summary
   - `DayChart` — visual day shape with NOW line
   - Stack of `MeetingSpineItem` instances — one per meeting, full editorial context
4. **What's Moving** section (in `MarginGrid`):
   - Section heading + summary
   - Stack of `EntityPortraitCard` instances — one per entity that shifted overnight
5. **Watch** section (in `MarginGrid`):
   - Section heading + summary
   - List of `WatchListRow` items (passive tracking, with `InferredAction` per item)
6. **Ask** section (in `MarginGrid`):
   - `AskAnythingDock` — multi-line conversational dock
7. **Briefing-end footer** — closer line, refresh status

`AtmosphereLayer` (turmeric tint) renders behind everything.

## Local nav approach

**Provides chapters to `FloatingNavIsland`** per D2 (synthesis). The chapters inventory:

- `today` → "Today" — the meeting spine
- `moving` → "Moving" — entity portraits
- `watch` → "Watch" — passive tracking
- `ask` → "Ask" — the conversational dock

Local pill renders these via `FloatingNavIsland`'s chapters contract; click smooth-scrolls to the section. Active chapter highlights via scroll-spy.

**No `DayStrip`** — D-spine mockup invents `DayStrip` (Yesterday / Today / Tomorrow) that "replaces nav island"; this proposal is **rejected** per D2. App-level navigation must remain present on briefing — it's the user's home base. If time-scoped nav (Yesterday / Today / Tomorrow) becomes a real product need, it surfaces as a separate pattern with its own justification, not as an implicit replacement.

## Patterns consumed

- `FolioBar` (chrome)
- `FloatingNavIsland` (chrome) — receives `chapters` prop
- `AtmosphereLayer` (chrome, tint=turmeric)
- `MarginGrid` (every section)
- `Lead` (the lead sentence)
- `DayChart` (Today section)
- `MeetingSpineItem` (Today section, repeated)
- `EntityPortraitCard` (Moving section, repeated)
- `ThreadMark` (universal hover affordance)
- `AskAnythingDock` (Ask section)
- (proposed Wave 2: `WatchListRow` + `InferredAction` for Watch section — these are v1.4.5 territory; for v1.4.3 the Watch section uses a simpler row pattern documented inline)

## Primitives consumed

- `Pill` (status pills, prep states)
- `IntelligenceQualityBadge` (per-meeting prep state — replacing the local `prep-state` chip from D-spine)
- `TrustBandBadge` (where surface-level trust signals appear)
- `FreshnessIndicator` (chapter-level "as of" labels, claim freshness)
- `ProvenanceTag` (where source attribution helps)
- `EntityChip` (entity references inside content)

## Tokens

- Primary tint: `turmeric` (atmosphere, FloatingNavIsland active state when on this surface)
- All standard token families (color, typography, spacing, motion)

## Notable interactions

- **Live refresh**: surface refreshes every 2 minutes when on screen; FolioBar's pulsing brand mark indicates live status
- **`ThreadMark` everywhere**: hover any addressable line (meeting, thread item, watch row) → "talk" button appears → click seeds AskAnythingDock with context
- **Inferred actions**: Watch items render with default-action button + chev → click chev opens popover with ranked alternatives (Bayesian); v1.4.5 work
- **Briefing readiness**: each meeting shows prep state (`ready` / `building` / `needs`) — drives whether the foot shows briefing link or "Create briefing" CTA

## Empty / loading / error states

- **Loading** — `EditorialLoading` skeleton; FolioBar shows "Building briefing…"
- **Error** — `EditorialError` (terracotta) with retry
- **Empty (no meetings, no movement, no watch)** — Lead sentence shifts to "quiet day" register; sections collapse to placeholder summaries
- **Cached / stale** — banner via FolioBar status text ("Cached briefing — refreshing…")

## Naming notes

Canonical name `DailyBriefing` per `NAMING.md`. Current src name `Dashboard.tsx` is the legacy mismatch. The rename is tracked under DOS-360 (DS-XCUT-04), deferred to post-v1.4.3 to avoid bundling rename churn with the redesign.

The src component `src/components/dashboard/DailyBriefing.tsx` is already named correctly; only the route / page wrapper at `src/pages/Dashboard.tsx` needs the rename.

## Naming bug to fix during v1.4.3

Per Audit 04: `DailyBriefing` accepts `freshness` prop but it's currently aliased `_freshness` (received but unused). v1.4.3 should consume the prop properly when `FreshnessIndicator` is wired in.

## History

- 2026-05-02 — Surface spec authored as part of Wave 1 (v1.4.3 substrate prep).
- v1.4.3 redesign in progress — D-spine mockup is the chosen direction.
- DOS-360 — `Dashboard` → `DailyBriefing` rename, deferred post-v1.4.3.
