# ADR-0077: Magazine-Layout Editorial Redesign

**Status:** Accepted

**Date:** 2026-02-14

**Participants:** James Giroux, Claude Code

---

## Context

The DailyOS app has achieved product-market fit with current functionality (briefing, actions, calendar, intelligence enrichment). However, the visual language and layout patterns feel instrumental rather than editorial. Users interact with dashboards and card-based layouts — tools, not publications.

ADR-0076 established the brand identity: "The computer is personal again." A 1960s-70s techno-humanist aesthetic. Warm, tactile, opinionated. The brand mark (asterisk) functions as art, not chrome. Color system is material-named (turmeric, terracotta, larkspur) not hex-coded.

The brand identity is currently unapplied to the UI. The app still feels like a Tauri port of a web dashboard, not a native Mac publication.

**Design Exploration Outcome:**
Five complete HTML mockups were built to test magazine-layout patterns:
1. Daily Briefing (daily-magazine-layout.html)
2. Account Detail (dailyos-account-detail.html)
3. Meeting Intelligence Report (dailyos-meeting-intel.html)
4. Actions List (dailyos-actions-list.html)
5. Weekly Forecast (dailyos-weekly-forecast.html)

All five mockups share a consistent design system and chrome, with page-specific atmosphere colors and content hierarchy. The design system proved:
- Shared folio bar (editorial masthead, frosted glass, brand mark, readiness stats)
- Shared floating nav island (right margin, icon tooltips, page-specific active state color)
- Shared atmospheric wash (radial gradients, breathing animation, page-specific color)
- Shared asterisk watermark (420px, rotated 12°, page-specific color, ~7% opacity)
- Shared section rules (thin lines, no cards, margin grid pattern with 100px left label column)
- Page-specific content hierarchy (tapering density, temporal grouping, prose-first)

The design system is internally consistent. The aesthetic lands. The five pages form a coherent family.

---

## Decision

**Adopt the magazine-layout editorial redesign across all main pages of the DailyOS app.**

### Design System (Inherited from Mockups)

**Chrome (Shared, Fixed):**
- **Folio bar:** Fixed top, 40px height, frosted glass (`rgba(cream, 0.85)` + `blur(12px)`). Left: brand mark `*` (turmeric, 18px Montserrat 800) + publication label. Center: date/time in mono. Right: readiness stats (sage/terracotta dots + text), status indicator (`>_ ready`), search trigger (`⌘K`).
- **Floating nav island:** Fixed right margin (28px from edge), vertically centered. 36px items in a 4x2 grid with dividers. Frosted glass background. Icon tooltips on hover. Page-specific active state color: turmeric (default), terracotta (actions), larkspur (weekly/forecast).
- **Asterisk watermark:** 420px Montserrat 800, rotated 12°, positioned behind hero content at -z-1. Opacity ~7%. Page-specific color: turmeric (briefing/account/meeting), terracotta (actions), larkspur (weekly).

**Atmosphere:**
- Fixed position radial gradients at very low opacity (4-11% primary). Different color per page to signal context. Breathing animation (12s or 16s depending on page tempo).
  - Daily Briefing: turmeric + larkspur
  - Account Detail: turmeric (warm, entity identity)
  - Meeting Intelligence: turmeric (intense, focused)
  - Actions: terracotta (urgency)
  - Weekly Forecast: larkspur (expansive, forward-looking)

**Page Container:**
- Max-width 1100px, centered, horizontal padding 120px, margin-top 40px (to clear folio)

**Section Patterns:**
- Margin grid: 100px left column (mono labels) + 32px gap + content column. Align items to `first baseline`.
- Section rules: thin (`1px solid var(--rule-heavy)`) horizontal dividers, not cards.
- Entity accent borders: thin left border (3px) on rows. Turmeric (accounts), larkspur (people/1:1), linen (internal).
- No rounded corners except nav island (16px) and folio-search button (4px).

**Typography & Spacing:**
- Hero headlines: Newsreader 76px, weight 400, letter-spacing -0.025em, line-height 1.06
- Body text: DM Sans 15-16px, weight 400, line-height 1.55
- Mono labels: JetBrains Mono 10-11px, weight 500, uppercase, letter-spacing 0.08em
- Hero narrative: Newsreader 21px italic, weight 300
- Section titles: Newsreader 22-28px, weight 400
- Padding: 80px top (hero), 72px bottom (briefing pages), 56px between sections

