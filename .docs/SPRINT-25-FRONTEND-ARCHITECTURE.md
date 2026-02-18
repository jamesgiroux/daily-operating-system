# Sprint 25 Frontend Architecture: Magazine-Layout Redesign

**Objective:** Refactor DailyOS main pages from dashboard/card-based layouts to magazine-layout editorial design.

**Design Reference:** ADR-0077 (Magazine-Layout Editorial Redesign)

**Mockups:** `/Users/jamesgiroux/Desktop/dailyos-*.html` (5 files)

**Implementation Target:** 80-90% with Haiku, code/aesthetic review with senior developer

---

## 1. Design System & CSS Structure

### 1.1 Shared Design Tokens

All color, typography, and spacing values come from ADR-0076 (Brand Identity). Create a centralized design token file.

**Suggested file:** `src/styles/design-tokens.css`

```css
:root {
  /* Paper (grounds) */
  --color-paper-cream: #f5f2ef;
  --color-paper-linen: #e8e2d9;
  --color-paper-warm-white: #faf8f6;

  /* Desk (frame) */
  --color-desk-charcoal: #1e2530;
  --color-desk-ink: #2a2b3d;
  --color-desk-espresso: #3d2e27;

  /* Spice (warm accents) */
  --color-spice-turmeric: #c9a227;
  --color-spice-saffron: #deb841;
  --color-spice-terracotta: #c4654a;
  --color-spice-chili: #9b3a2a;

  /* Garden (cool accents) */
  --color-garden-sage: #7eaa7b;
  --color-garden-olive: #6b7c52;
  --color-garden-rosemary: #4a6741;
  --color-garden-larkspur: #8fa3c4;

  /* Semantic */
  --color-text-primary: var(--color-desk-charcoal);
  --color-text-secondary: #5a6370;
  --color-text-tertiary: #8a919a;
  --color-rule-heavy: rgba(30, 37, 48, 0.12);
  --color-rule-light: rgba(30, 37, 48, 0.06);

  /* Typography */
  --font-serif: 'Newsreader', Georgia, serif;
  --font-sans: 'DM Sans', -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
  --font-mark: 'Montserrat', sans-serif;

  /* Spacing */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;
  --space-2xl: 48px;
  --space-3xl: 56px;
  --space-4xl: 72px;
  --space-5xl: 80px;
}
```

### 1.2 Shared Components

**Files to create/refactor:**

| Component | File | Purpose |
|-----------|------|---------|
| FolioBar | `src/components/layout/FolioBar.tsx` | Fixed header, masthead, readiness stats |
| FloatingNavIsland | `src/components/layout/FloatingNavIsland.tsx` | Right-margin nav, icon tooltips |
| AtmosphereLayer | `src/components/layout/AtmosphereLayer.tsx` | Fixed background gradients + watermark |
| HeroSection | `src/components/layout/HeroSection.tsx` | Page hero with headline + narrative |
| SectionRule | `src/components/layout/SectionRule.tsx` | Thin rule dividers |
| MarginGrid | `src/components/layout/MarginGrid.tsx` | 100px label column + content |
| CompactRow | `src/components/ui/CompactRow.tsx` | Reusable row for schedule, actions, stakeholders |

**Shared component layout wrapper:**

```tsx
// src/components/layout/MagazinePageLayout.tsx
export function MagazinePageLayout({
  heroSection,
  atmosphereColor, // 'turmeric' | 'terracotta' | 'larkspur'
  activePage, // for nav island active state
  children, // page sections
}: {
  heroSection: React.ReactNode;
  atmosphereColor: string;
  activePage: string;
  children: React.ReactNode;
}) {
  return (
    <div className="magazine-page">
      <AtmosphereLayer color={atmosphereColor} />
      <FolioBar activePage={activePage} />
      <FloatingNavIsland activePage={activePage} activeColor={atmosphereColor} />
      <main className="page-container">
        {heroSection}
        {children}
      </main>
    </div>
  );
}
```

---

## 2. Component Inventory

### 2.1 Existing Components to Preserve

These components work with the new layout (may need minor style updates):

- MeetingCard → Adapt to compact row format
- ActionItem → Adapt to compact row with context line
- EmailPreview → Adapt to compact row with prefix icon
- TagBadge → Reuse for entity accent colors, status indicators
- SearchInput → Integrate into folio search button

### 2.2 Existing Components to Refactor

These components need significant refactoring:

