# ADR-0073: Editorial Design Language

**Date:** 2026-02-13
**Status:** Accepted
**Applies to:** Sprint 27 (I221, I222, I223, I224) and all future UI work

## Context

DailyOS v0.7.x uses the right color palette (cream, charcoal, gold, sage, peach) and the right philosophical foundations (briefing > dashboard, consumption over production, conclusions before evidence). But the visual execution doesn't live up to the intent. The current UI is:

- **Noisy and condensed.** Tight spacing, dense card grids, many competing elements at equal visual weight.
- **Generic.** Could be any SaaS dashboard. Nothing about the visual design says "this is a personal briefing tool, not a database interface."
- **Choppy.** Content organized by boxes/borders/cards rather than by typographic rhythm. Sections don't flow — they sit in containers.

A mood board exercise (Feb 2026) identified a clear aesthetic direction from references including editorial calendar apps, material design explorations, and magazine-inspired mobile UIs. The consistent thread: **editorial calm** — interfaces that feel like receiving a well-art-directed document, not querying a database.

This decision codifies that direction as the binding design language for Sprint 27 and all forward UI work.

## Decision

### Design Principle: "A beautifully typeset daily briefing laid on a warm desk."

Every surface in DailyOS should feel like a document you receive, not a dashboard you operate. The user reads through it top-to-bottom and is done — finite, not infinite.

### 1. Typography as Architecture

Type does the structural work. Borders, card containers, and dividers are secondary or absent.

**Font stack:**
- **Headings:** Newsreader (serif) — editorial warmth, confident at large sizes
- **Body:** DM Sans (sans-serif) — clean, readable, neutral
- **Data/timestamps:** JetBrains Mono (monospace) — precision, technical grounding

**Scale (desktop):**
- Page headline: 40–44px, Newsreader, weight 400, letter-spacing -0.02em
- Section title: 22–26px, Newsreader, weight 400
- Card/item title: 19–20px, Newsreader, weight 400
- Body text: 15–16px, DM Sans, weight 300–400, line-height 1.65
- Meta/labels: 11–13px, DM Sans or JetBrains Mono, uppercase with letter-spacing for labels
- **The size jump between levels creates hierarchy without chrome.** If you need a border to tell sections apart, the type scale isn't working.

**Narrative voice:** Pages open with an AI-synthesized editorial statement ("Your Thursday is customer-heavy," "A back-loaded week with two renewal conversations"). This is the headline, not a greeting. It tells you the conclusion, then the page provides the evidence.

### 2. Warm Restraint — Color as State

