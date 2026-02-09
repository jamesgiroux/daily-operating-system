# Weekly Briefing UX Research

**Date:** 2026-02-08
**Purpose:** Inform the week page redesign (ADR-0052 Phase 2+). The current implementation renders mechanical data as equal-weight Card panels — it reads like a dashboard retrofit, not a briefing. This research examines how other products solve the "start of your week" experience.

---

## The Problem

The week page has the right data (ADR-0052 delivered readiness checks, day shapes, expanded actions, account health). But it renders as four independent Card containers stacked vertically with equal visual weight. There's no narrative flow, no hierarchy, no connective tissue between sections. It feels like a Grafana panel — each section is a widget that could be rearranged without affecting the others.

The daily briefing learned this lesson: ADR-0055 killed the summary card because it narrated the interface, and deleted the ReadinessStrip because it was redundant aggregation. The week page needs the same reckoning, but the fix is bigger — it requires both a layout restructure and the narrative/priority AI enrichment (I94) to serve as the emotional anchor.

---

## Seven Patterns Worth Stealing

### 1. The Finite Briefing (Presidential Daily Brief + ChatGPT Pulse)

The PDB delivers 6-7 short articles + 2 deep dives. ChatGPT Pulse delivers 5-10 cards and explicitly says "That's all for today." Both create a reading experience with closure.

**Core principle:** The curation IS the value. Ruthless prioritization — intelligence experts condense all signals into a few pages. The briefing has a beginning (most important thing), a middle (secondary items), and an end.

**For DailyOS:** The six-section hierarchy in ADR-0052 (narrative, priority, readiness, shape, actions, health) already has this structure. The key is making the sections feel like a narrative arc, not a dashboard. The page should explicitly end — no infinite scroll, no "load more." When you've read it, you're briefed.

**Connects to:** Principle 7 (Consumption Over Production), Principle 9 (Show the Work, Hide the Plumbing).

### 2. The Tapering Word Count (Morning Brew)

Morning Brew's newsletter progressively shortens item length from top to bottom. The main article is ~390 words. "Tour de Headlines" items are 60-120 words each. Quick hits at the end are one-liners. This "accelerates reading speed and pushes the reader to the end."

**Core principle:** The reader builds momentum. Start with the deepest item, progressively shorten, finish with quick hits. Creates a sense of completion rather than exhaustion.

**For DailyOS:** The week narrative (2-3 sentences) is the longest prose. Top priority is medium. Readiness checks are short items. Week shape is visual. Actions are a list. Account health is compact alerts. The natural hierarchy already tapers — lean into it by removing Card wrappers that flatten the density gradient.

**Connects to:** Principle 7 (Consumption Over Production) — optimize for reading speed.

### 3. Conclusions Before Evidence (CQC Board Report Formula)

Board Intelligence's framework: Context ("why now?"), Questions (3-5 items, MECE), Conclusions (direct answers), Input sought (what decision is needed). The executive gets the conclusion first, then can dig into supporting data.

**Core principle:** Tell the user what things mean, don't show them data and expect them to conclude. "Your week is back-loaded: Monday-Wednesday are open, Thursday-Friday are packed with customer meetings" beats a bar chart.

**For DailyOS:** The week narrative should be the conclusion. Everything below it is evidence. The user should be able to stop reading after the narrative + top priority and still have 80% of the value. Every item below should include "why now?" context, not just the fact.

**Connects to:** Principle 6 (AI-Native, Not AI-Assisted) — the AI does the synthesis, not the user.

### 4. One Synthesized Frame (Oura Ring Readiness Score)

Oura collapses HRV, resting heart rate, body temperature, sleep, and activity into a single 0-100 "Readiness Score." You don't need to understand HRV to know "72 means take it easy today." The detail is available on demand but the synthesis is the default view.

**Core principle:** Many signals collapsed into one concept. The user reads one thing and knows where they stand.

**For DailyOS:** Not "3 overdue, 2 missing agendas, 1 stale contact" — instead: "You're prepared for 4 of 7 external meetings. Two need agendas. One customer hasn't been touched in 3 weeks." The readiness section should synthesize, not enumerate.

**Connects to:** Principle 2 (Prepared, Not Empty) — the system tells you your readiness posture.

### 5. Data as Identity, Not Statistics (Spotify Wrapped)

Spotify transforms "you played this song 200 times" into "this was your anthem." Statistics become identity statements. Huge text, minimal density — each screen has one fact, presented large.

**Core principle:** The data tells a story about who the user is, not what the numbers are. "3 overdue actions" becomes "You have three commitments you haven't closed — two are for Acme, and their QBR is Thursday."