| Component | Current | New | Notes |
|-----------|---------|-----|-------|
| Dashboard | Grid layout + stats cards | Hero + schedule + loose threads | Rewrap existing data |
| ActionsList | Card grid | Temporal sections (overdue/today/week/upcoming) | Reorganize by urgency |
| SidebarNav | 240px fixed sidebar | Floating island (right margin) | Move to nav island |
| StatsRow | Card grid (readiness, stats) | Folio bar right section + status footer | Integrate into folio |
| PageHeader | Typography + icon | Folio bar component | Consolidate |

### 2.3 New Components to Build

| Component | File | Used In | Purpose |
|-----------|------|---------|---------|
| HeroSection | `src/components/layout/HeroSection.tsx` | All pages | 65-76px headline + narrative |
| SectionTitle | `src/components/ui/SectionTitle.tsx` | All pages | 22-28px title with margin label |
| ProseBlock | `src/components/ui/ProseBlock.tsx` | Account, Meeting intel | Prose paragraph with bold emphasis |
| ContextLine | `src/components/ui/ContextLine.tsx` | All pages | Italic gray context ("why now?") |
| RiskRow | `src/components/ui/RiskRow.tsx` | Account, Meeting intel | Risk title + context + severity dot |
| ParticipantRow | `src/components/ui/ParticipantRow.tsx` | Meeting intel | Name/title/role + intelligence + last contact |
| TalkingPoint | `src/components/ui/TalkingPoint.tsx` | Meeting intel | Numbered point with context |
| TimelineEntry | `src/components/ui/TimelineEntry.tsx` | Account | Date + event + context |
| EntityAccentBorder | `src/components/ui/EntityAccentBorder.tsx` | Rows | Thin left border (turmeric/larkspur/linen) |

---

## 3. Page-by-Page Rebuild Guide

### 3.1 Dashboard Redesign

