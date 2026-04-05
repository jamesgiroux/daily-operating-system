# I549 — Composable Report Slide Templates + Report Mockups

**Version:** v1.0.0 (Phase 4)
**Priority:** P1
**Area:** Design / Reports / Mockups
**Dependencies:** I547 (Book of Business), I397 (report infrastructure), I548 (inline editing consistency)
**Spec required:** Yes — multiple report types, composable design system, pencil mockup coordination

---

## Problem

DailyOS has 7 report types (Account Health, EBR/QBR, SWOT, Weekly Impact, Monthly Wrapped, Risk Briefing, Book of Business), each built independently with inline styles and ad-hoc layout patterns. They share visual DNA — Newsreader headings, DM Sans body, JetBrains Mono data, editorial section rules, the same color palette — but there's no shared visual vocabulary for report-specific patterns like stat strips, data tables, quote blocks, or status rows.

This creates two problems:
1. **Inconsistency** — each report invents its own version of common patterns (bulleted sections, metric displays, status badges)
2. **Design debt** — no reference mockups exist for reports in the pencil file, making it hard to evaluate and iterate on the visual language

Reports are the primary output artifact of DailyOS — the thing users share with leadership, bring to EBRs, and use to tell stories about their accounts. They deserve the same design rigor as the core pages.

---

## Solution

### Part 1: Composable Slide Template Components (Pencil)

Create reusable `.pen` components in the existing `dailyos.pen` mockup file that represent the building blocks of any report. These are design-system-level primitives, not page-specific layouts.

| Component | Description | Used By |
|-----------|-------------|---------|
| **Report Header** | Report title (Newsreader 32px), entity name, date label (mono), accent bottom border | All reports |
| **Narrative Block** | Large serif paragraph (Newsreader 18px italic, 1.65 line-height) for executive summaries and editorial prose | Account Health, EBR/QBR, BoB, Weekly Impact |
| **Stat Vitals Strip** | Horizontal row of 3-5 key metrics: large serif value + mono uppercase label beneath. Separated by thin vertical rules | Weekly Impact, Monthly Wrapped, BoB |
| **Bulleted Section** | Chapter Heading + bulleted list with `›` markers. Sage left border for positive, terracotta for negative | Account Health, SWOT, Weekly Impact |
| **Quote Block** | Customer voice callout: 3px turmeric left border, italic serif text, cream tint background, source attribution in mono | Account Health, EBR/QBR |
| **Status Row** | Left-bordered item (3px) with status badge pill (sage/saffron/terracotta). Title + description + optional source citation | Account Health (risks), EBR/QBR (challenges), BoB (leadership asks) |
| **Data Table** | Column headers (mono uppercase) + data rows. Alternating linen/white row fills. Monospace for numeric values | EBR/QBR (metrics), BoB (snapshot, value delivered) |
| **SWOT Quadrant** | Colored top border (sage/terracotta/larkspur/chili), uppercase heading, bulleted items with optional source badges | SWOT |
| **Deep Dive Card** | Warm-white card with entity name (Newsreader 20px), status narrative, bulleted workstreams/risks, impact text | BoB |

**Placement:** Below existing reusable components in the component area (x≈6559, y≈1200+).

### Part 2: Full Report Page Mockups (Pencil)

Compose the slide templates into full report mockups, placed to the right of the existing Risk Briefing page. Each mockup uses the DailyOS page shell (Folio Bar, Nav Island, cream background, asterisk watermark) and demonstrates the report with realistic sample data.

