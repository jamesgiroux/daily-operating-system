# I502 — Health Surfaces Across All Pages

**Priority:** P1
**Area:** Frontend / Pages + Components
**Version:** 1.0.0
**Depends on:** I499 (engine), I500 (Glean parsing), I503 (schema)
**ADR:** 0097

## Problem

Health data currently renders as a single colored dot on the accounts list (derived from the user-set `accounts.health` RAG field) and a static badge on the account hero. There is no display of:
- The computed health score (0-100)
- Confidence bands
- Relationship dimension sub-scores or evidence
- Divergence alerts (org score vs relationship signals)
- Health context on meeting briefings, daily briefings, or the week page
- Trend arrows alongside health dots

ADR-0097 defines 7+ surfaces where structured health data should appear. The `AccountHealth` type (I503) provides all the data — this issue renders it.

## Design

### Surface 1: Accounts List (`src/pages/AccountsPage.tsx`)

**Current:** `healthDotColor[account.health ?? ""]` renders a colored dot from the user-set RAG field.

**New:** Replace the simple dot with a health dot + trend arrow.

- Dot color: derived from `intelligence.health.band` (green/yellow/red) instead of the manual `account.health`. Fall back to `account.health` when `intelligence.health` is not yet computed.
- Trend arrow: small arrow icon (up/down/stable) next to the dot, derived from `intelligence.health.trend.direction`. Use `ArrowUpRight` for improving, `ArrowDownRight` for declining, `Minus` for stable (from lucide-react).
- Color mapping stays the same: green = `var(--color-garden-sage)`, yellow = `var(--color-spice-saffron)`, red = `var(--color-earth-terracotta)`.

Data source: the accounts list endpoint already includes `intelligence` on each account. The `health` field on `EntityIntelligence` (per I503) is available without additional API calls.

### Surface 2: Account Detail Hero (`src/components/account/AccountHero.tsx`)

**Current:** Health badge shows the user-set RAG as a colored label.

**New:** Replace with a richer health display:
- Score + band: `"72 — Stable"` with the score number in the band's color
- Trend icon: arrow matching `trend.direction`, inline after the text
- Confidence qualifier: when `confidence < 0.5`, append `"(limited signals)"` in muted text
- Source indicator: small label `"org"` / `"computed"` / `"you"` below the score, using `var(--color-ink-caption)` text

Layout: the score replaces the existing health badge position. The user's manual RAG badge remains visible as a separate, smaller indicator (their professional judgment preserved alongside the system score, per ADR-0097).

### Surface 3: Account Detail State of Play (`src/pages/AccountDetailEditorial.tsx`)

**Current:** The State of Play chapter shows `CurrentState` (working/not working/unknowns).

**New:** Add a "Relationship Health" section within the State of Play chapter, below the existing current state content. This section renders the 6 relationship dimensions:

```
┌──────────────────────────────────────────────────────┐
│ RELATIONSHIP HEALTH                                   │
│                                                       │
│  Meeting Cadence        ████████░░  78  ↗ improving   │
│  Email Engagement       ██████░░░░  62  → stable      │
│  Stakeholder Coverage   ████░░░░░░  41  ↘ declining   │
│  Champion Health        ███████░░░  72  → stable      │
│  Financial Proximity    █████████░  89  ↗ improving   │
│  Signal Momentum        ██████░░░░  55  → stable      │
│                                                       │
│  Evidence: "12 meetings in 90d, last meeting 3d ago"  │
│  Evidence: "Champion attended 4 of last 5 meetings"   │
└──────────────────────────────────────────────────────┘
```

Each dimension renders as:
- Label (left-aligned, `var(--font-body)`)
- Progress bar (0-100, colored by score: green >= 70, yellow >= 40, red < 40)
- Score number
- Trend arrow + label

Below the 6 dimensions, show top 3 evidence items from the highest-weighted dimensions, using `var(--font-caption)` in `var(--color-ink-caption)`.

Component: new `RelationshipDimensions` component in `src/components/account/RelationshipDimensions.tsx`. Consumes `RelationshipDimensions` type from `src/types/index.ts`.

### Surface 4: Account Detail Divergence Alert (`src/pages/AccountDetailEditorial.tsx`)

**New:** When `intelligence.health.divergence` is non-null, render an alert banner at the top of the account detail page, above the hero.

- **Critical divergence** (`severity: "critical"`): Banner with `var(--color-earth-terracotta)` border-left, terracotta text, exclamation icon.
- **Notable divergence** (`severity: "notable"`): Banner with `var(--color-spice-saffron)` border-left, saffron text, info icon.
- **Minor divergence**: No banner — mentioned in the narrative only.

Banner content: `divergence.description` text. If `leading_indicator` is true, prepend a "Leading indicator" label.

Component: new `DivergenceAlert` component in `src/components/account/DivergenceAlert.tsx`.

### Surface 5: Meeting Briefing Hero (`src/pages/MeetingDetailPage.tsx`)

**Current:** Meeting detail page shows meeting metadata and prep content. No account health context.

