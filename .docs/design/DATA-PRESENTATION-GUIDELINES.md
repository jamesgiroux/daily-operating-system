# Data Presentation Guidelines

**Last audited:** 2026-03-15

How to choose the right presentation for data in DailyOS.

---

## Decision Tree

```
Is this sequential items grouped by category?
  -> Lists with ChapterHeading groups

Is this featured or promoted content?
  -> Cards (ADR-0073: cards for featured content ONLY)

Is this tabular, multi-attribute data?
  -> Rows (EntityRow, list-row pattern)

Is this editorial narrative content?
  -> Sections with editorial typography (serif headlines, sans body)

Is this key-value metadata?
  -> VitalsStrip (horizontal) or refKey/refValue pairs (vertical)

Is this a single metric or stat?
  -> Stat lines (JetBrains Mono, inline)

Is this a stakeholder or people grid?
  -> StakeholderGallery (2-column grid with inline editing)

Is this a multi-slide report?
  -> Slide-deck layout with scroll-snap and chapter navigation

Is there no data?
  -> EditorialEmpty (for magazine pages) or EntityListEmpty (for list pages)
```

---

## Pattern Details

### Lists with ChapterHeading Groups

**Component:** `ChapterHeading` + item rows
**File:** `src/components/editorial/ChapterHeading.tsx`
**Where:** Actions (grouped by temporal period), WatchList (risks/wins/unknowns), entity detail chapters

`ChapterHeading` renders a heavy rule (`1px solid var(--color-rule-heavy)`) above a serif title (Newsreader, 28px, weight 400). Optional epigraph in italic serif below.

**Rules:**
- Each group gets a `ChapterHeading` with a descriptive title.
- Items within a group are text rows with thin dividers (`1px solid var(--color-rule-light)`), NOT cards.
- Lists should be scannable. No nested chrome.
- Optional `feedbackSlot` prop renders IntelligenceFeedback controls next to the title.

### Cards (Featured Content Only)

**When:** Meeting cards in briefings, priority items, signal cards, focus callouts
**Rule per ADR-0073:** Cards are reserved for featured/promoted content. Never use cards for regular list items.

**Visual treatment:**
- Background: Warm White on Cream
- Border-radius: `var(--radius-editorial-xl)` (16px)
- Shadow: `var(--shadow-md)`
- Cards should feel like magazine pull-outs, not data containers.

**Where cards are correctly used:**
- `BriefingMeetingCard` in daily briefing schedule
- `PostMeetingPrompt` capture overlay
- Featured callout blocks

**Where cards must NOT be used:**
- Entity list rows (use EntityRow)
- Action items in lists (use text rows with dividers)
- Regular content sections (use ChapterHeading + body text)

### Rows (EntityRow)

**Component:** `EntityRow`
**File:** `src/components/entity/EntityRow.tsx`
**Where:** Account list, project list, people list

A navigable row with accent dot (or avatar), name, optional subtitle, optional name suffix (badges), and right-aligned metadata slot.

**Visual treatment:**
- 14px vertical padding, 12px gap between dot and text.
- Bottom border: `1px solid var(--color-rule-light)` (except last item).
- Dot color varies by entity type or health status.
- Name in sans-serif (DM Sans), metadata slots in mono (JetBrains Mono).
- Entire row is a `<Link>` for navigation.
- Supports nested indentation via `paddingLeft` prop (for child accounts).

**Props:**
| Prop | Type | Purpose |
|------|------|---------|
| `to` | string | Route path |
| `params` | Record | Route params |
| `dotColor` | string | Accent dot color |
| `name` | string | Primary label |
| `showBorder` | boolean | Bottom divider |
| `nameSuffix` | ReactNode | Inline badges/tags |
| `subtitle` | ReactNode | Secondary line |
| `children` | ReactNode | Right-aligned metadata |
| `avatar` | ReactNode | Replaces accent dot |

**Rules:**
- Use for any navigable list of entities.
- Dot color should encode meaning (health, type, status).
- Keep rows scannable. One line for name, one for subtitle, metadata on the right.

### Sections (Editorial Typography)

**Where:** Entity detail page chapters, report slides, briefing narrative blocks
**Components:** `ChapterHeading`, `PullQuote`, `StateBlock`, `BriefingCallouts`

