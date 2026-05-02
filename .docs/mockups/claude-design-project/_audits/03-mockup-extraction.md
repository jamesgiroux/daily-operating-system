# Audit 03 — Incoming Mockups: D-spine, Settings, Meeting Detail

Done directly via Read after two codex retries hung; produces a single combined report instead of the original three sub-audits. Cross-mockup matrix, local-nav reconciliation, and net-new pattern candidates included.

**Files read:**
- `mockups/briefing/variations/D-spine.html` (1940 lines, full)
- `mockups/surfaces/settings/{index,app,parts}` (substrate + structure; section JSX skipped — structure is clear from app.jsx)
- `mockups/meeting/current/after.html` (354 lines, full); `before.html` skipped (legacy comparison)
- `mockups/surfaces/_shared/{tokens,primitives,chrome.css, chrome.js}` (substrate baseline)

---

## Mockup-side `_shared` system today

`_shared/` is a **well-developed candidate design system**, not just shared CSS — and it's been seeded from production but with deliberate additions worth flagging.

### Tokens (`_shared/tokens.css`, 157 lines)

50+ tokens organized by family. Key delta vs production:

- **Adds entity color aliases** that production removed: `--color-entity-account`, `--color-entity-project`, `--color-entity-person`, `--color-entity-action`, `--color-entity-user`. Audit 02 already flagged this as **the** reconciliation gap. Decision needed: production drops them deliberately, or `_shared/` is right and production should re-add. The mockups depend on these aliases; without them, the entity-color-as-identity pattern doesn't work.
- **Includes `--color-desk-espresso`** (production has it too, but used here for entity differentiation)
- **Identical tint percentages** to production for the most part — the reconciliation mostly comes down to entity aliases + a small set of additional percentages

### Primitives (`_shared/primitives.css`, 469 lines)

The mockup library calls these "primitives" but by our taxonomy most are **patterns** (job-named, composed):
- `.hero` — pattern (composed of eyebrow + title + lede + parent-link)
- `.type-badge` (Customer/Internal/Partner) — primitive (variants on data attribute)
- `.vitals` (VitalsStrip) — pattern
- `.chapter` (ChapterHeading + epigraph) — pattern
- `.margin-section` (MarginGrid) — pattern (label | content layout)
- `.state-block` (working/struggling) — pattern
- `.pull-quote` + `.pull-quote-left` — primitive with two variants
- `.pill` — primitive (5 tone variants on `data-tone`)
- `.stake-card` (StakeCard) — pattern
- `.timeline-entry` — pattern
- `.action-row` — pattern
- `.add-row` (AddToRecord) — pattern

Of these, the **only true primitives**: `type-badge`, `pull-quote`, `pill`. The rest are patterns.

### Chrome (`_shared/chrome.css` + `chrome.js`, 403 lines)

Auto-injected by `chrome.js` based on `<body data-*>` attributes:
- `.atmosphere` (AtmosphereLayer) — page-tinted radial gradient, animated breathing
- `.folio` (FolioBar) — fixed top frosted bar with crumbs + actions
- `.nav-island` (FloatingNavIsland) — fixed right-side, primary app nav with 10 sections + 2 rules
- `.page` (PageContainer)
- `.hero-watermark`
- `.finis` (FinisMarker)
- `.chrome-toggle` (mockup-only control to hide chrome)

`chrome.js` reads body attributes (`data-folio-label`, `data-folio-crumbs`, `data-folio-actions`, `data-active-page`, `data-tint`) and injects the chrome — clean per-page configuration model.

**Verdict on `_shared/`:** strong substrate. Tokens need a one-time reconciliation decision (entity aliases yes/no). The "primitives" file is mis-classified per our taxonomy — most of it is patterns. Chrome is well-architected and should promote nearly as-is.

---

## Per-mockup extraction

### D-spine briefing

**Status:** chosen direction for v1.4.3. Most ambitious of the three.

**Notable: D-spine drifts from `_shared/`.** It inlines its own tokens at the top of the file rather than loading `_shared/tokens.css`. It introduces its own `.folio`, its own atmosphere, and **completely replaces FloatingNavIsland** with a DayStrip horizontal nav.

**Tokens introduced (not in `_shared/`):**
- `--color-text-quaternary: #9098a3` (a fourth text gray)
- `--color-rule-medium` (between heavy and light)
- Several extra tint percentages: `-7`, `-18` for turmeric

**Sections (top to bottom):**