The palette stays: cream (#f5f2ef), charcoal (#1a1f24), gold (#c9a227), sage (#7fb685), peach (#e8967a).

**What changes is how much appears at any given moment.**

- **Background:** Cream dominates. ~85% of pixels should be cream or white.
- **Gold:** Active/now state, customer meetings, primary accent. Used on 4px accent bars, priority numbers, active sidebar items. Never as a background fill on large areas.
- **Sage:** Success/calm/complete. Used in pills and small indicators.
- **Peach:** Attention needed/overdue. Used in pills, overdue checkbox borders, risk labels.
- **Charcoal:** App chrome (sidebar), primary text. The dark sidebar frames warm content.
- **Color budget:** No more than 10–15% of any viewport should carry accent color. If everything is colored, nothing communicates.

### 3. Breathing Room as Feature

Space is not wasted — it's the primary differentiator between "productivity dashboard" and "personal briefing."

**Concrete spacing:**
- Page padding: 56px top, 64px sides (desktop)
- Between sections: 48–64px
- Inside cards: 28–32px padding
- Between cards in a list: 16–20px gap
- Between action items: 18px (border-separated, not card-wrapped)

**Density rules:**
- Show 3–5 items per section above the fold, not 12–15
- Meeting cards show prep context inline (no "expand" interaction for the critical info)
- Action items are text rows with generous padding, not mini-cards
- Whitespace between sections should feel *intentional*, like paragraph breaks in a magazine article

### 4. Cards Are for Featured Content Only

Current state: everything lives in a card. New direction: **most content is typographic rows separated by spacing and thin dividers.** Cards (with background + shadow) are reserved for:

- Meeting cards (the primary content unit — these earn the card treatment)
- Priority items in the weekly briefing (numbered, featured)
- Signal cards in intelligence reports (wins/risks deserve visual containment)
- The focus callout (gold-tinted, left-bordered — a pull quote, not a card)

Everything else (action items, readiness rows, stakeholder rows, metadata fields, timeline items) is presented as **styled text rows** — no card wrapping, no background, no shadow. Thin 1px dividers or spacing alone.

**Card style when used:**
- White background on cream
- border-radius: 16px (soft, not sharp)
- box-shadow: `0 1px 3px rgba(26,31,36,0.04), 0 8px 24px rgba(26,31,36,0.06)` (subtle, not elevated)
- Hover: slightly deeper shadow for interactive cards

### 5. Dark Chrome, Warm Content

The app shell creates a gallery effect:

- **Sidebar:** Charcoal background, muted white text, gold highlight on active item
- **Content area:** Cream background, border-radius on the left edge where it meets the sidebar (16px)
- **Effect:** Content feels *presented* — like a warm page set against a dark desk. The sidebar is the frame, not a competing information surface.

### 6. Signature Organic Element (Future)

The mood board references include flowing abstract lines and organic decorative shapes. DailyOS should develop a signature organic element — possibly:
- A flowing line connecting timeline items
- Subtle organic blob shapes behind section dividers
- Abstract curves as page-level decoration at very low opacity (3–6%)

**Deferred to implementation.** Sprint 27 focuses on typography, spacing, and color restraint. The organic element is an enhancement once the foundation is solid.

### 7. Finite Briefing Pattern

Every briefing page (daily, weekly, intelligence report) ends with an explicit terminal marker:

- Centered, italic, Newsreader, muted color
- "You're briefed. Go get it." / "End of weekly briefing." / "Intelligence current as of today."
- This signals: you've read everything. There is no more. You're done.

This reinforces Principle 1 (Zero-Guilt) and Principle 7 (Consumption) — the briefing is finite, not an infinite feed.

### 8. Pills Over Badges

State indicators use pill-shaped tags (border-radius: 100px) with colored dots:
- `pill-sage` + dot: Ready / Complete / Good
- `pill-peach` + dot: Needs prep / Overdue / At Risk
- `pill-gold` + dot: Active / Renewing / Partial
- `pill-neutral`: Internal / informational (no dot)

Pills are 12px text, 5px 14px padding. Small, contained, purposeful.

## Reference Implementation

A full HTML mockup demonstrating this language across four surfaces lives at:
`~/Desktop/dailyos-design-exploration.html`

This mockup shows:
- **Daily Briefing:** Narrative headline → focus pull-quote → meeting cards with accent bars → action rows → finite ending
- **Intelligence Report:** Entity name at editorial scale → health metrics strip → executive assessment prose → signal card grid → stakeholder rows
- **Weekly Briefing:** Narrative headline → week shape visualization → numbered priority cards → meeting readiness table
- **Account Detail:** Breadcrumb → entity name at scale → assessment prose → two-column (timeline + sidebar metadata) → action rows

## Consequences

**Easier:**
- Surfaces feel cohesive and intentional, not assembled from generic components
- Typography hierarchy eliminates need for decorative chrome
- Reduced density makes content scannable and reduces cognitive load
- Dark/warm framing creates premium, personal-tool feel
- Finite briefing pattern reinforces zero-guilt philosophy

**Harder:**
- Newsreader font adds a web font dependency (~40KB)
- Low density means more scrolling on information-heavy days
- Removing card containers from most content requires careful spacing to maintain visual grouping
- Serif headings are an opinionated choice — some users may prefer all-sans

**Trade-offs:**
- Density vs. calm: we choose calm. Power users who want 20 items visible at once are not the target.
- Decoration vs. restraint: we choose restraint now, organic elements later.
- Information above the fold vs. reading experience: we choose reading experience. The narrative headline + focus strip provide the "executive summary" above the fold; details flow below.

## Applies To

- Sprint 27: I221 (Focus page), I222 (Weekly briefing), I223 (Entity lists), I224 (Entity details)
- Dashboard (daily briefing) — already partially aligned via ADR-0055, Sprint 27 refines
- Intelligence reports — entity detail pages
- All future surface work inherits this language