Editorial sections use the magazine typography hierarchy:

| Element | Font | Size | Weight | Purpose |
|---------|------|------|--------|---------|
| Chapter title | Newsreader (serif) | 28px | 400 | Section heading |
| Epigraph | Newsreader (serif) | 17px | 300, italic | Section summary |
| Body text | DM Sans | 14-15px | 400 | Paragraph content |
| Pull quote | Newsreader (serif) | 18px | 300, italic | Featured excerpt |
| Mono label | JetBrains Mono | 10-11px | 500-600, uppercase | Category labels |

**Rules:**
- Narrative content reads top-to-bottom, like a magazine article.
- Pull quotes (`PullQuote`) use a turmeric left border for emphasis.
- State blocks (`StateBlock`) present working/struggling or momentum/headwinds pairs.
- Every section wrapped in `.editorial-reveal` for scroll-linked fade-in.

### Key-Value Metadata

**Horizontal:** `VitalsStrip` / `EditableVitalsStrip`
**Vertical:** `refKey` / `refValue` inline style pairs

#### VitalsStrip (Horizontal)

**File:** `src/components/entity/VitalsStrip.tsx`
**Where:** Below entity detail heroes

Horizontal strip of metrics. Each vital is a label + value pair:
- Label: JetBrains Mono, 10px, uppercase, tertiary color.
- Value: DM Sans or JetBrains Mono (for numbers), 14px, with optional highlight color.
- Color-coded highlights: turmeric for ARR, saffron for health, olive for status, larkspur for relationship.

**EditableVitalsStrip** (`src/components/entity/EditableVitalsStrip.tsx`) adds inline editing per field type (see Interaction Patterns doc, section 1).

#### refKey / refValue (Vertical)

**Where:** `ActionDetailPage` and other detail pages with form-like metadata

Vertical key-value pairs using shared inline styles:

```typescript
const refKey = {
  fontFamily: "var(--font-mono)",
  fontSize: 10, fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.06em",
  color: "var(--color-text-tertiary)",
  width: 100, flexShrink: 0,
};
const refValue = {
  fontFamily: "var(--font-sans)",
  fontSize: 14,
  color: "var(--color-text-primary)",
  flex: 1,
};
```

**Rules:**
- Use VitalsStrip for entity hero metadata (horizontal, compact).
- Use refKey/refValue for detail page fields (vertical, form-like).
- All labels in JetBrains Mono uppercase. All values in DM Sans or JetBrains Mono (numeric).

### Stat Lines (Metrics)

**Where:** FolioBar readiness stats, coverage analysis strips, capacity lines

Single metrics rendered inline with mono typography:

- Font: JetBrains Mono, 10-12px, weight 500-600.
- Color: context-dependent (sage for positive, terracotta for warnings).
- Format: `"4/6 prepped"`, `"2 of 8 known stakeholders with defined roles"`.

**StakeholderGallery coverage strip:**
```tsx
<span className={css.coverageNumbers}>{engagedCount} of {totalKnown}</span>
<span className={css.coverageLabel}>known stakeholders with defined roles</span>
```

**Rules:**
- Use mono font for all numeric data.
- Color-code metrics by sentiment (sage = good, terracotta = attention needed).
- Keep stat lines terse. No full sentences.

### Stakeholder Grid

**Component:** `StakeholderGallery`
**File:** `src/components/entity/StakeholderGallery.tsx`
**Where:** Entity detail pages ("The Room" chapter)

2-column grid of stakeholder cards with:
- Avatar with ring indicator (larkspur ring for linked people).
- Editable name, role, and assessment text.
- Engagement badge (color-coded selector).
- Last-seen date from linked person data.
- Coverage analysis strip below the grid.
- "Your Team" strip for account team members.

**Rules:**
- Limit visible grid to 6 items with "Show N more" expansion.
- Linked stakeholders become navigable links to person detail pages.
- Unlinked stakeholders show hover-reveal "Create contact" action.

### Empty States

**Editorial pages:** `EditorialEmpty`
**File:** `src/components/editorial/EditorialEmpty.tsx`

Centered serif italic title + optional sans-serif description. Padding: 64px vertical.

```tsx
<EditorialEmpty
  title="No actions yet"
  message="Actions will appear here as they're captured from meetings."
/>
```