**Current:** Grid with stats cards, separate sections (Today's Focus, Top 3, Schedule, Actions, Emails)

**New:** Magazine layout with hero, featured meeting, schedule rows, loose threads

**Data Sources:**
- Hero narrative: synthesized from briefing intelligence (account prep, meeting count, day character)
- Focus statement: top priority from intelligence.json
- Featured meeting: most important meeting of the day (QBR, renewal, critical stakeholder)
- Schedule rows: live from SQLite schedule table + prep status from intelligence
- Loose threads: live from SQLite actions + emails, grouped by relevance not type

**Implementation Steps:**
1. Replace current dashboard with MagazinePageLayout wrapper
2. Build HeroSection with synthesized headline (from briefing intelligence or AI synthesis)
3. Render current schedule data as CompactRow items
4. Reorganize actions + emails into "Loose Threads" section (interwoven, not separate)
5. Extract folio-right readiness stats from previous StatsRow component
6. Test with live data from current data sources

**Key File:** `src/pages/DashboardPage.tsx`

---

### 3.2 List Page Redesign

**Current:** Card-per-row layout, grouped by entity (Accounts/Projects/People)

**New:** Signal-first flat rows with entity accent borders

**Data Sources:**
- Rows: from list table (accounts, projects, or people)
- Entity color: account (turmeric), project (olive), person (larkspur)
- Status indicator: prep status, health score, or last interaction

**Implementation Steps:**
1. Replace current card layout with CompactRow grid
2. Add entity accent borders (3px left border in entity color)
3. Implement ListColumn components for thumbnail, name, context, status
4. Keep existing list filtering and sorting
5. Test with current data sources

**Key Files:** `src/pages/ListPage.tsx`, `src/components/ui/ListRow.tsx`, `src/components/ui/ListColumn.tsx`

---

### 3.3 Actions List Redesign

**Current:** Card grid or flat list without temporal grouping

**New:** Temporal sections (Overdue → Due Today → This Week → Upcoming) with tapering density

**Data Sources:**
- Actions: from SQLite actions table
- Due dates: from action.dueAt field
- Context: linked meeting name, person name, or description
- Status: overdue (terracotta), today (turmeric), upcoming (neutral)

**Implementation Steps:**
1. Query actions table, organize by due date into four buckets
2. Build four sections with SectionTitle (margin label + count)
3. Each section is a CompactRow list with:
   - Arrow prefix (`→` for actions, `✉` for emails)
   - Action text (weight 500 for overdue/today, 400 for later)
   - Context line (italic, "why now?" linked to meeting or person)
   - Due date chip (color-coded: terracotta/turmeric/neutral)
4. Implement shorter hero (42-45vh instead of 65vh)
5. Replace `* * *` end with status footer: "12 open · 3 completed today · last updated 8:14a"

**Key File:** `src/pages/ActionsPage.tsx`

---

### 3.4 Account Detail Page (New)

**Purpose:** Deep dive into a single account, showing intelligence, risks, stakeholders, activity

**Data Sources:**
- Account name + lifecycle: from accounts table
- Executive assessment: from account intelligence.json
- Current state, risks, wins: from intelligence.json
- Stakeholders: from people table (linked to account)
- Recent activity: from SQLite captures table + meeting history
- Actions: from actions table (linked to account)

**Implementation Steps:**
1. Create new route: `/accounts/:accountId`
2. Build MagazinePageLayout with turmeric atmosphere
3. Render seven sections from intelligence.json + live data:
   - Current State (prose paragraph)
   - Key Risks (RiskRow items with severity dots)
   - Recent Wins (RiskRow items with sage dots)
   - Stakeholder Map (ParticipantRow items)
   - Recent Activity (TimelineEntry items, reverse chronological)
   - Upcoming (meetings/milestones linked to this account)
   - Actions (action items linked to this account)
4. Link from account list and from featured meetings

**Key File:** `src/pages/AccountDetailPage.tsx`

---

### 3.5 Meeting Intelligence Report (New)

**Purpose:** Detailed prep for a single meeting, showing strategic context, talking points, risks, outcomes

**Data Sources:**
- Meeting details: from SQLite schedule table + event ID
- Intelligence: from intelligence.json (meeting-specific prep)
- Participants: from meeting attendees list + people table
- Actions: from actions table (linked to meeting)

**Implementation Steps:**
1. Create new route: `/meetings/:meetingId/intelligence`
2. Build MagazinePageLayout with turmeric atmosphere
3. Render seven sections from intelligence + prep data:
   - Why This Meeting Matters (prose)
   - Key Participants (ParticipantRow items)
   - Talking Points (TalkingPoint items with numbered bullets)
   - Risks to Navigate (RiskRow items)
   - Recent History (TimelineEntry items, reverse chronological)
   - Linked Actions (compact rows)
   - Desired Outcomes (prose + outcome statements)
4. Link from featured meeting on dashboard
5. "Back to briefing" link in folio-right

**Key File:** `src/pages/MeetingIntelligencePage.tsx`

---

### 3.6 Weekly Forecast (New)

**Purpose:** Synthesis of the week ahead, organized by priority, readiness, accounts, and shape

**Data Sources:**
- Week summary: synthesized from briefing intelligence or AI synthesis
- Day-by-day: from SQLite schedule table
- Accounts active this week: from schedule table (unique accounts)
- Readiness: from intelligence.json (prep status per external meeting)
- Actions due: from actions table (grouped by due date)

**Implementation Steps:**
1. Create new route: `/week` (replace current weekly view)
2. Build MagazinePageLayout with larkspur atmosphere (slower breathing)
3. Render six sections:
   - Priority (focus block)
   - Week Shape (prose + day-by-day rows)
   - Readiness (synthesized statement + external meeting status rows)
   - Key Accounts (3-4 most active accounts, prose per account)
   - Action Forecast (summary + grouped rows)
   - Week Health (portfolio-level synthesis prose)
4. Larger hero (68vh instead of 65vh) with more generous padding
5. Slower breathing animation (16s instead of 12s)

**Key File:** `src/pages/WeeklyForecastPage.tsx`

---

## 4. Data Flow & Integration

### 4.1 Live Data Integration

**For Dashboard:**
- Schedule: Query SQLite schedule table, filter for today, order by time
- Featured meeting: Filter schedule table by importance signals (QBR, renewal, escalation, large attendee count), pick first
- Actions: Query SQLite actions table, filter for unresolved, order by dueAt
- Emails: Query SQLite emails table, filter for today, order by receivedAt
- Prep status: For each schedule item, check intelligence.json for `nextMeetingReadiness`
- Loose threads: Join actions + emails, order by relevance to today's meetings

**For Actions Page:**
- All actions: Query SQLite actions table, filter for unresolved
- Group by dueAt:
  - Overdue: `dueAt < today`
  - Due today: `dueAt === today`
  - Due this week: `today < dueAt <= endOfWeek`
  - Upcoming: `dueAt > endOfWeek`
- For each action: fetch linked meeting (from meetingId), linked account (from accountId)
- Context line: "→ [action title] · [meeting name] · [account name]"

**For Account Detail:**
- Account: Query accounts table by ID
- Intelligence: Read account intelligence.json file
- Stakeholders: Query people table where accountId = this account
- Recent activity: Query captures table where accountId = this account, limit 5, reverse chronological
- Upcoming: Query schedule table where participants include account stakeholders, limit 3

**For Meeting Intelligence:**
- Meeting: Query schedule table by ID
- Intelligence: Read meeting-specific prep from intelligence.json
- Participants: Parse meeting attendees list + query people table
- Talking points: From intelligence.json meeting-specific section
- Actions: Query actions table where meetingId = this meeting
- Desired outcomes: From intelligence.json or hardcoded per meeting type

**For Weekly Forecast:**
- Week summary: Synthesized from briefing intelligence or AI (new endpoint)
- Day-by-day: Query schedule table for week, group by day
- Accounts active: Collect unique accountIds from schedule table for this week
- Readiness: Query intelligence.json, extract meeting prep status for each external meeting
- Actions due: Query actions table for week, group by dueAt
- Health: From intelligence.json or synthesized by AI

### 4.2 New Queries Needed

**In `src-tauri/src/queries/`:**

1. `featured_meeting(date: String) -> Meeting` — return most important meeting for a given date
2. `daily_intelligence_summary(date: String) -> IntelligenceSummary` — synthesized context for hero headline
3. `week_intelligence_summary(weekStart: String) -> WeekSummary` — synthesized context for weekly forecast hero
4. `external_meetings_readiness(weekStart: String) -> Vec<MeetingReadiness>` — prep status for each external meeting in week

All other queries already exist (schedule, actions, emails, accounts, people, intelligence).

---

## 5. Styling Strategy

### 5.1 CSS Architecture

**Option A: CSS Modules (Recommended)**
- `src/styles/design-tokens.css` — shared variables
- `src/styles/shared.css` — folio, nav island, atmosphere, shared patterns
- `src/components/layout/FolioBar.module.css`
- `src/components/layout/FloatingNavIsland.module.css`
- `src/components/ui/CompactRow.module.css`
- etc.

**Option B: Tailwind + Design Tokens**
- Extend Tailwind config with design token colors
- Use utility classes + custom classes for complex layouts
- May require custom Tailwind plugin for atmosphere/watermark patterns

**Option C: Inline Styles (Not Recommended)**
- CSS-in-JS (emotion, styled-components)
- Higher overhead for performance; avoid for this project

**Recommended: Option A** (CSS Modules + shared tokens)

### 5.2 Key CSS Patterns

**Folio bar:**
```css
.folio {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 100;
  height: 40px;
  padding: 10px 48px;
  background: rgba(245, 242, 239, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--color-rule-heavy);
}
```

**Floating nav island:**
```css
.nav-island {
  position: fixed;
  right: 28px;
  top: 50%;
  transform: translateY(-50%);
  z-index: 100;
  padding: 8px;
  background: rgba(250, 248, 246, 0.8);
  backdrop-filter: blur(12px);
  border-radius: 16px;
  box-shadow: 0 2px 12px rgba(30, 37, 48, 0.06);
}
```

**Atmosphere:**
```css
.atmosphere {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 100vh;
  pointer-events: none;
  z-index: 0;
  background: radial-gradient(...); /* page-specific */
  animation: atmosphere-breathe 12s ease-in-out infinite;
}
```

**Page container:**
```css
.page-container {
  position: relative;
  z-index: 1;
  max-width: 1100px;
  margin: 0 auto;
  padding: 0 120px 160px;
  margin-top: 40px;
}
```

**Margin grid (label + content):**
```css
.margin-grid {
  display: grid;
  grid-template-columns: 100px 1fr;
  gap: 0 32px;
  align-items: first baseline;
}
```

**Entity accent border:**
```css
.accent-account {
  border-left: 3px solid var(--color-spice-turmeric);
  padding-left: 12px;
  margin-left: -15px;
}
```

**Tapering density:**
```css
.section-overdue {
  font-weight: 500;
  padding: 16px 0;
}
.section-today {
  font-weight: 500;
  padding: 14px 0;
}
.section-upcoming {
  font-weight: 400;
  padding: 10px 0;
  color: var(--color-text-tertiary);
}
```

---

## 6. Implementation Sequencing

### Phase 1: Design System & Shared Components (Week 1)
- [ ] Create `src/styles/design-tokens.css` with all color/type/spacing tokens
- [ ] Build FolioBar component
- [ ] Build FloatingNavIsland component
- [ ] Build AtmosphereLayer component (with page-specific color props)
- [ ] Build MagazinePageLayout wrapper
- [ ] Create CompactRow + CompactColumn components

**Deliverable:** Shared chrome works on all pages, atmosphere + watermark render, nav island tooltips work

### Phase 2: Dashboard Redesign (Week 2)
- [ ] Refactor DashboardPage to use MagazinePageLayout
- [ ] Build HeroSection with synthesized headline
- [ ] Render schedule rows from live data
- [ ] Rewrap actions + emails as "Loose Threads"
- [ ] Test featured meeting isolation
- [ ] Integrate readiness stats into folio-right

**Deliverable:** Dashboard works with live data, visual layout matches mockup, performance baseline

### Phase 3: List Page & Actions List (Week 3)
- [ ] Refactor ListPage to use CompactRow + entity accent borders
- [ ] Refactor ActionsPage to temporal sections
- [ ] Build SectionTitle + ContextLine components
- [ ] Implement shorter hero on ActionsPage
- [ ] Test sorting/filtering in new layout
- [ ] Status footer instead of `* * *` end mark

**Deliverable:** Both pages use new layout, temporal grouping works, live data flows

### Phase 4: Entity Pages — Account Detail & Meeting Intelligence (Week 4)
- [ ] Create AccountDetailPage route + component
- [ ] Build ProseBlock, RiskRow, ParticipantRow, TimelineEntry components
- [ ] Query account intelligence.json + live stakeholders
- [ ] Create MeetingIntelligencePage route + component
- [ ] Build TalkingPoint component
- [ ] Link from dashboard (featured meeting) to intelligence page

**Deliverable:** Both entity pages render, intelligence data integrates, links work

### Phase 5: Weekly Forecast (Week 5)
- [ ] Create WeeklyForecastPage route + component
- [ ] Build day-by-day section with margin grid
- [ ] Implement larger hero + slower breathing animation
- [ ] Query weekly synthesis data
- [ ] Test action grouping by due date
- [ ] Link from nav island

**Deliverable:** Weekly forecast page works, data integrates, aesthetic complete

### Phase 6: Code Review & Aesthetic Polish (Week 6)
- [ ] Comprehensive code review (component structure, performance, edge cases)
- [ ] Aesthetic review (typography, spacing, colors in context)
- [ ] Performance profiling (layout shifts, animation smoothness)
- [ ] Accessibility audit (keyboard nav, screen readers, contrast)
- [ ] Bug fixes + edge case handling (long titles, many attendees, mobile)

**Deliverable:** Ship-ready frontend redesign

---

## 7. Testing Strategy

### 7.1 Visual Regression Testing
- Capture screenshots of each page at key breakpoints (1440px, 1024px, 768px)
- Compare against mockups (should match within ~5% visual differences)
- Test on actual macOS (Tauri v2 app window)

### 7.2 Data Integration Testing
- Dashboard: test with 0, 3, 10+ meetings, 0, 5, 20+ actions
- Actions: test with 0 overdue, many overdue, across multiple accounts
- Account detail: test with 0, 5, 20+ stakeholders
- Meeting intel: test with 1, 3, 10 participants
- Weekly: test with light week, heavy week, all-day events

### 7.3 Performance Testing
- Measure Time to Interactive (TTI) for each page
- Profile rendering with React DevTools
- Check for layout shifts during animation
- Test with low-end machine simulation (Chromium throttling)

### 7.4 Edge Case Testing
- Long action titles (> 60 chars)
- Many attendees (20+ in a meeting)
- Empty states (no actions, no accounts, no prep)
- Missing data (null fields, missing intelligence.json)
- Very small viewport (iPad, phone)

---

## 8. Handoff to Haiku

**Haiku Task Template:**

```
You are refactoring DailyOS frontend from dashboard layout to magazine-layout editorial design.

Reference ADR-0077 (Magazine-Layout Editorial Redesign) and the five HTML mockups.

Your task: Build Phase 1 (Design System & Shared Components)

Deliverables:
1. src/styles/design-tokens.css — all color/type/spacing tokens from ADR-0076
2. src/components/layout/FolioBar.tsx — fixed header with brand mark, pub label, readiness stats
3. src/components/layout/FloatingNavIsland.tsx — right-margin nav island with icon tooltips
4. src/components/layout/AtmosphereLayer.tsx — fixed atmosphere gradient + watermark
5. src/components/layout/MagazinePageLayout.tsx — wrapper component combining all above
6. src/components/ui/CompactRow.tsx — reusable row component for lists

Design System:
- All color values from ADR-0076 (turmeric, terracotta, larkspur, etc.)
- Typography: Newsreader (serif), DM Sans (body), JetBrains Mono (mono), Montserrat (mark)
- Spacing: use 8px, 16px, 32px, 48px, 56px, 72px, 80px multiples
- Folio bar: 40px height, cream+blur frosted glass, left brand mark + pub label, right readiness stats
- Nav island: 28px from right edge, centered vertically, 36px items, turmeric active state
- Atmosphere: fixed position, page-specific radial gradient (turmeric/terracotta/larkspur), breathing animation

Focus on:
- Clean component props (atmosphereColor, activePage, etc.)
- CSS organization (use CSS Modules or shared CSS file)
- Consistent spacing + typography
- Hover states on interactive elements
- Accessibility (semantic HTML, ARIA where needed)

Don't worry about:
- Live data integration (that's Phase 2)
- Mobile responsiveness (focus on 1440px)
- Integration with existing pages (we'll do that in Phase 2)

Output:
- All .tsx and .css files in proper directories
- Brief README in src/styles/ explaining design token structure
- No breaking changes to existing components
```

---

## 9. Notes for Code Reviewers

### Key Review Points

**Phase 1 Audit (Shared Components):**
- [ ] Design tokens are used consistently (no hardcoded colors)
- [ ] Folio bar is 40px, uses frosted glass correctly, all elements present
- [ ] Nav island is positioned at 28px right, centered vertically, all items present
- [ ] Atmosphere is behind all content (z-index: 0), watermark is behind hero (z-index: -1)
- [ ] Component props are clean and well-typed
- [ ] CSS is organized (either modules or shared file, not inline styles)
- [ ] No layout shifts on hover/animation

**Phase 2-5 Audits (Page Redesigns):**
- [ ] Pages use MagazinePageLayout wrapper
- [ ] Content follows section hierarchy from ADR-0077
- [ ] Typography matches mockups (hero 76px, body 15-16px, labels 10-11px mono)
- [ ] Spacing matches mockups (~56px between sections, ~80px hero top padding)
- [ ] Entity accent borders present on entity-linked rows
- [ ] Tapering density implemented (heavy → medium → light)
- [ ] "Why now?" context lines present on all actions/items
- [ ] Live data flows correctly (no hardcoded data in production code)

**Phase 6 Audit (Polish):**
- [ ] All interactive elements have hover states
- [ ] Typography is optical (smaller text reads clearly, headlines are not too large)
- [ ] Colors are readable (no contrast issues)
- [ ] Performance: TTI < 2s, no layout shifts during animation
- [ ] Responsive: works at 1440px and 1024px (mobile optional for initial release)
- [ ] Accessibility: keyboard navigation works, semantic HTML, no alt text missing

---

## 10. Future Considerations

**Post-Sprint 25:**
- Mobile responsiveness (Phase 7)
- Dark mode support (new atmosphere colors)
- Advanced animations (entrance/exit transitions)
- Print stylesheet (for exporting briefings)
- Customizable atmosphere colors per profile
- Generative daily art (ambitious future feature from ADR-0077 design exploration)

**Performance Optimizations:**
- Lazy-load atmosphere layer (might be expensive with dual radial gradients)
- Memoize AtmosphereLayer to prevent rerender on page change
- Consider CSS containment for isolated component rendering
- Profile animation performance (breathing 12s animation should be cheap, but measure)

---

## 11. Glossary

| Term | Definition |
|------|-----------|
| **Folio bar** | Editorial masthead (fixed top, 40px) with brand mark, pub label, date, readiness stats |
| **Nav island** | Floating right-margin navigation (8-9 items, icon tooltips) |
| **Atmosphere** | Fixed-position radial gradient backdrop, page-specific color, breathing animation |
| **Watermark** | 420px asterisk behind hero, page-specific color, ~7% opacity |
| **Magazine layout** | Design pattern: editorial hierarchy, prose over bullets, temporal grouping, finite documents |
| **Tapering density** | Progressive decrease in visual weight from important to less important content |
| **Margin grid** | Two-column layout: 100px label column + 32px gap + content column |
| **Entity accent** | Thin (3px) left border on rows, color by entity type (turmeric/larkspur/linen) |
| **Loose threads** | Actions + emails interwoven by relevance, not separated by type |
| **Context line** | Italic gray line explaining "why now?" for an action or item |
| **Prose block** | Full paragraph with optional bold emphasis, not bullet points |
| **Temporal hierarchy** | Organizing items by when (overdue → today → week → upcoming), not by entity |

---

**End of document.**
