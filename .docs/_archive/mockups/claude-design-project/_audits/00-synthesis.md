# Audit Synthesis — Reconciliation Plan

This is the deliverable that closes the audit phase and opens the population phase. It does three things:

1. **Surfaces decisions** the user needs to make before canonical entries land.
2. **Names the first wave** of canonical entries to write specs for, in priority order.
3. **Sequences the work** so v1.4.3 (Briefing) isn't blocked, with later waves following per version need.

Source audits: `01-app-inventory.md`, `02-account-detail-precedent.md`, `03-mockup-extraction.md`, `04-trust-ui-inventory.md`.

After decisions land, this document moves to `_archive/` and per-entry `.md` files take over as the source of truth.

---

## Decisions made — 2026-05-02

All eight decisions in Part 1 are resolved. Originals preserved below for context. Two were overrides; one was refined; five matched the original recommendation.

### D1 — Entity color aliases: **A (keep)** ✅
Add to canonical `tokens/color.md`. File Linear issue to migrate `src/styles/design-tokens.css` to consume them.

### D2 — Local nav: **Production `FloatingNavIsland` is canonical** ⚠️ override
Two distinct use cases were bundled in the original D2; separating them clarifies:

**Navigation** (where am I, where can I go) — production `src/components/layout/FloatingNavIsland.tsx` is canonical. It's a **dual-pill architecture**:
- **Global pill** (right): always-visible app navigation (Today / Week / Mail / Actions / Me / People / Accounts / Projects / Inbox / Settings)
- **Local pill** (left): chapter/section navigation that appears when the surface provides a `chapters` prop. Pills visually merge.

Mockup nav patterns (`DayStrip`, `SectionTabbar`) are **rejected as separate patterns** — surfaces provide chapters; `FloatingNavIsland` renders them. Three competing nav patterns collapse into one.

**Actions** (what can I do here) — `FolioActions` from the meeting mockup is a different concern: a toolbar of verbs (Copy / Share / Send Recap / Re-extract). Stays as a candidate pattern (Wave 4 or sooner if another deliverable surface needs it).

