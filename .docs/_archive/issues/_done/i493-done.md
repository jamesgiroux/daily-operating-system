# I493 — Account Detail Enriched Intelligence Surface

**Priority:** P1
**Area:** Frontend / Entity
**Version:** 1.0.0
**Depends on:** I499 (health scoring engine), I503 (health schema types), I505 (Glean stakeholder intelligence)

## Problem

The account detail page displays health as a simple color dot and text label ("Green Health"). With I499/I503 delivering structured health scoring and I505 (absorbs I486) delivering Glean-sourced person metadata, the surface can now render meaningfully richer intelligence: relationship depth as structured coverage analysis, stakeholder titles from Glean, and consolidated report access.

**Note:** Health rendering across all pages (accounts list, account hero, State of Play dimensions, divergence alerts, meeting briefings, daily briefing, week page) is owned by I502. This issue focuses on the **non-health** account detail enhancements: relationship depth structure in State of Play, Glean-sourced titles in The Room, and the Reports chapter.

The current page has 8 chapters (Headline, State of Play, The Room, Watch List, The Record, The Work, Appendix, Finis). This issue enhances 2 existing chapters and adds 1 new chapter. It does not restructure the page — it deepens what's already there.

## Design

### 1. State of Play Enhancement — Relationship Depth as Structured Fields

> **Note:** Health band rendering in the Headline/Hero area is owned by I502 (Surface 2). This issue does NOT add health UI.

The `StateOfPlay` component currently renders `working` and `struggling` text blocks from intelligence. Add a structured relationship depth section below the existing narrative content.

**New fields from `relationship_depth`:**

| Field | Display | Format |
|---|---|---|
| Champion strength | "Champion: Strong / Moderate / Weak / None identified" | Pill with appropriate color |
| Executive access | "Executive access: Direct / Indirect / None" | Text row |
| Stakeholder coverage | "Coverage: 4 of 6 roles filled (67%)" | Progress-style text with percentage |
| Coverage gaps | "Missing: Executive Sponsor, Technical Lead" | Terracotta text for visibility |

**Layout:** Margin grid pattern (`s.marginGrid` from editorial-briefing.module.css). Mono label "RELATIONSHIP DEPTH" in the left column, structured fields in the right column. Thin divider above to separate from the narrative State of Play content.

**Coverage gaps** render with terracotta accent when roles defined by the active preset's `stakeholder_roles` are unassigned. E.g., if CS preset defines Champion, Executive Sponsor, Technical Lead, and Day-to-Day — but Executive Sponsor has no person assigned — show "No executive sponsor identified" in terracotta.

### 2. The Room Enhancement — Glean-Sourced Titles + Engagement Level

The `StakeholderGallery` currently shows stakeholder names and roles. With I486's Glean person writeback active, stakeholders now have `title` and `department` fields populated from Glean.

**Enhancements to each stakeholder row:**
- **Title + Department:** Below the stakeholder name, show "VP Engineering, Platform Team" in DM Sans 13px, secondary color. Only when Glean data is available.
- **Engagement level:** Badge showing activity recency:
  - **Active** (sage pill): met within last 14 days
  - **Warm** (turmeric pill): met within last 30 days
  - **Cold** (terracotta pill): not met in 30+ days or never met
- Engagement calculated from `last_meeting_date` on the person record relative to current date.

**Glean attribution:** When title/department comes from Glean, show a subtle "via Glean" attribution in mono 10px, tertiary color. This builds trust that the data is sourced, not invented.

### 3. New Reports Chapter

Add a new "Reports" chapter after "The Work" and before "Appendix".