**Entity lists:** `EntityListEmpty`
**File:** `src/components/entity/EntityListShell.tsx`

Centered message with optional children (e.g., action buttons). Uses mono font for retry buttons.

**Report empty states:** Custom per report type. Centered with mono label, serif title, sans description, and generate button. See `AccountHealthPage` empty state for the pattern.

**Rules:**
- Always provide an empty state. Never show a blank page.
- Empty states should explain what will appear and how to populate the page.
- Use serif italic for the primary message (magazine aesthetic).
- Offer an action when possible (generate, import, create).

### Generating / Loading States

**Components:** `GeneratingProgress`, `EditorialLoading`, `EntityListSkeleton`, `Skeleton`

| State | Component | Where |
|-------|-----------|-------|
| Report generation | `GeneratingProgress` | Account Health, Risk Briefing, EBR/QBR, SWOT |
| Page loading | `EditorialLoading` or `Skeleton` | Entity details, lists |
| List loading | `EntityListSkeleton` | Account/Project/People lists |

`GeneratingProgress` shows phased progress with a timer, editorial quotes, and phase descriptions. Each report defines its own phases and quotes.

**Rules:**
- Use `GeneratingProgress` for AI-driven generation (10+ seconds).
- Use `Skeleton` or `EditorialLoading` for data fetching (< 3 seconds).
- Always show what's happening. No mystery spinners.

---

## Domain-Specific Presentation

### Intelligence Display

**Components:** `StateOfPlay`, `StateBlock`, `PullQuote`, `BriefingCallouts`, `IntelligenceQualityBadge`
**Where:** Entity detail pages, daily briefing, meeting detail

Intelligence data is presented through editorial narrative, never as raw data dumps.

**State of Play** (`StateOfPlay.tsx`): Two-column layout (working/struggling) on account detail pages. Each side renders bullet points from the intelligence assessment. Follows the magazine editorial aesthetic -- conclusions first, evidence beneath.

**Pull Quote** (`PullQuote.tsx`): The "one thing to know" insight. Turmeric left border, italic serif text. Used as the lead-in on meeting briefings and entity detail heroes.

**Intelligence Quality Badge** (`IntelligenceQualityBadge.tsx`): Shows quality level of meeting intelligence using product vocabulary:
- New (grey) -- sparse data
- Building (turmeric) -- developing
- Ready (sage) -- sufficient data
- Updated (sage + dot) -- fresh data

**Briefing Callouts** (`BriefingCallouts.tsx`): Signal-driven callout boxes within briefing content. Highlight new information, risks, or opportunities that the user should notice.

**Rules:**
- Intelligence is always presented as editorial narrative, not raw JSON or field dumps
- Use product vocabulary (ADR-0083): "briefing" not "intelligence", "insights" not "signals"
- The hero/pull quote gives the synthesis; the page body provides the proof
- Quality labels must match the vocabulary table (New/Building/Ready/Updated)

### Health Score Visualization

**Components:** `HealthBadge`, `DimensionBar`, `VitalsStrip`
**Where:** Account detail pages, account list rows, dashboard

Health scores follow ADR-0097 ("One Score, Two Layers"):

**HealthBadge** (`shared/HealthBadge.tsx`): Compact badge showing overall health status with color coding:
- Healthy: sage green
- Monitor: turmeric/saffron
- At Risk: terracotta
- Critical: chili red

**DimensionBar** (`shared/DimensionBar.tsx`): Bar chart visualization for the 6 health dimensions. Each dimension gets a proportional bar in its category color.

**VitalsStrip health field**: In the vitals strip, health appears as a color-coded value with the HealthBadge component.

**Rules:**
- Health is always a color-coded status, not a raw number
- The LLM explains the score -- humans read the narrative, not the algorithm
- Health dimensions appear as supporting evidence beneath the headline score

### Timeline Formatting

**Component:** `UnifiedTimeline` (`entity/UnifiedTimeline.tsx`), `TimelineEntry` (`editorial/TimelineEntry.tsx`)
**Where:** Entity detail pages ("The Record" chapter)

The unified timeline merges meetings, emails, and captures into a single chronological stream:

