# Account Detail Page — Content Design Principles
**Related Linear:** DOS-48 (parent IA redesign), DOS-113 (sentiment), new issues for Context and Work tabs
**Session date:** 2026-04-17
**Mockup:** `.docs/mockups/account-health-outlook-globex.html`
**Glean prompt:** `.docs/mockups/glean-prompt-health-outlook-signals.md`

---

## Context

DailyOS is a native Tauri app for CS managers. The account detail page has 3 JTBD tabs:
1. **Health & Outlook** — "Is this on track?" (design work **conceptually complete**)
2. **Context** — "What's the full context?" (content design **pending**)
3. **The Work** — "What are we doing about it?" (content design **pending**)

The shell, navigation, view switching, and section redistribution are shipped (DOS-111, DOS-47, DOS-112). This doc captures the **content design thinking** that applies to each tab's cards, density, and data sources.

---

## Core design principles (tab-agnostic)

### 1. The page is a real-time account state rollup, not a static dossier

Density is variable by design. Some days a card-heavy account page. Some days a single "On track — nothing new" confirmation. **Density IS the verdict.** This is fundamentally different from Gainsight/Vitally/ChurnZero — they show fixed dashboards; this shows newsworthy state.

Implication: every tab must have a "fine" state. Absence of cards should feel confident, not empty.

### 2. Triage-led, not inventory-led

Each tab asks its JTBD question and surfaces only what answers it. Cards EXIST when they're actionable. Sections collapse when they have nothing to say. Don't show the computed health dashboard when the user needs to know "is this fine?" — show the one or two things that aren't fine.

### 3. User sentiment is authoritative, not supporting

The user's assessment replaces the computed verdict as the headline. The machine's computed health becomes supporting evidence shown below. When they diverge, the page acknowledges the tension and invites the user to teach the system what it missed.

User sentiment is a **journal entry**, not a dropdown:
- Value (Strong / On Track / Concerning / At Risk / Critical)
- Optional note ("James seemed checked out, Jorge dominated architecture")
- Timestamp ("Set 12 days ago")
- Timeline (90-day sparkline of historical sentiment)
- Divergence flag (acknowledges mismatch with computed, invites detail)

Every note is training data for the Bayesian feedback loop.

### 4. Source attribution is transparency, not clutter