**Structure:**
```
ChapterHeading: "Reports"

┌────────────────────────────────────────────────┐
│ VP Account Review                              │  ← Newsreader 19px
│ Executive-ready account summary                │  ← DM Sans 13px, secondary
│ Last generated: Feb 15, 2026  ·  Generate ↗   │  ← Mono 11px
├────────────────────────────────────────────────┤
│ Portfolio Health Summary                       │
│ Health trends and risk factors                 │
│ Not generated  ·  Generate ↗                   │
├────────────────────────────────────────────────┤
│ EBR / QBR                                      │
│ Business review preparation                    │
│ Last generated: Feb 10, 2026  ·  Generate ↗   │
├────────────────────────────────────────────────┤
│ Renewal Readiness                              │  ← Only if account has renewal date
│ Renewal preparation assessment                 │
│ Not generated  ·  Generate ↗                   │
├────────────────────────────────────────────────┤
│ Coaching Patterns                              │  ← Only when 3+ Monthly Wrapped exist
│ CS coaching insights from meeting patterns     │
│ Not generated  ·  Generate ↗                   │
└────────────────────────────────────────────────┘
```

Each report row:
- Report name: Newsreader 19px, primary color, clickable (navigates to report page)
- Description: DM Sans 13px, secondary color
- Status line: JetBrains Mono 11px — "Last generated: {date}" or "Not generated"
- Staleness indicator: if generated 14+ days ago, show "(may be outdated)" in terracotta mono
- Generate button: mono 11px uppercase link in turmeric, triggers report generation

**Availability rules:**
- Renewal Readiness: only shown when account has `renewal_date` set
- Coaching Patterns: only shown when user has 3+ Monthly Wrapped reports
- All other reports: always shown

Use `getAccountReports()` from `src/lib/report-config.ts` for report type definitions. Fetch last-generated dates via existing report listing command.

**Ordering note:** This issue may ship before the CS Report Suite (I489-I498). The Reports chapter must render dynamically from `getAccountReports()`, showing only report types that actually exist in the `ReportType` enum. Currently that's `AccountHealth`, `EbrQbr`, `Swot`, and `RiskBriefing`. As CS report types ship, they register in `getAccountReports()` and appear automatically — no I493 code changes needed.

### Chapter Order (Updated)

```
1. Headline (hero — health rendering per I502)
2. State of Play (narrative + relationship depth)
3. The Room (stakeholders + titles + engagement)
4. Watch List (unchanged)
5. The Record (unchanged)
6. The Work (unchanged)
7. Reports (NEW)
8. Appendix (unchanged)
9. Finis
```

## Files to Modify

| File | Change |
|---|---|
| `src/components/entity/StateOfPlay.tsx` | Add relationship depth structured fields section below existing narrative. Accept `relationshipDepth` and `presetStakeholderRoles` props. |
| `src/components/entity/StakeholderGallery.tsx` | Render title/department from Glean below name. Add engagement level pill (Active/Warm/Cold). Show "via Glean" attribution. |
| `src/components/account/ReportsChapter.tsx` | **New file.** Reports chapter with report rows, availability rules, generate buttons, staleness indicators. |
| `src/pages/AccountDetailEditorial.tsx` | Pass `relationshipDepth` to StateOfPlay. Add "reports" to chapter list. Render `ReportsChapter` between The Work and Appendix. |

## Acceptance Criteria

1. **State of Play** chapter: `relationship_depth` rendered as structured fields — champion strength, executive access, stakeholder coverage percentage, coverage gaps list
2. **The Room** chapter: stakeholders show title/department from Glean (I505) where available + engagement level (active/warm/cold based on last meeting recency)
3. **New Reports chapter**: consolidated access to all report types available for this account. Each shows: report type name, last generated date (or "Not generated"), staleness indicator, one-click generate button
4. Reports chapter respects availability rules: Renewal Readiness only for accounts with renewal date, Coaching Patterns only when 3+ Monthly Wrapped exist
5. Coverage gaps render visibly in State of Play: "No executive sponsor identified" when role is unassigned
6. All components use design system tokens — not arbitrary hex values

## Out of Scope

- **Health rendering** — all health UI (confidence band, trend arrows, dimension progress bars, divergence alerts) is owned by I502
- Restructuring the page chapter order beyond adding Reports
- New backend intelligence calculations — this consumes existing I499/I503/I505 data
- Inline report viewing — generate navigates to the dedicated report page
- Account health history chart (future — needs time-series data)
- Editing health score directly — health is system-computed, not user-editable