| Section | Margin label | Job |
|---|---|---|
| Folio | — | Top frosted bar (custom: includes day-readiness counts inline) |
| DayStrip | — | Yesterday • Today • Tomorrow nav under folio (replaces FloatingNavIsland) |
| The Lead | "one sentence" | Big serif headline, optional inline `.sharp` highlight marker |
| Today (THE SPINE) | "4 meetings" | Section heading + summary + DayChart + meeting list |
| What's Moving | "3 entities" | Entity portrait cards with color-banded aside + lede + threaded items |
| Watch | "4 quiet" | Single-line tracked items with InferredAction or ParkLink |
| Ask | "anything" | AskAnythingDock — multi-line input + suggestion chips + scope footer |
| Briefing-end | — | Closer line, refresh status |

**Patterns introduced:**

- `DayStrip` — Yesterday/Today/Tomorrow horizontal nav with previews; **replaces FloatingNavIsland on this surface**
- `Lead` — one big serif sentence with optional `.sharp` highlight (turmeric marker)
- `DayChart` — hour-tick row + 110px-tall stacked bar chart with NOW line; bars colored by meeting type (customer/internal/oo/cancel/warn)
- `MeetingSpineItem` — Time column | Eyebrow (entity glyph + name + rule) + serif title + italic context + foot row (attendees · prep state · briefing link/create button); states: past/in-progress/upcoming/cancelled
- `EntityPortraitCard` — `.acc-card` with color-banded aside (state + name + foot stats) + main column (giant italic glyph + serif lede + threaded list of typed events)
- `EntityThreadList` — typed-dot list (mtg/act/mail/life) with when + what + optional thread-mark
- `WatchListRow` — who (mono) | what (serif) | InferredAction | thread-mark
- `InferredAction` — verb + confidence-marker dot + chev → popover with ranked alternatives ("Bayesian" learning footer)
- `ParkLink` — low-confidence alternative to InferredAction (just a dashed-underline "Park…")
- `AskAnythingDock` — multi-line: input row + suggestion chip row + scope footer (sources + write-back indicator)
- `ThreadMark` — universal "talk" hover affordance, appears on hover of any addressable line, opens Ask seeded with context
- `PrepStateChip` — ready/building/new/sparse/captured (mono) with colored swatch — **same vocabulary as `IntelligenceQualityBadge` from Audit 04**

**Tokens used:** all the standard families plus the D-spine-local additions noted above.

### Settings

**Status:** redesigned, awaiting its own version slot.

**Properly uses `_shared/`** — loads tokens, chrome, primitives via link tags. Uses chrome.js auto-injection. Adds `settings.css` (837 lines) for surface-specific styling.

**Built with React** (UMD bundle) — `app.jsx` + `parts.jsx` + 3 section files. Auto-saves on every change (no submit).

**Sections:** Identity, Connectors, Briefing & AI, Data, Activity, System, Diagnostics.

**Patterns introduced:**

- `SettingsMasthead` — eyebrow + h1 + lede + GlanceCells row (4 stat cards: Connectors / Database / AI today / Anomalies)
- `GlanceCell` — primitive: key (mono small caps) + value (with status dot + warn variant)
- `SectionTabbar` — numbered horizontal tabs (`01 You`, `02 Connectors`, …) with scroll-spy via IntersectionObserver, optional state dot for warning state
- `SectionHead` — eyebrow + h2 + epi + meta + action
- `Row` — label/help (left) | control (middle) | aux (right) — the universal settings row
- `InlineInput` — click-to-edit input with pencil affordance, mono/sans toggle
- `Switch` — toggle (aria-checked button)
- `Segmented` — tinted button group (canonicalize from the existing pill pattern?)
- `Chip` — removable tag with × (different visual treatment from `_shared/.pill`!)
- `Btn` — generic kind-variant button (defaults + per-section)
- `TweaksPanel` — meta/dev affordance, NOT product. Slide-in panel with sliders/radios/toggles for design exploration. Should be excluded from canonical product surfaces but kept available for mockup work.

**Local nav approach:** FloatingNavIsland (canonical app nav) **plus** in-page SectionTabbar with scroll-spy. Two layers, clear roles.

### Meeting Detail (`after.html`)

**Status:** redesigned (`before.html` is the legacy current state).

**Drifts from `_shared/`.** Loads its own `_meeting.css` (not read directly but inferred — 200+ classes prefixed `cur-pm-`, `cur-folio-`, `cur-chapter-`). All custom styling.

**Has a custom FolioActions sub-row** under FolioBar — a horizontal action toolbar (Copy, Share, Send Recap, Re-extract). This is a new pattern.