Every card shows:
- **Source tag**: charcoal "Local" (works without Glean) vs turmeric "Glean" (cross-source pattern detection) vs turmeric "Glean only" (can't exist without Glean)
- **Evidence citations**: dated, linked to source (Zendesk ticket, Gong call, transcript file, email ID)

This makes clear what the page can do without Glean (the floor) vs what Glean adds (enrichment).

### 5. Divergences are a distinct first-class card type

When the data says one thing and another data source (or the user) says another, surface the mismatch as its own card. Distinct visual treatment (saffron spine vs terracotta/turmeric). Divergences include:
- CRM vs reality (SF says "Disqualified" while everything else says active)
- Our pitch vs their authority (we assume consolidation scope they don't have)
- User gut vs computed (your read is earlier than the machine's)
- Channel sentiment divergence (tickets frustrated vs meetings cordial)
- Our narrative vs their requirements (Trust Center vs DORA/SOC 2)

Divergences are often the highest-signal content on the page.

### 6. The page should reveal our own data quality

Meta-cards about data capture gaps (missing Gong recordings, un-indexed Notion, hit-or-miss Staircase notes) are first-class content. The system reports on its own limitations. Users trust a tool that admits what it doesn't know.

---

## Health & Outlook — design summary (DONE conceptually)

**JTBD:** "Is this on track?"

**Information architecture (top to bottom):**
1. Shared header (name, badges, vitals, source coverage indicator)
2. Sentiment hero — elevated journal entry with value + sparkline + note + "still accurate?" + inline divergence flag when computed disagrees
3. **Needs attention** — triage cards ordered by urgency
4. **Divergences** — subgroup of cards where data/narrative mismatches
5. **Outlook: renewal** — confidence + peer benchmark + recommended start
6. Supporting: computed vs signal trend — demoted from headline, shows dimensions
7. Meta: "About this intelligence" — flags our own data capture gaps
8. Finis marker

**Card types (spine color):**
- Terracotta: urgent active friction (e.g. Defensive Mode blocking API)
- Turmeric: soon / strategic drift / expansion windows
- Larkspur: stakeholder changes
- Saffron: divergences (data/narrative mismatches)
- Charcoal (muted): meta / data quality notes

**Real Globex Holdings content demonstrates the approach:**
- 6 triage cards + 3 divergences = a representative "high activity" state
- For healthy accounts this collapses to 0-1 cards and an "on track" confirmation

---

## Outstanding UI refinements for Health & Outlook

User feedback from the last review (to address when reopened):

1. **Sentiment hero feels busy.** Went from a beautiful single-line prompt (unset state) to a dense block (set state with sparkline + note + divergence flag). Needs breathing room and visual hierarchy. Possible fix: collapse sparkline and divergence flag behind a disclosure — show on hover/click. Note should feel like pull quote, not inline metadata.

2. **Needs attention cards need more breathing room.** Current padding between triage cards is tight. Increase vertical rhythm, or add light separator texture.

3. **Divergences doesn't read as distinct chapter.** Currently uses same card shell as Needs Attention. Needs stronger "new section" signal — maybe offset left margin, different background tint, or wrap in a container with its own editorial framing. The saffron spine alone isn't enough.

4. **Section order question.** Current flow is "triage → divergences → outlook → supporting → meta." Consider:
   - Does outlook belong higher (before triage)? The renewal framing might set better context.
   - Supporting section (computed vs signal tension) is meaningful but buried. Worth surfacing?
   - Sequence should tell a story — "what's happening → what doesn't add up → where are we headed → what the numbers say → how trustworthy is this page"?

5. **"Wall of do-then-consume" feel.** The user said it's "not bad but we could do another once-over." The page is information-dense; good visual editorial rhythm matters more now than new content.

---

## The 10 gaps (from UX consultation)

These are data/intelligence gaps identified, ranked by impact. Five are addressed in Health & Outlook. The rest fit Context or Work.

| # | Gap | Where it fits | Status |
|---|-----|---------------|--------|
| 1 | Champion-at-risk signals (LinkedIn, tenure, sentiment trend, backup champion) | Health & Outlook triage | Partial (new tech lead card; no LinkedIn/tenure) |
| 2 | Product usage trend (not feature list) | Health dimension or card | Not yet addressed |
| 3 | Sentiment divergence across channels | Health divergence card | ✅ DONE in mockup |
| 4 | Transcript-extracted questions (churn/expansion adjacent) | Health triage cards | Partial (headless pricing card) |
| 5 | Similar-account renewal benchmark | Outlook section | ✅ DONE in mockup |
| 6 | Commercial signals (payment, discount, budget, procurement) | Outlook / Context | Deferred until Context design |
| 7 | Reference-ability / advocacy track | **Context tab** | Pending |
| 8 | Intra-account network map | **Context tab** | Pending (partial via stakeholder change card) |
| 9 | External market signals | Health (when material) | Not yet addressed |
| 10 | Quote wall (verbatim from transcripts) | **Context tab** | Pending |

---

## Glean prompt — leading signals enrichment

Full prompt at `.docs/mockups/glean-prompt-health-outlook-signals.md`. Tested live on Globex Holdings.

**What worked:**
- Evidence quality excellent — every signal dated, cited with ticket/call URLs
- Divergence detection exceptional — CRM status vs reality was a complete surprise
- Cross-source pattern recognition (infra drift across 4 tickets) is what justifies Glean over local
- Honest source-gap reporting (flagged its own Notion/Slack/LinkedIn indexing limits)

**What didn't land:**
- Schema drift — Glean preferred its own `signals[]` shape over our `champion_risk` / `product_usage_trend` / etc. fields
- Some categories weren't populated because data sources aren't indexed (LinkedIn, payments, advocacy)

**Integration plan:**
1. Add `entity_assessment.health_outlook_signals_json` migration
2. Call this prompt after main enrichment in `intel_queue.rs`
3. Post-process Glean's `signals[]` into our category buckets via Rust
4. Trend-based signals (usage, sentiment) use a separate PTY analysis pass: raw data → synthesized trend → DB field

---

## Context tab — design pending

**JTBD:** "What's the full context?"

**Current sections (moved from old page, ready to redesign content):**
- AccountPullQuote
- StateOfPlay + AccountTechnicalFootprint
- StrategicLandscape
- StakeholderGallery ("The Room")
- ValueCommitments
- UnifiedTimeline ("The Record")
- FileListSection

**Design questions for next session:**
1. Is this a dossier (static reference) or a narrative (story about who they are)?
2. How does it differ from Health & Outlook philosophically? Health is temporal; Context is… what?
3. Same triage approach, or a different IA entirely?
4. Which of the remaining gaps (reference-ability, network map, quote wall) fit here?

---

## The Work tab — design pending

**JTBD:** "What are we doing about it?"

**Current sections:**
- RecommendedActions + TheWork
- WatchList + WatchListPrograms
- AccountReportsSection

**Design questions for next session:**
1. Is this forward-looking (what we plan to do) or backward-looking (what we've done)?
2. How does it connect to Health & Outlook's "Needs attention" cards? (Probably should — a triage card resolved becomes work captured here.)
3. Role of Linear issues linked to this account?
4. Commitment tracking — ownership, due dates, status.