**Color Tokens (ADR-0076):**
- Paper: cream `#f5f2ef`, linen `#e8e2d9`, warm-white `#faf8f6`
- Desk: charcoal `#1e2530`, ink `#2a2b3d`, espresso `#3d2e27`
- Spice: turmeric `#c9a227`, saffron `#deb841`, terracotta `#c4654a`, chili `#9b3a2a`
- Garden: sage `#7eaa7b`, olive `#6b7c52`, rosemary `#4a6741`, larkspur `#8fa3c4`
- Semantic: text-primary (charcoal), text-secondary (#5a6370), text-tertiary (#8a919a)

### Content & Layout Principles

**Conclusions Before Evidence:**
Hero section sets the frame. The first thing you read is the synthesis, not the data. Data is available on demand below the fold.

**Prose Over Bullets:**
Sections like "Current State" or "Why This Meeting Matters" are prose paragraphs, not bullet lists. Compound facts are bolded. The reading experience matters.

**Temporal Hierarchy (Not Entity Hierarchy):**
The actions list groups by when (overdue → today → this week → upcoming), not by who. The daily briefing features the most important meeting. The weekly forecast organizes by day-of-week and theme, not just listing all events.

**Tapering Density:**
Content deepens early (featured meeting gets full editorial treatment), then tapers. Overdue actions are visually heavy (500 weight, full context, terracotta). Upcoming items are whispers (300-400 weight, minimal context, tertiary text). Momentum and reading flow.

**Finite Documents:**
Every page has an explicit end. Daily briefing ends with `* * *` and "You're briefed. Go get it." Weekly forecast ends with `* * *` and "Plan your week." Actions list ends with a status footer. No infinite scroll. When you've read it, you know.

**One Synthesized Frame:**
Instead of "3 overdue, 2 missing agendas, 1 needs follow-up," the system says "You're prepared for 4 of 7 external meetings" (one number, one meaning). Many signals combine into one insight.

**"Why Now?" on Every Item:**
Every action, meeting, or thread includes temporal context. "Review QBR deck — the meeting is Thursday and the renewal decision happens this quarter." Not just "Review QBR deck."

### Page-Specific Hierarchies

**Daily Briefing:**
1. Hero (narrative synthesis of today)
2. Focus (one turmeric-bordered focus statement)
3. Featured meeting (lead story: title, metadata, narrative prep, "Before this meeting" action box)
4. Schedule (compact rows, entity accent borders)
5. Loose Threads (actions + emails interwoven, "why now?" context)
6. End mark (`* * *`)

**Account Detail:**
1. Hero (account name, executive assessment)
2. Current State (prose paragraph)
3. Key Risks (compact rows, severity dots)
4. Recent Wins (compact rows, sage dots)
5. Stakeholder Map (name/title/role/last contact, recency colors)
6. Recent Activity (timeline, reverse chronological)
7. Upcoming (meetings/decisions)
8. Actions (linked to this account)
9. End mark (`* * *`)

**Meeting Intelligence:**
1. Hero (account name, meeting title, strategic frame)
2. Meeting Metadata (logistics strip)
3. Why This Meeting Matters (prose)
4. Key Participants (name/role/intelligence, last contact)
5. Talking Points (numbered, specific, intelligence-backed)
6. Risks to Navigate (compact rows)
7. Recent History (timeline leading to this meeting)
8. Linked Actions (before/during)
9. Desired Outcomes (what success looks like)
10. End mark (`* * *`)

**Actions List:**
1. Hero (synthesized headline: "3 need you today")
2. Overdue (terracotta, heavy typography, full context)
3. Due Today (turmeric, 500 weight, full context)
4. This Week (neutral, 400 weight, brief context)
5. Upcoming (faded, 300-400 weight, minimal context)
6. Status footer (no `* * *`, living list)

**Weekly Forecast:**
1. Hero (week number, synthesized headline: "A renewal week")
2. Priority (turmeric focus block)
3. Week Shape (prose + day-by-day rows)
4. Readiness (synthesized statement + external meeting prep status)
5. Key Accounts (3-4 accounts active this week, prose per account)
6. Action Forecast (summary + grouped by urgency)
7. Week Health (portfolio-level synthesis prose)
8. End mark (`* * *`)

---

## Consequences

### Frontend Impact (High)

**Component Refactoring Required:**
- Folio bar (new shared component, replaces header)
- Floating nav island (new shared component, replaces sidebar)
- Atmospheric wash (new shared element, layered behind all content)
- Asterisk watermark (new shared element)
- Section rules & margin grid (new layout pattern, replaces card-based layout)
- Hero section (new shared pattern, replaces greeting/stats row)
- Loose threads / compact rows (refactored from existing list patterns)

**Page Rebuilds:**
- Dashboard: rewrap existing schedule/actions/emails in new layout
- List (actions): reorganize by temporal hierarchy instead of entity
- Account detail: new page, built from entity intelligence data
- Meeting intelligence: new page, built from meeting prep data
- Weekly forecast: new page, built from weekly synthesis data

**CSS System:**
- Extract inline CSS from mockups into shared design token file (`src/styles/design-tokens.css` or similar)
- Create component stylesheets for folio, nav island, section patterns
- Update Tailwind config to use design tokens (or migrate to CSS custom properties)

**No Data Model Changes:**
The backend data (meetings, actions, accounts, intelligence JSON) stays the same. The UI is a new view into existing data.

### Backend Impact (Minimal)

**Queries may need optimization:**
- Dashboard may need a "featured meeting" query (most recent + prep status)
- Actions list may need temporal grouping queries (overdue, due today, due this week)
- Account detail and meeting intelligence pages consume existing intelligence JSON data

**No schema changes required.** Existing SQLite tables, briefing JSON artifacts, and entity intelligence files work as-is.

### Product Impact (Medium)

**Visual Language Becomes Consistent:**
Currently, brand identity (ADR-0076) is documented but not lived in the UI. After this redesign, every page feels like part of the same publication. Consistent atmosphere, consistent chrome, consistent typography.

**Reading Experience Improves:**
Switching from dashboards to editorial layout changes how users consume information. Less "I need to scan this board for the one important thing" and more "I'm reading a briefing that prioritizes for me."

**No Feature Changes:**
No new capabilities. All existing workflows (briefing delivery, action tracking, meeting prep) work exactly the same. The UI is a reskin that honors the brand.

### Implementation Path

**Phase 1: Shared Chrome (I1):**
- Build FolioBar component
- Build FloatingNavIsland component
- Add atmosphere div + watermark div to layout
- Apply design tokens globally

**Phase 2: Dashboard Redesign (I2):**
- Rewrap schedule, focus, featured meeting, actions in new layout
- Test live data flowing through new components

**Phase 3: List Page & Actions Redesign (I3):**
- Refactor list page to flat rows (not cards)
- Refactor actions page to temporal grouping

**Phase 4: Entity Pages (I4):**
- Build Account detail page from account intelligence
- Build Meeting intelligence page from meeting prep data

**Phase 5: Weekly Forecast (I5):**
- Build weekly forecast page from weekly synthesis data

**Phase 6: Polish & Testing (I6):**
- Aesthetic review (typography, spacing, colors in context)
- Performance testing (new layout, new components)
- Edge case handling (long titles, many attendees, etc.)

---

## Reference

**Mockups:**
- `/Users/jamesgiroux/Desktop/dailyos-magazine-layout.html` (Daily briefing)
- `/Users/jamesgiroux/Desktop/dailyos-account-detail.html` (Account detail)
- `/Users/jamesgiroux/Desktop/dailyos-meeting-intel.html` (Meeting intelligence)
- `/Users/jamesgiroux/Desktop/dailyos-actions-list.html` (Actions list)
- `/Users/jamesgiroux/Desktop/dailyos-weekly-forecast.html` (Weekly forecast)

**Related ADRs:**
- ADR-0076: Brand Identity (color system, typography, brand mark)
- ADR-0073: Typography & Spacing Rules
- ADR-0055: Schedule-First Dashboard Layout (superseded by this decision for magazine layout philosophy)

**Design Principles:**
- P1: Zero-Guilt by Default
- P2: Prepared, Not Empty
- P3: Buttons, Not Commands
- P7: Consumption Over Production (this decision operationalizes P7)