**Wave 1 impact:**
- Drop `DayStrip` (was entry #15). DailyBriefing provides chapters via `chapters` prop.
- `FloatingNavIsland` (entry #11) spec must document the dual-pill API, chapters contract, when the local pill appears, merge-rendering behavior, source: `src/components/layout/FloatingNavIsland.tsx`.
- Settings doesn't need `SectionTabbar`; its sections become chapters.

### D3 — Pill consolidation: **B (family of named primitives)** ✅
`Pill` is the visual primitive (with tone variants). Named primitives compose it: `EntityChip`, `TypeBadge`, `ProvenancePill`, etc. `Chip` (removable) is separate.

### D4 — `src/components/{shared,editorial}/` rename: **A (don't rename)** ✅
Document the implementation-folder vs. taxonomy mapping in `SYSTEM-MAP.md`. `data-ds-tier` attributes carry the semantic truth.

### D5 — Freshness/quality vocabulary: **Three distinct primitives** ⚠️ refines
v1.4.0 introduces multiple trust vocabularies. After review, **three are user-facing on Wave 1 surfaces** and each deserves its own primitive (god-component rejected; conflating freshness into trust-band rejected):

- **`TrustBandBadge`** (NEW per v1.4.0 substrate) — `likely_current / use_with_caution / needs_verification`. Wired to DOS-320 render surface filter. The user-facing judgment.
- **`IntelligenceQualityBadge`** (existing primitive in `src/components/entity/`) — `sparse / developing / ready / fresh`. Intelligence completeness — orthogonal to trust judgment. Document as-is. D-spine's `prep-state` consumes this with the right variant (don't invent a third primitive).
- **`FreshnessIndicator`** (NEW per v1.4.0 substrate) — timestamp + relative age (e.g., "as of 3h ago", "stale 5d"). Raw recency signal. Renders next to TrustBandBadge often but means something different.

**Receipts/inspection vocabularies** (`Resolved / ResolvedWithFlag / Suggestion / NoMatch` resolver bands; `ok / corrected / flagged` consistency states) defer to Wave 2 (v1.4.4 Trust UI) where the receipt experience is in scope.

**Wave 1 impact:** entry #6 (was "QualityBadge") splits into three entries — 6a `TrustBandBadge` (new), 6b `IntelligenceQualityBadge` (existing, document), 6c `FreshnessIndicator` (new). D-spine's `prep-state` swap when implementing means consuming `IntelligenceQualityBadge` with the right variant.

### D6 — `AccountViewSwitcher`: **Keep production as-is** ⚠️ override
Production has `AccountViewSwitcher` (bottom pill bar). Don't migrate to anything else. Document it as the canonical pattern for tri-view account switching. If we change it later, it goes through a versioned design-system update **after testing**, not as a casual reconciliation.

**Wave 1 impact:** drop the `DS-DECISION-06` Linear issue. `AccountViewSwitcher` becomes a canonical pattern in its own right (Wave 5 / surface pass). The mockup's `AccountTabs` masthead approach is rejected.

### D7 — `Dashboard` → `DailyBriefing` rename: **C (after v1.4.3)** ✅
Stays as Task #7. Don't bundle.

### D8 — `_shared/` substrate promotion: **B (after D1 lands)** ✅
Sequence: D1 token reconciliation PR → `_shared/` promotion to `.docs/design/reference/_shared/` PR.

---

## Wave 1 — updated entry list (post-decisions)

Reflects D2 (drop DayStrip, FloatingNavIsland is dual-pill production component), D5 (split QualityBadge into TrustBandBadge + IntelligenceQualityBadge), D6 (AccountViewSwitcher unchanged).

| # | Tier | `data-ds-name` | Source | Job |
|---|---|---|---|---|
| 1 | tokens | `color` (file) | `_shared/tokens.css` + production (post-D1) | Color tokens including entity aliases |
| 2 | tokens | `typography` (file) | `_shared/tokens.css` + production | Type families, scale, weights, line-heights |
| 3 | tokens | `spacing` (file) | `_shared/tokens.css` + production | Base unit + xs..5xl scale |
| 4 | tokens | `motion` (file) | `_shared/tokens.css` + production | Transitions, durations, animations |
| 5 | primitive | `Pill` | `_shared/.pill` | Generic pill with `tone` variants (D3) |
| 6a | primitive | `TrustBandBadge` | NEW per D5 + v1.4.0 contract (DOS-320) | likely_current / use_with_caution / needs_verification |
| 6b | primitive | `IntelligenceQualityBadge` | `src/components/entity/IntelligenceQualityBadge.tsx` | sparse / developing / ready / fresh; existing, document |
| 6c | primitive | `FreshnessIndicator` | NEW per D5 + v1.4.0 substrate (`source_asof`, freshness fallback) | Timestamp + relative age ("as of 3h", "stale 5d") |
| 7 | primitive | `ProvenanceTag` | `src/components/ui/ProvenanceTag.tsx` (already exists) | Source attribution label |
| 8 | primitive | `EntityChip` | D-spine `.entity-chip` + production refs | Entity reference with entity color (composes Pill per D3) |
| 9 | primitive | `TypeBadge` | `_shared/.type-badge` | Customer / Internal / Partner badge |
| 10 | pattern | `FolioBar` | `_shared/.folio` + chrome.js | Top frosted bar with crumbs + actions |
| 11 | pattern | `FloatingNavIsland` | `src/components/layout/FloatingNavIsland.tsx` (dual-pill production component, per D2) | Global app nav + optional local pill via `chapters` prop |
| 12 | pattern | `AtmosphereLayer` | `_shared/.atmosphere` | Page-tinted radial gradient |
| 13 | pattern | `MarginGrid` | `_shared/.margin-section` | Label + content layout (D-spine signature) |
| 14 | pattern | `ChapterHeading` | `src/components/editorial/ChapterHeading.tsx` + `_shared/.chapter` + variants | Section opener (heavy rule + serif title + epigraph) |
| ~~15~~ | ~~pattern~~ | ~~`DayStrip`~~ | **dropped per D2** | DailyBriefing provides chapters to `FloatingNavIsland` instead |
| 16 | pattern | `Lead` | D-spine `.lead-sentence` | Single-sentence headline |
| 17 | pattern | `DayChart` | D-spine `.day-chart` | Hour ticks + colored bars + NOW line |
| 18 | pattern | `MeetingSpineItem` | D-spine `.meeting` | Magazine-style meeting list item |
| 19 | pattern | `EntityPortraitCard` | D-spine `.acc-card` | Color-banded aside + lede + threaded events |
| 20 | pattern | `ThreadMark` | D-spine `.thread-mark` | Universal "talk about this" hover affordance |
| 21 | pattern | `AskAnythingDock` | D-spine `.ask-bar` | Multi-line input + suggestions + scope |
| 22 | surface | `DailyBriefing` | mockup D-spine + `Dashboard.tsx` (rename per D7 deferred) | Surface spec; documents which chapters it provides to `FloatingNavIsland` |

**Total:** 22 entries → 23 (DayStrip dropped; IntelligenceQualityBadge + FreshnessIndicator added).

## Linear issues — updated

Drop `DS-DECISION-06` (D6 closed: keep AccountViewSwitcher in production). Drop `DS-DECISION-02` rephrasing (D2 resolved: production FloatingNavIsland canonical, document chapters contract). Other 6 decision issues remain as **closed-with-resolution** records.

Add: `DS-XCUT-05: Document FloatingNavIsland chapters contract and surface adoption` — surfaces (DailyBriefing, Settings, etc.) need to define their chapters and consume the production component.

---

## Part 1 — Decisions

Eight decisions to make. Each has options + a recommendation. Numbered so you can respond by number.

### Decision 1 — Entity color aliases

> Audits 02 + 03

**Question:** Production removed `--color-entity-account`, `--color-entity-project`, `--color-entity-person`, `--color-entity-action`, `--color-entity-user`. Mockup `_shared/tokens.css` re-added them. Which is canonical?

**Options:**
- **A. Keep aliases.** Tokens express semantic intent (entity = identity), not just paint. Mockups depend on them; the entity-color-as-identity pattern (turmeric for accounts, larkspur for people) is encoded throughout D-spine.
- **B. Drop aliases.** Production was simpler. Components reference the underlying spice/garden tokens directly.

**Recommendation: A — keep aliases.** They give us a single name to change if we ever want to retint accounts. They also make `data-ds-tier="token"` entries semantically meaningful ("entity color" is a documented concept). Cost: a one-time PR to add them back to `src/styles/design-tokens.css` and migrate any direct `--color-spice-turmeric` references that mean "account."

**If A:** add to canonical `tokens/color.md`. File a Linear issue to migrate production.

---

### Decision 2 — Local nav coexistence

> Audit 03

**Question:** D-spine, Settings, and Meeting Detail use three different local-nav approaches (`DayStrip` / `SectionTabbar` / `FolioActions`). D-spine claims `DayStrip` *replaces* `FloatingNavIsland`. What's canonical?

**Options:**
- **A. Coexist with rules.** Four named patterns. App-level `FloatingNavIsland` always present. `DayStrip` only on time-scoped surfaces (briefing). `SectionTabbar` for long single-page surfaces. `FolioActions` toolbar for deliverable surfaces.
- **B. Single canonical pattern.** Pick one (probably `FloatingNavIsland`). Surfaces with deeper navigation needs solve them with breadcrumbs, scroll, or in-content links.
- **C. Replace FloatingNavIsland with a context-aware nav.** It morphs based on surface (DayStrip on briefing, sections on settings, etc.).

**Recommendation: A — coexist with rules.** The four patterns each solve a real problem. Rule of thumb: **keep `FloatingNavIsland` always present** on every surface, including briefing — it's the user's home base for app movement. `DayStrip` is *additional* time-scoped nav on briefing, not a replacement. The D-spine source comment (*"replaces the floating nav island"*) was a designer's wish; it loses the user's app-level wayfinding. Coexistence is a small visual cost for a large usability gain.

**If A:** four canonical patterns: `FloatingNavIsland`, `DayStrip`, `SectionTabbar`, `FolioActions`. `DailyBriefing` surface spec documents the coexistence rule explicitly.

---

### Decision 3 — Pill consolidation strategy

> Audits 02 + 03

**Question:** 7+ pill-shaped affordances exist across surfaces (`entity-chip`, `ask-chip`, `prep-state`, `now-tag`, `next-tag`, `s-chip`, `cur-pm-pill`, `_shared/.pill`, `type-badge`). What's the architecture?

**Options:**
- **A. One generic `Pill` primitive with tone variants.** Everything else is a one-off styling. Risks losing semantic meaning.
- **B. Family of named primitives sharing a visual base.** `Pill` (generic, tone variants) + `EntityChip`, `QualityBadge`, `TypeBadge`, `ProvenancePill`, `AskChip` as named primitives that compose `Pill` but carry semantic meaning.
- **C. Status quo.** Each surface ships its own.

**Recommendation: B — family of named primitives.** `Pill` is the visual primitive (`_shared/.pill` already does this well — 5 tones via `data-tone`). On top of that, named primitives carry meaning: `EntityChip` knows it's tied to an entity; `QualityBadge` knows it's intelligence quality; `TypeBadge` knows it's customer/internal/partner. Each named primitive renders a `Pill` underneath but adds props + `data-ds-name` so the inspector and grep work. A "removable Chip" (Settings) is a separate primitive — different interaction.

**If B:** `Pill` (primitive), `EntityChip` (primitive, composes Pill + accepts entity type), `QualityBadge` (primitive), `TypeBadge` (primitive), `ProvenancePill` (primitive), `Chip` (separate, removable). Total: 6 primitives.

---

### Decision 4 — `src/components/shared/` and `editorial/` taxonomy mismatch

> Audit 01

**Question:** Both folders are named "shared/editorial" but contain mostly **patterns** by our taxonomy (`HealthBadge`, `ChapterHeading`, `PullQuote`, `TimelineEntry`, `MeetingCard`, `MeetingRow`, etc.). Folder structure doesn't match canonical taxonomy. Rename?

**Options:**
- **A. Don't rename folders.** Document the taxonomy mapping in surface specs and `data-ds-tier` attributes. Folder structure stays; semantic structure lives in markdown.
- **B. Rename gradually.** Move files to `src/components/{primitives,patterns}/` over multiple PRs. High churn for marginal correctness.
- **C. Rename in one big PR.** Atomic but disruptive.

**Recommendation: A — don't rename folders.** Folder structure is implementation detail; the taxonomy is what matters for the design system. `data-ds-tier="pattern"` on `<HealthBadge>` is what makes it a pattern, not its folder. Rename is high-risk for low value. Folder names like `shared/` and `editorial/` are even semantically informative for *implementation* even if not for taxonomy.

**If A:** document the mapping in `.docs/design/SYSTEM-MAP.md` (a "implementation folders vs. taxonomy" subsection) and ensure each promoted entry's `Source` field cites the correct file.

---

### Decision 5 — `PrepStateChip` ↔ `IntelligenceQualityBadge` consolidation

> Audits 03 + 04

**Question:** Same vocabulary (`ready / building / new / sparse`) appears in two places: D-spine's `prep-state` chip and `src/components/entity/IntelligenceQualityBadge.tsx`. One consolidated name?

**Options:**
- **A. `QualityBadge`** — matches existing src naming, generic enough for non-prep contexts.
- **B. `PrepStateChip`** — matches D-spine intent, scoped to meeting prep state.
- **C. `FreshnessIndicator`** — matches Audit 04's candidate, broader trust framing.

**Recommendation: A — `QualityBadge`.** It already exists in src under that name. Variants: `ready / building / new / sparse / captured` (D-spine adds `captured`). Use it everywhere quality/freshness/prep-state surfaces (briefing meeting items, account detail, anywhere else).

**If A:** `QualityBadge` becomes a canonical primitive. `prep-state` from D-spine consumes it. `FreshnessIndicator` from Audit 04 candidates becomes a separate primitive (different visual + freshness-only scope).

---

### Decision 6 — `AccountViewSwitcher` migration

> Audits 02 + 03

**Question:** `AccountViewSwitcher` (production: bottom dark pill bar) vs. mockup `AccountTabs` (masthead). Audit 02 flagged this as production being mid-migration. Audit 03 surfaces `SectionTabbar` from Settings as a candidate canonical pattern. Migrate Account Detail to `SectionTabbar`?

**Options:**
- **A. Yes, migrate Account Detail to `SectionTabbar`.** Consolidates with Settings, removes the bottom-pill one-off.
- **B. Keep `AccountViewSwitcher` as a separate pattern.** Bottom pill has a real product reason (visible while scrolling, less competing chrome).
- **C. Defer — let Account Detail stay as-is, evaluate in v1.4.4 or later.**

**Recommendation: A — migrate, but as its own PR after `SectionTabbar` lands.** The bottom-pill is a one-off; consolidating saves a pattern. Order: (1) settle `SectionTabbar` spec from Settings, (2) wire it into Account Detail, (3) retire `AccountViewSwitcher`. Don't bundle with v1.4.3 work.

**If A:** file as a separate Linear issue under Design System project, blocking on `SectionTabbar` landing.

---

### Decision 7 — `Dashboard` → `DailyBriefing` rename timing

> Audit 04, NAMING.md

**Question:** `src/pages/Dashboard.tsx` is the canonical example of a name-vs-job mismatch. When?

**Options:**
- **A. Now, before v1.4.3 starts.** Removes mental overhead during the redesign.
- **B. As part of v1.4.3.** Bundle with the redesign PR.
- **C. After v1.4.3.** Avoid bundling; sequence as a separate small PR.

**Recommendation: C — after v1.4.3.** Renames are best done in isolation. Bundling with v1.4.3 makes the diff harder to review and risks delaying v1.4.3 if the rename hits an unexpected dependency. As a follow-up rename PR, it's small, easy to review, easy to back out.

**If C:** keep as Task #7 (already in our task list). Don't block v1.4.3 on it.

---

### Decision 8 — `_shared/` substrate promotion path

> Audits 02 + 03 (cross-cutting)

**Question:** `_shared/{tokens,primitives,chrome}.css + chrome.js` are largely good. When and how do they get promoted to `.docs/design/reference/_shared/` as the canonical reference render substrate?

**Options:**
- **A. Promote as-is now.** Risks codifying the entity-alias divergence and the mockup-side primitive/pattern misclassification.
- **B. Promote after Decision 1 (entity aliases) lands.** Update `_shared/tokens.css` to match production + canonical entity tokens, then move.
- **C. Don't promote the files; rewrite from scratch.** Highest cost; only worth it if existing files have load-bearing problems.

**Recommendation: B — promote after Decision 1.** The substrate is good enough to keep. Update tokens.css to match the canonical token decisions, then move the four files into `.docs/design/reference/_shared/`. Leave `primitives.css` largely as-is (the mis-classification is more about *naming* than *content* — a `.hero` rule is fine; we just call it a pattern in markdown). Chrome.css and chrome.js promote unchanged.

**If B:** sequence: Decision 1 → token reconciliation PR → `_shared/` promotion PR.

---

## Part 2 — First wave of canonical entries (for v1.4.3 Briefing)

These are the entries to write `.md` specs for **before v1.4.3 implementation starts**. Each row has the canonical name (pascal case, becomes both filename and `data-ds-name`), tier, source, and one-line job.

| # | Tier | `data-ds-name` | Source | Job |
|---|---|---|---|---|
| 1 | tokens | `color` (file) | `_shared/tokens.css` + `src/styles/design-tokens.css` (post-Decision 1) | Color tokens including entity aliases |
| 2 | tokens | `typography` (file) | `_shared/tokens.css` + production | Type families, scale, weights, line-heights |
| 3 | tokens | `spacing` (file) | `_shared/tokens.css` + production | Base unit + xs..5xl scale |
| 4 | tokens | `motion` (file) | `_shared/tokens.css` + production | Transitions, durations, animations |
| 5 | primitive | `Pill` | `_shared/.pill` | Generic pill with `tone` variants |
| 6 | primitive | `QualityBadge` | `src/components/entity/IntelligenceQualityBadge.tsx` + D-spine `.prep-state` | ready/building/new/sparse/captured |
| 7 | primitive | `ProvenanceTag` | `src/components/ui/ProvenanceTag.tsx` (already exists) | Source attribution label |
| 8 | primitive | `EntityChip` | D-spine `.entity-chip` | Entity reference with entity color |
| 9 | primitive | `TypeBadge` | `_shared/.type-badge` | Customer/Internal/Partner badge |
| 10 | pattern | `FolioBar` | `_shared/.folio` + chrome.js | Top frosted bar with crumbs + actions |
| 11 | pattern | `FloatingNavIsland` | `_shared/.nav-island` + chrome.js | App-level nav (always present, per Decision 2) |
| 12 | pattern | `AtmosphereLayer` | `_shared/.atmosphere` | Page-tinted radial gradient |
| 13 | pattern | `MarginGrid` | `_shared/.margin-section` | Label + content layout (D-spine signature) |
| 14 | pattern | `ChapterHeading` | `src/components/editorial/ChapterHeading.tsx` + `_shared/.chapter` + variants | Section opener (heavy rule + serif title + epigraph) |
| 15 | pattern | `DayStrip` | D-spine `.daystrip` | Time-scoped Y/T/T nav (briefing only, per Decision 2) |
| 16 | pattern | `Lead` | D-spine `.lead-sentence` | Single-sentence headline |
| 17 | pattern | `DayChart` | D-spine `.day-chart` | Hour ticks + colored bars + NOW line |
| 18 | pattern | `MeetingSpineItem` | D-spine `.meeting` | Magazine-style meeting list item |
| 19 | pattern | `EntityPortraitCard` | D-spine `.acc-card` | Color-banded aside + lede + threaded events |
| 20 | pattern | `ThreadMark` | D-spine `.thread-mark` | Universal "talk about this" hover affordance |
| 21 | pattern | `AskAnythingDock` | D-spine `.ask-bar` | Multi-line input + suggestions + scope |
| 22 | surface | `DailyBriefing` | mockup D-spine + Dashboard.tsx | Canonical surface spec; documents nav coexistence + section order |

22 entries to spec. Each uses `_TEMPLATE-entry.md`. Estimated ~30-60 min per entry depending on complexity. Total: ~15-20 hours of spec work, parallelizable across multiple sessions.

**For each spec:** name, status (proposed → canonical when v1.4.3 lands), `data-ds-name`, `data-ds-spec`, variants, tokens consumed, source, surfaces that consume it, history.

Note: `InferredAction` and `InferredActionPopover` are deferred to Wave 2 (v1.4.5 Salience & Recommendations). They're foundational *for v1.4.5*, not load-bearing for v1.4.3.

---

## Part 3 — Sequencing strategy

Four waves, sequenced by version need:

### Wave 1 — v1.4.3 Briefing substrate (NOW)
The 22 entries above. Spec these before briefing implementation starts. Most are existing code/CSS being documented + reconciled; none require new implementation work to *spec*.

### Wave 2 — v1.4.4 Trust UI (after Wave 1)
- `TrustBand` (pattern — composes `QualityBadge` + `ProvenancePill` + `FreshnessChip`)
- `ClaimRow` (pattern)
- `FreshnessChip` (primitive — distinct from QualityBadge)
- `SourceCoverageLine` (primitive)
- `ConfidenceScoreChip` (primitive)
- `VerificationStatusFlag` (primitive)
- `DataGapNotice` (primitive)
- `AsOfTimestamp` (primitive)
- `AboutThisIntelligencePanel` (pattern)
- `DossierSourceCoveragePanel` (pattern)
- `ReceiptCallout` (pattern — v1.4.4 inspection layer)
- `StaleReportBanner` (pattern)
- `ConsistencyFindingBanner` (pattern)

### Wave 3 — Settings redesign (parallel with Wave 2)
- `SectionTabbar` (pattern)
- `FormRow` (pattern — generic Row from settings/parts.jsx)
- `InlineInput` (primitive)
- `Switch` (primitive)
- `Segmented` (primitive)
- `RemovableChip` (primitive — distinguish from Pill)
- `GlanceCell` (primitive) + `GlanceRow` (pattern)
- `SurfaceMasthead` (pattern — generic from settings + meeting heroes)
- `Settings` (surface spec)

### Wave 4 — Meeting Detail redesign (after Wave 3)
- `FolioActions` (pattern)
- `MeetingHero` (or `DeliverableHero` — see if it generalizes from `SurfaceMasthead`)
- `MeetingStatusPill` (primitive)
- `AgendaThreadList` (pattern)
- `PredictionsVsRealityGrid` (pattern)
- `SignalGrid` (pattern)
- `EscalationQuote` (pattern)
- `FindingsTriad` (pattern)
- `ChampionHealthBlock` (pattern)
- `CommitmentRow` (pattern)
- `SuggestedActionRow` (pattern — already flagged in Audit 02)
- `RoleTransitionRow` (pattern)
- `MeetingDetail` (surface spec)

### Wave 5+ (deferred from this synthesis)
- v1.4.2 Account Detail-introduced patterns from Audit 02 (15 patterns including `ContextDossierChapters`, `StakeholderRoomGrid`, `ReferenceShapeGrid`, `WorkbenchCommitmentCard`, `WorkbenchSuggestionCard`, etc.) — fold these into Waves 2-4 where they reuse, keep the rest surface-internal until promoted.
- v1.4.5 Salience: `InferredAction`, `InferredActionPopover`, `ParkLink`, `WatchListRow`
- v1.4.6 Proactive: extensions to `AskAnythingDock`

---

## Part 4 — Linear sub-issues to file under Design System project

Proposed issues, grouped:

### Decisions (8 issues — one per decision, scoped for discussion)
- `DS-DECISION-01: Entity color aliases — keep or drop`
- `DS-DECISION-02: Local nav coexistence — four patterns with rules`
- `DS-DECISION-03: Pill consolidation — family of named primitives`
- `DS-DECISION-04: src/components/{shared,editorial}/ taxonomy mismatch — folder rename or doc-only`
- `DS-DECISION-05: PrepStateChip ↔ IntelligenceQualityBadge → QualityBadge`
- `DS-DECISION-06: AccountViewSwitcher → SectionTabbar migration`
- `DS-DECISION-07: Dashboard → DailyBriefing rename timing`
- `DS-DECISION-08: _shared/ substrate promotion path`

### Wave tracking (4 issues — each tracks the wave's spec writing)
- `DS-WAVE-1: v1.4.3 substrate — 22 canonical entries (spec writing)`
- `DS-WAVE-2: v1.4.4 trust UI — 13 entries`
- `DS-WAVE-3: Settings redesign — 9 entries`
- `DS-WAVE-4: Meeting Detail redesign — 13 entries`

### Cross-cutting work (4 issues)
- `DS-XCUT-01: Token reconciliation PR — production tokens + entity aliases (post-Decision 1)`
- `DS-XCUT-02: _shared/ promotion to .docs/design/reference/_shared/ (post-Decision 1+8)`
- `DS-XCUT-03: data-ds-* attribute migration into src/ components (gradual)`
- `DS-XCUT-04: Naming reconciliation track (Dashboard → DailyBriefing first; Audits surface more candidates)`

Total: 16 Linear issues. All under `Design System` project (no version, no target date — these run in tandem).

---

## Part 5 — Naming reconciliation candidates

Surfaced by audits beyond the `Dashboard → DailyBriefing` rename:

| Current | Canonical | Source | Status |
|---|---|---|---|
| `Dashboard.tsx` (file/component/route) | `DailyBriefing` | NAMING.md, Audit 04 | Top of queue |
| `AccountViewSwitcher` | (retire — replace with `SectionTabbar`) | Audits 02, 03 | Decision 6 |
| `WorkButton` | (retire — extend `Button` variants) | Audit 02 | Surface-internal cleanup |
| `AccountTypeBadge` | `TypeBadge` | Audit 02 | Promote to primitive |
| `AccountPullQuote` | (retire — extend `PullQuote` with freshness slot) | Audit 02 | Surface-internal cleanup |
| `IntelligenceQualityBadge` | `QualityBadge` | Audit 04, Decision 5 | Rename in place |
| `prep-state` chip class | `QualityBadge` consumer | Audit 03, Decision 5 | Class rename in mockup → component swap when implemented |
| `DailyBriefing.freshness` (received but unused / aliased `_freshness`) | (consume the prop) | Audit 04 | Bug, not rename — fix while implementing v1.4.3 |

---

## Part 6 — What stays in exploration (not promoted)

Not everything in mockups should become canonical. These stay in `.docs/mockups/` and don't get specs:

- **`TweaksPanel`** (Settings + D-spine) — design exploration tool, not a product surface. Useful for mockups; keep available; don't canonize as a product pattern.
- **`mockup-note`** (`_shared/primitives.css` `.mockup-note`) — annotation overlay; mockup-only.
- **`chrome-toggle`** (`_shared/chrome.css` `.chrome-toggle`) — mockup-only debug control.
- **D-spine briefing variations** (A-commander, B-one-thing, C-editorial-print, etc.) — superseded by D-spine; move to `mockups/_archive/` after v1.4.3 ships.
- **D-spine inferred-action `Bayesian` learning footer copy** — copy, not pattern; product decision.
- **All `cur-pm-*` prefixed classes from `meeting/_meeting.css`** — should NOT promote as-is. Consolidate into the canonical `MeetingDetail` patterns in Wave 4.

---

## Part 7 — Cross-cutting follow-ups

These touch multiple tiers and need coordinated work:

1. **Pill family architecture** (Decision 3) — `Pill` as visual primitive, named primitives compose it. Affects ~7 existing styles.
2. **`data-ds-*` attribute adoption in src/** — gradual; every promoted primitive/pattern gets attributes added when the spec lands.
3. **Token-source synchronization** — `src/styles/design-tokens.css` ↔ `_shared/tokens.css` ↔ canonical `tokens/*.md`. Needs ongoing discipline; CI check could enforce.
4. **Surface-spec authoring** — every surface gets a `surfaces/<Name>.md` documenting patterns consumed, local-nav choice, layout regions. Audit 02's `ThreeLensAccountSurface` and similar get filed there.
5. **Reference render seed** — once `_shared/` promotes (Decision 8), build `reference/{tokens,primitives,patterns}.html` showcase pages with the inspector loaded. This is the "see the system in browser" deliverable.

---

## Part 8 — What this synthesis does NOT decide

Explicitly out of scope:

- **Implementation order in `src/`.** The audits + this synthesis are about the design system contract. When v1.4.3 actually ships, the implementation order is its own planning document.
- **Versioning of individual entries.** Once entries land, they all start at design-system `v0.1.0`. Bumps come per change.
- **Storybook adoption.** Decided earlier (no, hand-rolled inspector). Revisit per the documented triggers.
- **Figma export pipeline.** Future work; needs `_shared/` promoted first as the export source.
- **Per-pattern API decisions.** The spec for each entry decides its own API; this synthesis just names the entries.

---

## Recommended next actions

1. **You respond by decision number** (1-8) — agree, override, defer.
2. After decisions: **file the 16 Linear issues** under Design System project (I can do this via MCP).
3. After decisions: **write Wave 1 specs** (22 entries). Can run in parallel via codex with bounded prompts (one entry per agent, simple template fill).
4. **Token reconciliation PR** runs in parallel with spec writing.
5. **Land Wave 1 specs + token reconciliation** before v1.4.3 implementation starts.

That's the plan.