**For DailyOS:** Action items should carry context that connects them to the user's professional relationships and upcoming moments. The data should feel personal, not administrative.

**Connects to:** Principle 10 (Outcomes Over Activity) — measure effectiveness, not engagement.

### 6. The Guided Ritual Arrives (Sunsama)

Sunsama's weekly planning is a 4-step guided flow that appears at a scheduled time. It's not a page you visit — it's a moment that arrives. Review last week, set objectives, journal, share.

**Core principle:** The weekly moment is a ritual, not a destination. It has temporal context (it arrives Monday morning) and emotional arc (review then plan then reflect).

**For DailyOS:** The weekly page should feel like opening a briefing document, not visiting a dashboard. Critically, DailyOS generates the synthesis (unlike Sunsama which asks the user to write it). The ritual is reading, not producing. The scheduler already triggers `/week` generation — the page should feel like receiving a document, not querying a database.

**Connects to:** The Prime Directive ("The system operates. You leverage"), Principle 1 (Zero-Guilt by Default) — the ritual is passive consumption with no guilt for skipping.

### 7. "Why Now?" Framing (CQC Board Formula)

Every item in an executive briefing explains its temporal urgency. Not "Review QBR deck" but "Review QBR deck — the meeting is Thursday and the renewal decision happens this quarter."

**Core principle:** Context makes items actionable. Without temporal framing, a list of tasks is just a list. With it, the user understands urgency without the system assigning priority numbers.

**For DailyOS:** The `context` field on actions (already in the data model) should surface in the week view. Overdue items especially need "why this matters now" reasoning. The AI enrichment (I94) should generate this context where it doesn't exist.

**Connects to:** Principle 2 (Prepared, Not Empty) — the user should understand not just what to do but why it matters this week.

---

## Anti-Patterns to Avoid

### The Stats Card Grid (our current state)

4-6 cards in a row, each showing one number with a label. "12 meetings" / "3 overdue" / "5 due this week." No context, no narrative, no "so what?" Cards create visual equality — everything demands the same attention. Numbers without context are meaningless ("3 overdue" could be fine or catastrophic).

### Gamification as Engagement

Streak counters, completion percentages, leaderboards. Violates Principle 1 (Zero-Guilt) and Principle 10 (Outcomes Over Activity).

### The Infinite Feed

No explicit end to the briefing. Continuous scroll. User must decide what matters. Violates Principle 7. ChatGPT Pulse solved this: "That's all for today."

### Calendar Duplication

Showing a calendar grid that Google Calendar already provides. DailyOS can't out-calendar Google Calendar. The value is intelligence, not display.

---

## Key Products Studied

| Product | What They Get Right | What They Miss |
|---------|-------------------|----------------|
| **ChatGPT Pulse** | Finite briefing (5-10 items), proactive overnight generation, "that's all" ending, lightweight curation feedback | New product, still evolving |
| **Reclaim.ai** | Calendar IS the briefing — already optimized when you arrive | No narrative layer, no readiness intelligence |
| **Motion** | Per-meeting intelligence briefs | Calendar-centric, credit-based, no weekly synthesis |
| **Linear Pulse** | AI-generated summary with audio digest option, three-tier relevance | Work context only, not personal productivity |
| **Sunsama** | Guided ritual with emotional arc, objectives not tasks | User produces the synthesis (we should generate it) |
| **Morning Brew** | Tapering word count, conversational tone, curated-not-original | Newsletter format, not interactive |
| **Oura Ring** | Single synthesized readiness score, personalized baselines | Health domain only |
| **Spotify Wrapped** | Data as identity statements, huge text minimal density | Annual not weekly, entertainment domain |

---

## Implications for Implementation

The ADR-0052 data architecture is validated — the six sections map well to these patterns. The problem is rendering, not data.

**Layout changes (no AI required):**
- Remove Card wrappers from sections that should feel like prose (narrative, readiness)
- Full-width narrative text as opening paragraph, no border/icon
- Taper visual density from top (prose) to bottom (compact lists)
- Readiness as inline synthesized text, not enumerated card items
- Week shape stays visual (density bars work) but without Card wrapper
- Page has explicit end — no open-ended scroll

**AI enrichment (I94):**
- `weekNarrative`: 2-3 sentences framing the week's shape and key moments
- `topPriority`: single item with "why now?" reasoning
- Action context: AI-generated reasoning connecting actions to upcoming meetings/relationships
- These are the emotional anchor — without them, the page opens with warnings instead of framing

**Both should land together.** The layout restructure without narrative still opens with readiness warnings (all problems, no framing). The narrative without layout restructure is prose trapped in a Card. They're complementary — the narrative needs the breathing room, and the layout needs the narrative to anchor it.