**New:** In the meeting briefing hero section, when the meeting is linked to an account entity that has health data:
- Health band badge: colored pill showing `"Green"` / `"Yellow"` / `"Red"` + score number
- Divergence alert: if the account has a divergence alert, show a compact version inline: `"⚠ Org: Green, Relationship signals declining"` in `var(--color-spice-saffron)` text

Data source: the meeting detail endpoint already returns linked entities. Load the account's intelligence data from the existing `intelligence` field on the account detail response.

### Surface 6: Daily Briefing Attention Section (`src/components/dashboard/DailyBriefing.tsx`)

**Current:** The `AttentionSection` shows proposed actions and overdue items.

**New:** Add a health attention line when any accounts have declining relationship signals. This appears as a single line at the top of the attention section:

`"3 accounts with declining relationship signals"` — clickable, navigates to the accounts list filtered by health trend = declining.

Computation: iterate over all non-archived accounts, count those where `intelligence.health.trend.direction === "declining"`. Only show if count > 0.

This requires adding health data to the daily briefing data fetch. The accounts list is already loaded for the daily briefing's schedule/meeting context — add a `decliningHealthCount` field to the briefing data or compute it client-side from the accounts query.

### Surface 7: Week Page (`src/pages/WeekPage.tsx`)

**Current:** Weekly forecast shows meetings grouped by day with attendees and prep status.

**New:** For each meeting linked to an account, show a small health badge inline:
- Colored dot + score number next to the account name
- Only shows for meetings with account entities that have health data
- No divergence alerts here — too noisy for the weekly view

### Design System Compliance

All new components must use existing design tokens from `src/styles/design-tokens.css`:
- Colors: `var(--color-garden-sage)`, `var(--color-spice-saffron)`, `var(--color-earth-terracotta)` for green/yellow/red
- Typography: `var(--font-body)` for labels, `var(--font-caption)` for evidence, `var(--font-data)` (JetBrains Mono) for score numbers
- Spacing: use `var(--space-*)` tokens. Section rules over cards per ADR-0073.
- No new colors or font stacks. Progress bars use the existing palette.

## Files to Modify

| File | Change |
|---|---|
| `src/pages/AccountsPage.tsx` | Replace health dot source from `account.health` to `intelligence.health.band` with fallback. Add trend arrow icon. |
| `src/components/account/AccountHero.tsx` | Replace health badge with score + band + trend + confidence + source display. Keep user RAG as secondary badge. |
| `src/pages/AccountDetailEditorial.tsx` | Add RelationshipDimensions component to State of Play chapter. Add DivergenceAlert banner above hero. |
| `src/components/account/RelationshipDimensions.tsx` | New component: 6-dimension progress bars with scores, trends, and evidence. |
| `src/components/account/DivergenceAlert.tsx` | New component: alert banner for critical/notable health divergence. |
| `src/pages/MeetingDetailPage.tsx` | Add account health band badge and compact divergence alert to meeting briefing hero. |
| `src/components/dashboard/DailyBriefing.tsx` | Add declining health count attention line to AttentionSection. |
| `src/pages/WeekPage.tsx` | Add health dot + score inline on meetings with account entities. |

## Acceptance Criteria

1. Accounts list shows health dots derived from `intelligence.health.band` (not just user-set RAG), with trend arrows for accounts that have computed health
2. Account detail hero displays the health score number, band color, trend arrow, and confidence qualifier
3. Account detail State of Play chapter includes a "Relationship Health" section with 6 dimension progress bars, scores, and trend indicators
4. When an account has a critical divergence, a terracotta-bordered alert banner appears at the top of the account detail page with the divergence description
5. Meeting briefing hero shows the linked account's health band badge; if divergence exists, a compact alert appears inline
6. Daily briefing attention section shows "N accounts with declining relationship signals" when N > 0
7. Week page shows a health dot + score next to account names on meetings with account entities
8. All health colors use design system tokens: sage (green), saffron (yellow), terracotta (red) — verified via design-tokens.css
9. Score numbers use `var(--font-data)` (JetBrains Mono). Evidence text uses `var(--font-caption)`.
10. Accounts without health data gracefully fall back to the existing user-set RAG dot (no blank spaces or missing indicators)

## Open Design Decision: Sparse Dimension Display

I499 documents that **3 of 6 dimensions will be null for most users** (no champion assigned, no financial data, no email signals). Surface 3's six-bar display needs a design decision for null dimensions. Options include: show all 6 with greyed-out "No data" state, show only populated dimensions, or show populated + unlock prompts ("Assign a champion to see Champion Health"). **This decision should be made during implementation when the actual data shapes are visible, not speculatively now.** The AC should validate whichever approach is chosen against a sparse account (1-2 populated dimensions).

## Out of Scope

- Portfolio page health heatmap (I492 — separate issue)
- Report consumption of health dimensions (reports already access `AccountHealth` via the intelligence JSON)
- Editing health scores from the UI (the score is computed, not user-set; the user's manual RAG remains editable via the existing VitalsStrip)
- Health trend sparklines or historical charts (requires multiple enrichment cycles of data — future enhancement)
- Mobile/responsive layout adjustments (desktop-first per current app scope)