**Sections (top to bottom):**

| Section | Job |
|---|---|
| Folio | Crumbs + center timestamp + "processed" status |
| FolioActions row | Copy / Share / Send Recap / Re-extract (toolbar below folio) |
| Hero | Wrapped-X-min-ago status pill + overline + serif h1 + metadata line + one-paragraph synthesis |
| What Happened to Your Plan | Agenda thread tracking — items from briefing checked off post-meeting |
| Predictions vs. Reality | Two-column risks/wins comparison vs the briefing's predictions |
| Conversation | TalkBalanceBar + SignalGrid (4 stats) + EscalationQuote + competitor mentions |
| Findings | Three-column wins/risks/decisions with evidence quotes |
| Champion Health | Relationship arc — name + status + evidence + risk paragraph |
| Commitments & Actions | Yours/Theirs commitments + Suggested actions (accept/dismiss) + Pending |
| Role Changes | Name + before-status → after-status |
| Finis | Marker + processed timestamp |

**Patterns introduced:**

- `MeetingStatusPill` — "Wrapped 8 minutes ago · 56 min recorded" with green check / saffron processing state
- `MeetingHero` — status pill + overline + serif h1 + metadata line + one-paragraph synthesis
- `FolioActions` — sub-row of action buttons below FolioBar (Copy/Share/SendRecap/Re-extract)
- `AgendaThreadList` — planned items from briefing with confirmed (✓), open (○), new (+) states; carries-over with overdue highlighting
- `PredictionsVsRealityGrid` — two columns (risks / wins) comparing meeting outcome to briefing predictions
- `TalkBalanceBar` — proportional segments with name + percentage; **already exists in `src/components/shared/TalkBalanceBar.tsx`** ✓
- `SignalGrid` — 2x2 stats: Question density / Decision maker active / Forward-looking / Monologue risk
- `EscalationQuote` — highlighted attributed quote (large italic + attribution + timestamp)
- `FindingsTriad` — three-column Wins/Risks/Decisions cards with evidence quotes + attribution
- `ChampionHealthBlock` — name + status (still champion · weakening) + evidence quote + risk paragraph
- `CommitmentRow` — YOURS/THEIRS tag + text
- `SuggestedActionRow` — suggested-pill + title + meta + context quote + accept/dismiss controls
- `RoleTransitionRow` — name + before → after status pill chain

---

## Cross-mockup pattern matrix

Patterns appearing in 2+ mockups are highest-leverage promotion candidates. Patterns present in only one are surface-internal until proven otherwise.

| Pattern | D-spine | Settings | Meeting | Notes |
|---|---|---|---|---|
| **FolioBar** | custom | canonical | custom + actions row | drift — three variants |
| **FloatingNavIsland** | **REPLACED** | canonical | absent | major divergence on D-spine |
| **AtmosphereLayer** | inline | canonical | canonical | mostly aligned |
| **PageContainer** | margin-grid | full-width | full-width | three approaches |
| **MarginGrid** (label \| content) | yes (heavy) | partial | no | D-spine signature; settings uses for some sections |
| **Hero** | `.lead` (big sentence) | `.s-masthead` | `.cur-hero` | three different heroes — see reconciliation |
| **ChapterHeading** | `.section-heading` + `.section-summary` | `.s-section-head` (eyebrow + h2 + epi) | `.cur-chapter-title` | three variants of "section opening" |
| **FinisMarker** | `.briefing-end` | `.finis` (canonical) | `.finis` (canonical) | D-spine uses local, others use shared |
| **TalkBalanceBar** | no | no | yes | already in `src/components/shared/` |
| **PrepStateChip / IntelligenceQualityBadge** | yes (4 states) | no | no | matches `IntelligenceQualityBadge` in src |
| **Pill** (variants) | `.entity-chip` + `.ask-chip` + `.prep-state` + `.now-tag` + `.next-tag` | `.s-chip` | `.cur-pm-pill` | **lots of pill drift** — 7+ variants across surfaces |
| **Button** | `.create-btn` + `.brief-link` + `.folio-action` + various | `.s-btn` | `.cur-pm-btn` (+accept) | **lots of button drift** |
| **ThreadMark** | yes (universal hover) | no | no | D-spine unique; promote? |
| **InferredAction with popover** | yes | no | no | D-spine unique; foundational for v1.4.5 |
| **AskAnythingDock** | yes | no | no | D-spine unique; foundational for v1.4.6 |

---