- **Entity color accent**: 3px left border in entity color (turmeric for accounts, olive for projects, larkspur for people)
- **Date headers**: JetBrains Mono, 10px uppercase, grouped by date
- **Entry rows**: Event type icon + title + time + optional summary
- **Truncation**: Long timelines show recent entries with "Show earlier" expansion

**TimelineEntry** (`editorial/TimelineEntry.tsx`): Individual timeline row with entity color accent. Has CSS module.

**Rules:**
- Timeline always runs newest-to-oldest (most recent at top)
- Group entries by date using mono uppercase headers
- Use entity-specific accent colors for the left border
- Keep individual entries scannable -- one line for title, one for summary

### Email Display

**Where:** Emails page (`EmailsPage.tsx`), daily briefing attention section

**Email thread rendering:**
- Grouped by importance/entity
- Subject line in DM Sans 15px
- Sender and timestamp in JetBrains Mono 10px
- Entity chips (`EmailEntityChip`) for linked accounts/projects
- Signal extraction highlights (key phrases, sentiment)
- Dismiss button for individual emails (tracked via local `Set<string>`)

**Daily briefing email section:**
- Top 3-4 urgent emails surfaced in the Attention section
- Compact rendering: subject + sender + entity chip
- Links through to full Emails page

**Rules:**
- Email display is an intelligence surface, not a mail client
- Surface the "why this matters" (entity context, signal extraction), not just the message
- Emails in the briefing are curated (high-signal, capped count), not exhaustive

### Meeting Prep Format

**Where:** Meeting Detail page (`MeetingDetailPage.tsx`), daily briefing schedule

**Meeting briefing structure** (editorial, one-read format):
1. Key insight pull quote (the one thing to know)
2. Meeting metadata strip (time, duration, attendees, entity chips)
3. Before This Meeting: actions to complete, context to review
4. Risks: identified risks for this meeting
5. The Room: attendee grid with roles, engagement, assessment
6. Your Plan: AI-proposed + user agenda items

**Attendee rendering:**
- Name, role, organization in primary text
- Temperature badge (hot/warm/cool/cold)
- Engagement level selector
- Assessment summary (truncated with expand)
- Last-met date
- Link to person detail page if linked

**Post-meeting rendering:**
- Outcomes section appears at top for past meetings
- Summary, wins, risks, next actions
- `PostMeetingIntelligence` component for talk balance, key moments
- Transcript attachment button

**Rules:**
- Pre-meeting: conclusions first, evidence below. The user should know the key insight before reading any detail.
- Post-meeting: outcomes first (what happened), then the pre-meeting briefing below for reference.
- Attendee data uses the StakeholderGallery inline editing model.

### Action Queue Presentation

**Where:** Actions page (`ActionsPage.tsx`), entity detail "The Work" sections, daily briefing attention section

**Actions page tabs:**
- **Suggested**: Proposed actions from AI. Each row has accept/dismiss buttons via `ProposedActionRow`.
- **Pending**: Active commitments. Grouped temporally: Overdue (terracotta accent) / This Week / Later.
- **Completed**: Record of done work. Shows completion date.

**Action row rendering** (`shared/ActionRow.tsx`):
- Status toggle circle (left)
- Priority pill (P1/P2/P3) with priority-specific colors
- Title text (navigable link to detail page)
- Entity chip (linked account/project)
- Due date in mono font
- Source badge (where the action came from)

**Entity detail "The Work"** (`entity/TheWork.tsx`):
- Upcoming meetings for the entity
- Open actions linked to the entity
- Compact rendering (no tabs, no grouping)

**Daily briefing attention section:**
- 2-3 meeting-relevant or overdue actions
- Compact rendering with entity context

**Rules:**
- Overdue actions always use terracotta accent (attention needed)
- Priority pills use semantic colors: P1 = destructive, P2 = primary, P3 = muted
- Action titles are always navigable links to the action detail page
- Temporal grouping is the default organization (meeting-centric grouping deferred)

### Data Provenance

**Component:** `ProvenanceTag` (`ui/ProvenanceTag.tsx`)
**Where:** Entity detail pages

Source provenance indicator showing where data came from and its confidence level. Follows ADR-0098 source priority: User (4) > Clay (3) > Glean/Gravatar (2) > AI (1).

**Rules:**
- Provenance is informational, not interactive
- Higher-priority sources override lower-priority ones
- User corrections always have the highest confidence (1.0)