| Report | Atmosphere | Key Sections |
|--------|-----------|--------------|
| **Account Health** | Turmeric | Report Header → Narrative Block (assessment) → Quote Block → Bulleted Section ×2 (working/struggling) → Status Row ×3 (risks) → Bulleted Section (actions) → Finis |
| **EBR/QBR** | Larkspur | Report Header → Narrative Block (exec summary) → Bulleted Section (quarter in brief) → Quote Block → Status Row ×3 (value delivered) → Data Table (metrics) → Status Row ×2 (challenges) → Narrative Block (roadmap) → Data Table (next steps) → Finis |
| **SWOT Analysis** | Olive | Report Header → Narrative Block (summary) → 2×2 grid of SWOT Quadrants → Finis |
| **Weekly Impact** | Eucalyptus | Report Header → Stat Vitals Strip (meetings/actions/headline) → Bulleted Section (priorities moved) → Bulleted Section (wins) → Narrative Block (what you did) → Bulleted Section (watch) → Finis |
| **Monthly Wrapped** | Eucalyptus | Report Header → Stat Vitals Strip (conversations/entities/people/signals) → personality card → moments → Narrative Block (hidden pattern) → three-word summary → Finis |
| **Book of Business** | Turmeric | Report Header → Stat Vitals Strip (accounts/ARR/at-risk/renewals) → Narrative Block (exec summary) → Status Row ×2 each (risks/opportunities) → Data Table (account snapshot) → Deep Dive Card ×3 → Data Table (value delivered) → Narrative Block (themes) → Data Table (leadership asks) → Finis |

**Placement:** Continuing the page row (x≈32800+, y=0), each page 1440px wide with standard gaps.

---

## Sample Data for Mockups

All mockups should use realistic but fictional data that demonstrates the editorial voice:

- **Account name:** "Meridian Partners" (turmeric entity)
- **Person names:** Sarah Chen (VP Customer Success), Marcus Rivera (Head of Engineering)
- **Quarter:** Q1 2026
- **Health score:** 72 (Stable)
- **ARR:** $2.4M
- **Metrics:** Meeting cadence 3/week, response time <4h, NPS 62

---

## Design Rules

1. **All components use design token colors** — no hardcoded hex except in the pencil file (which maps to the same palette)
2. **Typography follows the 3-font stack** — Newsreader (headlines, narrative), DM Sans (body, UI), JetBrains Mono (data, labels)
3. **Paper fills the page (80%+)** — Cream `#f5f2ef`, Linen `#e8e2d9`, Warm White `#faf8f6`
4. **Accent colors ≤15%** — Turmeric `#c9a227`, Sage `#7eaa7b`, Terracotta `#c4654a`, Larkspur `#8fa3c4`
5. **Section rules, not cards** — dividers + spacing = structure. Cards only for featured content (Deep Dive Card)
6. **ADR-0083 vocabulary** — no system terms in any mockup text
7. **Every report ends with Finis Marker**

---

## Acceptance Criteria

1. **9 reusable slide template components** exist in `dailyos.pen` as `reusable: true` nodes with `DailyOS/Report/` name prefix
2. Each template component uses the correct DailyOS typography (Newsreader, DM Sans, JetBrains Mono) and palette (paper grounds, spice/garden accents)
3. **6 full report page mockups** exist in `dailyos.pen`: Account Health, EBR/QBR, SWOT, Weekly Impact, Monthly Wrapped, Book of Business
4. Each report page mockup uses the DailyOS page shell (Folio Bar instance, Nav Island instance, cream background)
5. Report pages are composed from the reusable slide template components (instances, not copies)
6. All mockup text follows ADR-0083 vocabulary — zero instances of "entity", "intelligence", "enrichment", "signal", or "prep"
7. Each report page has an appropriate atmosphere watermark color matching the report type
8. All 6 report mockups end with a Finis Marker instance
9. Realistic sample data demonstrates the editorial voice (conclusions, not labels; narrative, not bullet dumps)
10. Slide templates are composable — a new report type could be assembled from existing templates without creating new components

---

## Out of Scope

- Frontend code changes (this is design/mockup work only)
- CSS module extraction or refactoring of existing report renderers
- New report types beyond the existing 6
- Interactive prototyping or click-through flows
- PDF export layout (separate concern, I302)