## Local nav reconciliation

The user explicitly flagged this. Three completely different approaches across three surfaces:

### 1. D-spine — `DayStrip` (REPLACES FloatingNavIsland)

A horizontal nav under the FolioBar with **Yesterday | Today | Tomorrow**, optional preview text. Time-based, not section-based. Pulses a turmeric dot on "today."

The D-spine source comments are explicit: *"This is the only secondary chrome; replaces the floating nav island for page-level navigation."*

**Why it works:** briefing IS time-scoped — yesterday/today/tomorrow is the natural axis. FloatingNavIsland's "Briefing / Week / Inbox / Actions / Accounts / People / Projects / Me / Settings" doesn't help inside the briefing.

**Why it's a problem:** if every surface invents its own local-nav pattern (D-spine has DayStrip, Settings has SectionTabbar, Meeting has FolioActions toolbar), users learn three patterns. App-level FloatingNavIsland disappears entirely on briefing — that's the navigation home.

### 2. Settings — `FloatingNavIsland` + `SectionTabbar`

Settings keeps the canonical FloatingNavIsland for app nav, AND adds an in-page SectionTabbar (`01 You · 02 Connectors · 03 Briefing & AI…`) with scroll-spy. Two layers of nav with clear roles: app-level (right edge) and within-page (top tab bar).

**Why it works:** scroll-spy + numbered sections is honest about depth; you always know where you are in a long settings page.

### 3. Meeting Detail — `FolioBar crumbs + FolioActions row`

Meeting uses no in-page nav. FolioBar has breadcrumbs (Meetings / Meridian Harbor — QBR), and there's a separate `cur-folio-actions` toolbar row below FolioBar with action buttons (Copy, Share, Send Recap, Re-extract). The page is short enough not to need section nav.

**Why it works:** meeting detail is a single deliverable; section nav would be ceremony. Action buttons are surfaced as toolbar.

### Recommendation

**Three nav patterns can coexist if they're named, documented, and chosen on principle**:

| Pattern | When to use |
|---|---|
| `FloatingNavIsland` | App-level nav. **Always present** on stable surfaces. |
| `DayStrip` | Time-scoped surfaces where the day is the navigation axis. Briefing only? |
| `SectionTabbar` | Long single-page surfaces with discrete sections. Settings, possibly Account Detail (replacing the `AccountViewSwitcher` flagged in Audit 02). |
| `FolioActions` | Sub-row of action buttons below FolioBar. For "deliverable" surfaces (Meeting Recap, Reports). |
| (no in-page nav) | Short, single-purpose surfaces. |

**The contested decision:** does D-spine genuinely *replace* FloatingNavIsland, or does it *coexist* with it (keep nav-island for app movement, add DayStrip for time movement)?

**My read:** coexist. Removing FloatingNavIsland on briefing means users can't get to other parts of the app from their landing surface. But D-spine's argument that briefing is "time-scoped" deserves a response — maybe FloatingNavIsland collapses or hides on briefing while DayStrip is primary.

This is a decision for the user, not the audit. Surface it; don't pre-resolve it.

---

## Net-new patterns this batch introduces

Patterns that don't exist in `_shared/` or `src/` today and should be promoted as the v1.4.3 + Settings + Meeting design system contributions:

### From D-spine
- `DayStrip` — local nav (contested vs FloatingNavIsland)
- `Lead` — single-sentence headline with optional inline marker
- `DayChart` — visual day-shape (hour ticks + bars + NOW line)
- `MeetingSpineItem` — magazine-style meeting list item
- `EntityPortraitCard` — color-banded aside + lede + threaded events
- `EntityThreadList` — typed-dot timeline rows
- `WatchListRow` — passive items with InferredAction
- `InferredAction` (+ `InferredActionPopover`) — verb + confidence chev + ranked alternatives
- `ParkLink` — low-confidence alternative to InferredAction
- `AskAnythingDock` — multi-line input + suggestions + scope
- `ThreadMark` — universal "talk about this" affordance
- `PrepStateChip` — should consolidate with `IntelligenceQualityBadge` (Audit 04)

### From Settings
- `SettingsMasthead` (or generic `SurfaceMasthead`)
- `GlanceCell` (primitive) + `GlanceRow` (pattern)
- `SectionTabbar` — numbered scroll-spy tab bar
- `SectionHead` — universal section opener (consolidates with D-spine's `.section-heading`+`.section-summary` and Meeting's `.cur-chapter-title`)
- `SettingsRow` (or generic `FormRow`) — label/help | ctrl | aux
- `InlineInput`
- `Switch`
- `Segmented`
- `RemovableChip` (different visual from existing `Pill` — distinguish or consolidate)
- `TweaksPanel` — **dev/meta only**, exclude from product surface canon

### From Meeting Detail
- `MeetingStatusPill`
- `MeetingHero` (or generic `DeliverableHero`)
- `FolioActions` — toolbar sub-row below FolioBar
- `AgendaThreadList` — predicted items checked off
- `PredictionsVsRealityGrid`
- `SignalGrid` — 2x2 stats
- `EscalationQuote`
- `FindingsTriad`
- `ChampionHealthBlock`
- `CommitmentRow`
- `SuggestedActionRow` — Audit 02 also flagged this as a v1.4.2 pattern
- `RoleTransitionRow`

---

## Gap report

For each mockup: what reuses cleanly, what extends an existing primitive/pattern, what is genuinely new.

### D-spine
- **Reuses cleanly:** `_shared/.margin-section` (MarginGrid)
- **Extends:** `_shared/.pill` → adds `.now-tag`, `.next-tag`, `.entity-chip`, `.ask-chip`, `.prep-state` (5+ pill variants — should consolidate or become distinct primitives)
- **Genuinely new:** DayStrip, Lead, DayChart, MeetingSpineItem, EntityPortraitCard, EntityThreadList, WatchListRow, InferredAction, ParkLink, AskAnythingDock, ThreadMark
- **Drifts from `_shared/`:** inlines tokens, custom FolioBar, replaces FloatingNavIsland

### Settings
- **Reuses cleanly:** `_shared/tokens.css`, `_shared/chrome.css`, `_shared/primitives.css` (loads all three), chrome.js auto-injection
- **Extends:** the canonical chrome by adding in-page SectionTabbar
- **Genuinely new:** SettingsMasthead, GlanceCell/GlanceRow, SectionTabbar, SectionHead, Row, InlineInput, Switch, Segmented, RemovableChip, Btn
- **Drifts:** none structural; `RemovableChip` visual differs from `_shared/.pill` and should be consolidated

### Meeting Detail
- **Reuses cleanly:** TalkBalanceBar exists in `src/components/shared/`
- **Extends:** ChapterHeading concept → cur-chapter-title (different look)
- **Genuinely new:** MeetingStatusPill, MeetingHero, FolioActions, AgendaThreadList, PredictionsVsRealityGrid, SignalGrid, EscalationQuote, FindingsTriad, ChampionHealthBlock, CommitmentRow, SuggestedActionRow, RoleTransitionRow
- **Drifts from `_shared/`:** uses its own `_meeting.css` instead of `_shared/primitives.css`; all classes prefixed `cur-`

---

## Lessons across the three mockups

1. **`_shared/` was promised but not always honored.** D-spine and Meeting Detail both inline their own substrate. This is the same pattern Audit 02 flagged with the AccountViewSwitcher / mockup AccountTabs gap — mockups drift from each other when they should be drifting toward consolidation.

2. **"Hero" and "section heading" are reinvented every time.** Three different Hero treatments, three different ChapterHeading variants. Strong promotion candidates for a generic `SurfaceMasthead` and `SectionHead` with surface-specific variants.

3. **Pill / Button drift is severe.** D-spine has 5+ pill variants (entity-chip, ask-chip, prep-state, now-tag, next-tag), Settings has its own (s-chip, s-btn), Meeting has another set (cur-pm-pill, cur-pm-btn). These are exactly the "should be a primitive variant, isn't" cases Audit 02 saw with WorkButton.

4. **Local nav is a real architectural decision** — not a minor styling choice. Three different approaches across three surfaces means every new surface invents its own. Reconcile now.

5. **D-spine introduces three foundational patterns for future versions:** `InferredAction` (v1.4.5 Salience & Recommendations), `AskAnythingDock` (v1.4.6 Proactive Intelligence), `ThreadMark` (cross-version conversational affordance). These need to be settled in the design system before those versions ship.

6. **Audit 04's trust patterns are largely *absent* from these mockups.** D-spine's `prep-state` chip is the closest, and it matches `IntelligenceQualityBadge` from Audit 04. None of the three mockups uses `TrustBand`, `ProvenanceTag`, or `FreshnessChip` patterns — but trust-pattern *concepts* (briefing freshness, "no briefing yet", "briefing fresh", "captured") show up everywhere as ad-hoc copy. Real opportunity to weave the trust UI through the surfaces during build.
