# Primitive + pattern canonical-module promotion audit

DOS-375 Phase 1. Per-entry plan for promoting the spec'd primitives and patterns that currently lack canonical CSS modules.

## Summary

- Audited: 18 primitives + 31 patterns (49 total)
- Recommend extract: 14 primitives + 10 patterns = 24 entries (5 clusters)
- Recommend keep-embedded: 1 entry (TypeBadge)
- Already canonical (no action): 3 primitives + 21 patterns = 24 entries

---

## Promotion-needed primitives

### Pill

- **Spec:** `.docs/design/primitives/Pill.md`
- **Current CSS location:** No scoped CSS module exists anywhere in `src/`. Pill-shaped styling lives as scattered local variants:
  - `src/components/work/WorkSurface.module.css:247-305` — `.audiencePill`, `.audienceCustomer`, `.audienceInternal`, `.visibilityPill`, `.visibilityPrivate`, `.visibilityShared`
  - `src/components/ui/meeting-entity-chips.tsx` — inline `style={{ background, color, borderRadius, padding }}` objects (each chip rendered at ~lines 210-250)
  - `src/components/ui/email-entity-chip.tsx` — same inline style pattern (~lines 106-119)
  - No `Pill.tsx` or `Pill.module.css` exists in `src/`
- **Used in surfaces/components:** WorkSurface (audience + visibility pills), MeetingEntityChips, EmailEntityChip. Spec intends it for DailyBriefing, AccountDetail, MeetingDetail, Settings, ProjectDetail, PersonDetail.
- **Recommendation:** **Extract**
- **Rationale:** The spec calls this "the visual primitive for any inline status / label / category badge" — all named badge primitives compose it. 7+ drift variants already exist per the Pill.md History note. Without a canonical module every new surface reinvents the pill shape.
- **Extraction complexity:** **Moderate** — 5 tone variants × ~8 CSS props each ≈ 40 lines. Two consumers using inline styles need DOM-level refactor (removing `style={{}}` objects, applying className). WorkSurface needs its local pill variants removed in a follow-up.
- **Phase 2 cluster:** Cluster A — Universal Indicators

---

### EntityChip

- **Spec:** `.docs/design/primitives/EntityChip.md`
- **Current CSS location:** No CSS module. Both existing implementations use identical inline `style={{}}` color-map patterns:
  - `src/components/ui/meeting-entity-chips.tsx:47-57` — `entityColor` and `entityBg` const maps; applied inline per chip
  - `src/components/ui/email-entity-chip.tsx:29-38` — duplicate const maps; applied inline at ~lines 106-107, 114-119
- **Used in surfaces/components:** MeetingEntityChips (DailyBriefing schedule rows, MeetingDetail), EmailEntityChip (emails page)
- **Recommendation:** **Extract**
- **Rationale:** Two independent implementations already exist with identical color logic. DOS-357 already reconciled the entity-type token mapping. Extracting to `EntityChip.module.css` + a shared `EntityChip.tsx` removes the last inline-style drift and lets both consumers converge on one component.
- **Extraction complexity:** **Moderate** — ~15 lines CSS for 3 entity-type tones. Migration is a DOM change: both consumers must replace inline `style={{}}` color objects with CSS module class application. The `removable` and `editable` interaction variants in `meeting-entity-chips.tsx` must be preserved.
- **Phase 2 cluster:** Cluster B — Entity Primitives

---

### RemovableChip

- **Spec:** `.docs/design/primitives/RemovableChip.md`
- **Current CSS location:** No `src/` implementation exists. Mockup origin: `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx:73-80` (`Chip` component with inline styles). No CSS module or TSX component in production code.
- **Used in surfaces/components:** Not yet in production. Spec targets Settings (Wave 3) as canonical consumer.
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 3 primitive. Creating it canonical from the start avoids Settings inventing yet another local chip pattern. ~20 lines CSS, 1 initial consumer.
- **Extraction complexity:** **Trivial** — ~20 CSS rules for default, hover, focus-visible, disabled states. No migration needed; no existing consumers.
- **Phase 2 cluster:** Cluster A — Universal Indicators

---

### TrustBandBadge

- **Spec:** `.docs/design/primitives/TrustBandBadge.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 1 primitive; planned at `src/components/ui/TrustBandBadge.tsx`. No CSS module exists. Note: trust-band color tokens (`--color-trust-likely-current`, `--color-trust-use-with-caution`, `--color-trust-needs-verification`) are also not yet defined in `src/styles/design-tokens.css`.
- **Used in surfaces/components:** Not yet used. Intended for DailyBriefing, AccountDetail, MeetingDetail, ProjectDetail, PersonDetail, and as the key primitive inside the `TrustBand` pattern.
- **Recommendation:** **Extract**
- **Rationale:** Trust band rendering is the visual spine of v1.4.0 claim surfaces. Without a canonical module every claim-rendering surface will invent its own band colors.
- **Extraction complexity:** **Trivial** — ~25 CSS lines for 3 band variants. Greenfield. Note: Phase 2 agent must also add the 3 trust-band color tokens to `src/styles/design-tokens.css` as part of this extraction.
- **Phase 2 cluster:** Cluster A — Universal Indicators

---

### FreshnessIndicator

- **Spec:** `.docs/design/primitives/FreshnessIndicator.md`
- **Current CSS location:** No CSS module. The closest existing component, `src/components/editorial/ChapterFreshness.tsx`, uses entirely inline `style={{}}` objects for all styling (lines 61-81) — fontFamily, fontSize, textTransform, letterSpacing, color, margin, display, flexWrap, gap, and conditional staleness color. No `ChapterFreshness.module.css` or `FreshnessIndicator.module.css` exists.
- **Used in surfaces/components:** `ChapterFreshness` is consumed by AccountDetail (chapter freshness strips), ProjectDetail, PersonDetail. `FreshnessIndicator` is the canonical name this component should be promoted to.
- **Recommendation:** **Extract**
- **Rationale:** `ChapterFreshness.tsx` is the concrete implementation to refactor. Moving its 20+ inline style declarations to a CSS module simultaneously promotes the canonical primitive name and eliminates inline-style drift.
- **Extraction complexity:** **Moderate** — `ChapterFreshness.tsx` is 83 lines, entirely inline-styled. Every `style={{}}` prop must be converted to a CSS class. 3 surfaces consume `ChapterFreshness` and will need their imports updated when it is renamed to `FreshnessIndicator`. The staleness color logic (conditional `style` prop) needs a `data-staleness` attribute approach in the module.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### AsOfTimestamp

- **Spec:** `.docs/design/primitives/AsOfTimestamp.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 primitive; planned at `src/components/ui/AsOfTimestamp.tsx`. No TSX component or CSS module exists.
- **Used in surfaces/components:** Not yet used. Intended for Wave 2 receipts, inspection panels, DossierSourceCoveragePanel, and generated report metadata.
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 2 primitive. Creating canonical from the start costs ~15 lines CSS.
- **Extraction complexity:** **Trivial** — 4 variants (relative, absolute, both, unavailable), all mono text color changes. ~15 CSS lines. No migration needed.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### MeetingStatusPill

- **Spec:** `.docs/design/primitives/MeetingStatusPill.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 4 primitive; mockup origin `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html:39-43`. No TSX component or CSS module exists.
- **Used in surfaces/components:** Not yet used. Intended for MeetingDetail SurfaceMasthead accessory slot.
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 4 primitive. Composes Pill with meeting-specific state tokens. Must be canonical from the start — it is the canonical accessory for SurfaceMasthead on MeetingDetail.
- **Extraction complexity:** **Trivial** — 3 state variants (wrapped/processing/failed), ~20 CSS lines. No migration needed. Depends on Pill (Cluster A) landing first.
- **Phase 2 cluster:** Cluster A — Universal Indicators

---

### ProvenanceTag

- **Spec:** `.docs/design/primitives/ProvenanceTag.md`
- **Current CSS location:** `src/index.css:352-365` — global classes `.provenance-tag` (14 lines) and `.provenance-discrepancy` (3 lines). The TSX component at `src/components/ui/ProvenanceTag.tsx:44` applies `className="provenance-tag"` as a bare global string — not a CSS module import. No `ProvenanceTag.module.css` exists.
- **Used in surfaces/components:** AccountDetail (claim attribution rows), DailyBriefing, ProjectDetail, PersonDetail. The `ProvenanceTag.tsx` component already exists and is imported by those surfaces.
- **Recommendation:** **Extract**
- **Rationale:** The TSX component exists but uses a global class. Moving 14 lines from `src/index.css` to `ProvenanceTag.module.css` and updating the one `className` string in `ProvenanceTag.tsx` is the minimal correct fix. The global class must then be removed from `index.css`.
- **Extraction complexity:** **Trivial** — 14 lines CSS to move, 1 import + className change in `ProvenanceTag.tsx`. Zero consumer-side changes (consumers import the component, not the class).
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### SourceCoverageLine

- **Spec:** `.docs/design/primitives/SourceCoverageLine.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 primitive; planned at `src/components/ui/SourceCoverageLine.tsx`. No TSX component or CSS module exists.
- **Used in surfaces/components:** Not yet used. Intended for DossierSourceCoveragePanel and AboutThisIntelligencePanel (Wave 2).
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 2 primitive. ~20 lines CSS.
- **Extraction complexity:** **Trivial** — 3 variants (default/withStaleCount/empty), mono text with saffron stale-count accent. ~20 CSS lines. No migration needed.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### ConfidenceScoreChip

- **Spec:** `.docs/design/primitives/ConfidenceScoreChip.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 primitive; planned at `src/components/ui/ConfidenceScoreChip.tsx`. No TSX component or CSS module exists.
- **Used in surfaces/components:** Not yet used. Intended for Wave 2 receipt and inspection panels.
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 2 primitive. Uses the same trust-band tokens as TrustBandBadge. ~25 CSS lines.
- **Extraction complexity:** **Trivial** — 4 variants (3 bands + unavailable), chip shape ~5 CSS props. No migration needed. Depends on trust-band tokens landing in Cluster A.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### VerificationStatusFlag

- **Spec:** `.docs/design/primitives/VerificationStatusFlag.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 primitive; planned at `src/components/ui/VerificationStatusFlag.tsx`. No TSX component or CSS module exists.
- **Used in surfaces/components:** Not yet used. Intended for Wave 2 receipt panels (LifecycleVerificationRow, ConsistencyFindingBanner, EvidenceBackedClaimRow).
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 2 primitive. Icon + mono label in 3 states. ~20 CSS lines.
- **Extraction complexity:** **Trivial** — 3 status variants (ok/corrected/flagged), icon-gap + color treatment. No migration needed.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### DataGapNotice

- **Spec:** `.docs/design/primitives/DataGapNotice.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 primitive; planned at `src/components/ui/DataGapNotice.tsx`. No TSX component or CSS module exists.
- **Used in surfaces/components:** Not yet used. Intended for DossierSourceCoveragePanel and AboutThisIntelligencePanel (Wave 2).
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 2 primitive. ~15 CSS lines.
- **Extraction complexity:** **Trivial** — 2 severity variants (info/warning), inline mono label + saffron warning color. No migration needed.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### Switch

- **Spec:** `.docs/design/primitives/Switch.md`
- **Current CSS location:** `src/features/settings-ui/NotificationSection.module.css:47-77` — `.switch` (36×20px track, olive active tint, 0.2s ease), `.switch[data-checked="true"]`, `.switchThumb` (16px circle, translateX transition), `.switch[data-checked="true"] .switchThumb`. These classes are consumed by the private `Toggle` function inside `NotificationSection.tsx:38-56` via `s.switch` / `s.switchThumb`. No shared `Switch.tsx` or `Switch.module.css` exists.
- **Used in surfaces/components:** NotificationSection only (1 consumer currently). Wave 3 Settings expansion adds many more.
- **Recommendation:** **Extract**
- **Rationale:** The CSS already exists and is correct — this is pure promotion. Extract to `Switch.module.css`, create `Switch.tsx` from the private Toggle function, update NotificationSection to use `<Switch>` instead. Dead CSS must be removed from `NotificationSection.module.css` after migration.
- **Extraction complexity:** **Moderate** — ~30 lines CSS to extract; the Toggle function inside NotificationSection must be promoted to a shared component; NotificationSection must import and use the new component. One consumer today; the full Settings surface in Wave 3.
- **Phase 2 cluster:** Cluster D — Form Controls

---

### Segmented

- **Spec:** `.docs/design/primitives/Segmented.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 3 primitive; mockup origin `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx:39-51`. No TSX component or CSS module.
- **Used in surfaces/components:** Not yet used. Intended for Settings (Wave 3).
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 3 primitive. Creating canonical from the start is the only valid path before Wave 3 Settings work begins.
- **Extraction complexity:** **Trivial** — 3 tint variants × selected/hover/focus states ≈ 35 CSS lines. No migration needed.
- **Phase 2 cluster:** Cluster D — Form Controls

---

### GlanceCell

- **Spec:** `.docs/design/primitives/GlanceCell.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 3 primitive; mockup origin `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/app.jsx:57-74`. No TSX component or CSS module.
- **Used in surfaces/components:** Not yet used. Intended for Settings masthead via GlanceRow (Wave 3).
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave 3 primitive. Key/value stat cell with optional status dot ≈ 25 CSS lines.
- **Extraction complexity:** **Trivial** — 3 status variants (none/healthy/warn), key/value text treatment, optional dot. No migration needed.
- **Phase 2 cluster:** Cluster D — Form Controls

---

## Promotion-needed patterns

### DayChart

- **Spec:** `.docs/design/patterns/DayChart.md`
- **Current CSS location:** No `src/` implementation. Mockup substrate: `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` — classes `.day-chart`, `.day-bars`, `.bar`, `.bar.past`, `.bar.now-bar`, `.now-line`. Planned component: `src/components/dashboard/DayChart.tsx`.
- **Used in surfaces/components:** Not yet implemented. Canonical for DailyBriefing's Today section.
- **Recommendation:** **Extract**
- **Rationale:** Greenfield Wave pattern. Visually complex self-contained layout (absolutely positioned meeting bars, NOW indicator, hour ticks, legend). Cannot live inside a parent module — it is too structurally distinct.
- **Extraction complexity:** **Complex** — ~100+ CSS lines (absolute-position bar grid, NOW line, legend, hover states, meeting-type color variants × 4). Requires JS for bar left%/width% positioning calculations from time data. No migration needed (greenfield), but component implementation is non-trivial.
- **Phase 2 cluster:** Cluster E — Briefing Patterns

---

### EntityPortraitCard

- **Spec:** `.docs/design/patterns/EntityPortraitCard.md`
- **Current CSS location:** No `src/` implementation. Mockup substrate: `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` — classes `.acc-card`, `.acc-aside`, `.acc-main`, `.acc-thread`, `.ent-acme`, `.ent-northwind`, `.ent-priya`. Planned component: `src/components/dashboard/EntityPortraitCard.tsx`.
- **Used in surfaces/components:** Not yet implemented. Canonical for DailyBriefing's Moving section.
- **Recommendation:** **Extract**
- **Rationale:** Magazine-style portrait card with a CSS grid layout (200px aside | 1fr main), entity-tinted aside band, thread list with typed dots, and stat grid. Self-contained — will never live inside a parent module.
- **Extraction complexity:** **Complex** — ~80+ CSS lines (CSS grid, entity-tint variants × 3, thread dot colors × 4 event types, stat grid, giant glyph treatment). Greenfield. No migration needed.
- **Phase 2 cluster:** Cluster E — Briefing Patterns

---

### ThreadMark

- **Spec:** `.docs/design/patterns/ThreadMark.md`
- **Current CSS location:** No `src/` implementation. Mockup substrate: `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` — class `.thread-mark`. Planned component: `src/components/ui/ThreadMark.tsx`.
- **Used in surfaces/components:** Not yet implemented. Intended for DailyBriefing (WatchListRow, EntityThreadList, MeetingSpineItem foot) and cross-version for all entity surfaces with addressable lines.
- **Recommendation:** **Extract**
- **Rationale:** Universal "talk about this" hover affordance. Must be canonical so it can be dropped into any addressable line without parent-surface CSS coupling.
- **Extraction complexity:** **Trivial** — ~15 CSS lines (mono label, opacity/transform transition, turmeric hover, turmeric-7 background). Note: parent-hover reveal requires each consuming component to add a CSS cascade rule (`.parent:hover .threadMark { opacity: 1 }`). This is a cross-component concern — see Edge Cases.
- **Phase 2 cluster:** Cluster E — Briefing Patterns

---

### AskAnythingDock

- **Spec:** `.docs/design/patterns/AskAnythingDock.md`
- **Current CSS location:** No `src/` implementation. Mockup substrate: `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` — classes `.ask`, `.ask-bar`, `.ask-bar-input`, `.ask-bar-suggestions`, `.ask-bar-scope`, `.ask-chip`. Planned component: `src/components/dashboard/AskAnythingDock.tsx`.
- **Used in surfaces/components:** Not yet implemented. Canonical for DailyBriefing; cross-version foundational for v1.4.6.
- **Recommendation:** **Extract**
- **Rationale:** Three-row editorial input dock with frosted-glass background, suggestion chips, and scope footer. Will be used globally in v1.4.6 — must be self-contained canonical from the start.
- **Extraction complexity:** **Complex** — ~80 CSS lines (three-row layout, frosted-glass card, chip row, scope footer tint, focus shadow). Rotating italic placeholder requires JS. Greenfield; no migration.
- **Phase 2 cluster:** Cluster E — Briefing Patterns

---

### TrustBand

- **Spec:** `.docs/design/patterns/TrustBand.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 pattern; planned at `src/components/intelligence/TrustBand.tsx`. Composes TrustBandBadge + ProvenanceTag + FreshnessIndicator.
- **Used in surfaces/components:** Not yet implemented. Intended as the default trust render inside every ClaimRow.
- **Recommendation:** **Extract**
- **Rationale:** Layout wrapper over three Wave 1 primitives. CSS is the flex layout and gap tokens for compact/default/expanded variants. ~30 lines. Blocked on Cluster C primitives.
- **Extraction complexity:** **Moderate** — ~30 lines CSS, but strictly sequenced after its three composed primitives. No migration needed (greenfield); dependency chain is the risk.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### ClaimRow

- **Spec:** `.docs/design/patterns/ClaimRow.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 pattern; planned at `src/components/intelligence/ClaimRow.tsx`.
- **Used in surfaces/components:** Not yet implemented. Will be the core render unit for every claim on every editorial surface.
- **Recommendation:** **Extract**
- **Rationale:** Single-claim display row composing TrustBand. CSS owns row layout, corrected/flagged/dismissed states, and expandable interaction shell. ~60 CSS lines. Blocked on TrustBand.
- **Extraction complexity:** **Complex** — ~60 CSS lines, 4 states (default/corrected/flagged/dismissed), dependency on TrustBand → 3 primitives. React state logic for expandable mode is non-trivial beyond the CSS.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### ReceiptCallout

- **Spec:** `.docs/design/patterns/ReceiptCallout.md`
- **Current CSS location:** No `src/` implementation. Proposed Wave 2 pattern; planned at `src/components/intelligence/ReceiptCallout.tsx`.
- **Used in surfaces/components:** Not yet implemented. Drill-in from ClaimRow on all claim-rendering surfaces (AccountDetail, DailyBriefing, MeetingDetail, ProjectDetail, PersonDetail).
- **Recommendation:** **Extract**
- **Rationale:** Receipt inspection shell with 2 positions (inline/drawer), 3 per-band border tints, and open/close animation. Self-contained; deepest dependency in the trust chain.
- **Extraction complexity:** **Complex** — ~60 CSS lines, 2 positions (inline/drawer), 3 band tints, transition animation. Full trust chain dependency (blocked on ClaimRow → TrustBand → 3 primitives). Phase 2 must sequence this last in Cluster C.
- **Phase 2 cluster:** Cluster C — Trust UI Primitives

---

### SurfaceMasthead

- **Spec:** `.docs/design/patterns/SurfaceMasthead.md`
- **Current CSS location:** No shared module. Each existing surface uses ad-hoc:
  - Settings: `src/pages/SettingsPage.module.css` (layout-only) + `src/features/settings-ui/styles.ts` (inline JS style objects for field labels, inputs)
  - MeetingDetail: `src/components/meeting-intel.module.css:10-25` — `.meeting-intel_heroTitle` (76px serif, max-width 780px)
  - Planned shared component: `src/components/layout/SurfaceMasthead.tsx`
- **Used in surfaces/components:** SettingsPage (Wave 3 migration), MeetingDetail (Wave 4 migration). Future: Reports surfaces.
- **Recommendation:** **Extract**
- **Rationale:** The spec explicitly generalizes two existing surface mastheads into one contract. Without a canonical module, Settings Wave 3 and MeetingDetail Wave 4 will each build divergent mastheads again.
- **Extraction complexity:** **Moderate** — ~50 CSS lines for 3 density variants (compact/default/rich), 2-column layout with accessory slot, optional glance slot. Migration requires both SettingsPage and MeetingDetail to adopt the new component. Visual parity verification needed for both.
- **Phase 2 cluster:** Cluster D — Form Controls

---

### GlanceRow

- **Spec:** `.docs/design/patterns/GlanceRow.md`
- **Current CSS location:** No `src/` implementation. Mockup origin: `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/app.jsx:57-74` — `.s-glance`, `.gcell`. Planned: `src/components/layout/GlanceRow.tsx` and `GlanceCell.tsx`.
- **Used in surfaces/components:** Not yet implemented. Primary consumer: Settings masthead via SurfaceMasthead.
- **Recommendation:** **Extract**
- **Rationale:** Horizontal GlanceCell layout row. ~25 lines CSS for default/compact/wrap variants. Greenfield; no migration. Depends on GlanceCell (Cluster D).
- **Extraction complexity:** **Trivial** — ~25 lines CSS. Depends on GlanceCell primitive. No migration.
- **Phase 2 cluster:** Cluster D — Form Controls

---

### FormRow

- **Spec:** `.docs/design/patterns/FormRow.md`
- **Current CSS location:** No shared module. Settings sections use inline JS style objects throughout `src/features/settings-ui/styles.ts` for field labels (`.fieldLabel`), inputs (`.input`), and buttons (`.btn`, `.btnPrimary`, `.btnGhost`). The 3-column label/help | control | aux FormRow layout does not exist as a CSS module anywhere.
- **Used in surfaces/components:** Not yet implemented as a shared pattern. All settings sections compose their own versions.
- **Recommendation:** **Extract**
- **Rationale:** Wave 3 Settings explicitly needs FormRow as the universal row contract. Without it, every settings section will have a divergent 3-column layout.
- **Extraction complexity:** **Moderate** — ~40 CSS lines for 4 variants (default/dense/stacked/readonly). Migration scope: all settings section files under `src/features/settings-ui/` must adopt the pattern (broad within-Settings scope, but co-located).
- **Phase 2 cluster:** Cluster D — Form Controls

---

## Already-canonical (verified, no action needed)

| Entry | Tier | Module file |
|---|---|---|
| TypeBadge | primitive | Composed from `src/components/account/AccountHero.module.css` (`.customerBadge`, `.internalBadge`, `.partnerBadge`) + `src/components/entity/EntityHeroBase.module.css` (`.heroBadge`). See Keep-embedded section. |
| IntelligenceQualityBadge | primitive | `src/components/entity/IntelligenceQualityBadge.module.css` |
| InlineInput | primitive | `editable-inline.module.css` (mirrored to `.docs/design/reference/_shared/styles/editable-inline.module.css`; used by existing EditableInline component) |
| Lead | pattern | `src/styles/editorial-briefing.module.css` — `.editorial-briefing_hero`, `.editorial-briefing_heroHeadline`, `.editorial-briefing_heroNarrative`, `.editorial-briefing_staleness` |
| MarginGrid | pattern | `src/styles/editorial-briefing.module.css` — `.editorial-briefing_marginGrid`, `.editorial-briefing_marginLabel`, `.editorial-briefing_marginContent` |
| ChapterHeading | pattern | `src/components/editorial/ChapterHeading.module.css` |
| MeetingSpineItem | pattern | `src/styles/editorial-briefing.module.css` — `.editorial-briefing_scheduleRows`, `.editorial-briefing_scheduleRow`, `.editorial-briefing_scheduleContent`, `.editorial-briefing_scheduleTitle`, `.editorial-briefing_nowPill`, `.editorial-briefing_expandHint`, etc. |
| FolioBar | pattern | `src/components/layout/FolioBar.module.css` |
| FloatingNavIsland | pattern | `src/components/layout/FloatingNavIsland.module.css` |
| AtmosphereLayer | pattern | `src/components/layout/AtmosphereLayer.module.css` |
| AboutThisIntelligencePanel | pattern | `src/components/account/health.module.css` — `.health_metaCard`, `.health_metaCardLabel`, `.health_metaCardText`, `.health_metaCardSubsection`, `.health_metaCardSubLabel` |
| DossierSourceCoveragePanel | pattern | `src/components/account/AboutThisDossier.module.css` — `.AboutThisDossier_section`, `.AboutThisDossier_eyebrow`, `.AboutThisDossier_card`, `.AboutThisDossier_cardLabel`, `.AboutThisDossier_cardText` |
| ConsistencyFindingBanner | pattern | `src/components/meeting/meeting-intel.module.css` — `.meeting-intel_consistencyBanner`, `.meeting-intel_consistencyBannerFlagged`, `.meeting-intel_consistencyCount` |
| StaleReportBanner | pattern | `src/components/reports/report-shell.module.css` — `.report-shell_staleBanner`, `.report-shell_staleBannerButton` |
| FolioActions | pattern | `src/components/meeting/meeting-intel.module.css` — `.meeting-intel_folioActions`, `.meeting-intel_folioBtn`, `.meeting-intel_folioBtnInline`, `.meeting-intel_folioBtnDisabled` |
| AgendaThreadList | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_threadList`, `.PostMeetingIntelligence_threadItem`, `.PostMeetingIntelligence_threadIcon*` |
| PredictionsVsRealityGrid | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_predictionGroup`, `.PostMeetingIntelligence_predictionItem*`, `.PostMeetingIntelligence_predictionIcon*`, `.PostMeetingIntelligence_predictionMatch` |
| SignalGrid | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_signalGrid`, `.PostMeetingIntelligence_signalKey`, `.PostMeetingIntelligence_signalValue` |
| EscalationQuote | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_escalationBlock`, `.PostMeetingIntelligence_escalationQuote`, `.PostMeetingIntelligence_escalationAttribution` |
| FindingsTriad | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_findingsGroup`, `.PostMeetingIntelligence_findingItem`, `.PostMeetingIntelligence_findingDot*`, `.PostMeetingIntelligence_evidenceBlock*` |
| ChampionHealthBlock | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_championHeader`, `.PostMeetingIntelligence_championName`, `.PostMeetingIntelligence_championBadge*`, `.PostMeetingIntelligence_championEvidence`, `.PostMeetingIntelligence_championRisk` |
| CommitmentRow | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_commitmentsList`, `.PostMeetingIntelligence_commitmentItem`, `.PostMeetingIntelligence_commitmentIcon`, `.PostMeetingIntelligence_commitmentTag` |
| SuggestedActionRow | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_actionItem*`, `.PostMeetingIntelligence_suggestedPill`, `.PostMeetingIntelligence_actionText`, `.PostMeetingIntelligence_actionMeta`, `.PostMeetingIntelligence_actionContext`, `.PostMeetingIntelligence_actionControls`, `.PostMeetingIntelligence_actionBtn*` |
| RoleTransitionRow | pattern | `src/components/meeting/PostMeetingIntelligence.module.css` — `.PostMeetingIntelligence_roleChange`, `.PostMeetingIntelligence_roleHeader`, `.PostMeetingIntelligence_roleName`, `.PostMeetingIntelligence_roleTransition`, `.PostMeetingIntelligence_roleStatus`, `.PostMeetingIntelligence_roleArrow`, `.PostMeetingIntelligence_roleEvidenceBlock`, `.PostMeetingIntelligence_roleEvidence` |

---

## Phase 2 extraction clusters

### Cluster A — Universal Indicators (4 entries)

- **Entries:** Pill, RemovableChip, TrustBandBadge, MeetingStatusPill
- **Source modules to refactor:**
  - `src/components/work/WorkSurface.module.css` — remove local `.audiencePill`, `.audienceCustomer`, `.audienceInternal`, `.visibilityPill`, `.visibilityPrivate`, `.visibilityShared` in favour of `Pill` once module lands
  - `src/components/ui/meeting-entity-chips.tsx` — remove inline `style={{}}` chip-shape objects (chip visual migrates to Cluster B, but pill base shape comes from Cluster A)
  - `src/styles/design-tokens.css` — add `--color-trust-likely-current`, `--color-trust-use-with-caution`, `--color-trust-needs-verification` tokens (required for TrustBandBadge)
- **Target:**
  - `src/components/ui/Pill.module.css` + `Pill.tsx`
  - `src/components/ui/RemovableChip.module.css` + `RemovableChip.tsx`
  - `src/components/ui/TrustBandBadge.module.css` + `TrustBandBadge.tsx`
  - `src/components/meeting/MeetingStatusPill.module.css` + `MeetingStatusPill.tsx`
- **Risk:** Low. Execute Pill first — it is a dependency of RemovableChip, TrustBandBadge (all compose Pill), and MeetingStatusPill. WorkSurface migration is a single-file refactor with no DOM changes needed.

---

### Cluster B — Entity Primitives (1 entry)

- **Entries:** EntityChip
- **Source modules to refactor:**
  - `src/components/ui/meeting-entity-chips.tsx` — replace inline `entityColor`/`entityBg` style objects with CSS module classes; consolidate to shared `EntityChip.tsx`
  - `src/components/ui/email-entity-chip.tsx` — same
- **Target:**
  - `src/components/ui/EntityChip.module.css` + `EntityChip.tsx`
- **Risk:** Medium. Both consumers use identical inline-style logic but have different interaction models (MeetingEntityChips is removable + editable with EntityPicker; EmailEntityChip is read-only or editable). The Phase 2 agent must preserve both variants' interaction logic while migrating the visual style to the module. DOM change: `style={{...}}` → `className={styles.chip}` + modifier classes. Must execute after Cluster A (Pill) is done.

---

### Cluster C — Trust UI Primitives (10 entries, ordered by dependency)

- **Entries (execution order):** ProvenanceTag → FreshnessIndicator → AsOfTimestamp → SourceCoverageLine → ConfidenceScoreChip → VerificationStatusFlag → DataGapNotice → TrustBand → ClaimRow → ReceiptCallout
- **Source modules to refactor:**
  - `src/index.css:352-365` — remove `.provenance-tag` + `.provenance-discrepancy` global classes after `ProvenanceTag.module.css` lands
  - `src/components/editorial/ChapterFreshness.tsx` — migrate all ~20 inline `style={{}}` props to `FreshnessIndicator.module.css` classes; rename component to `FreshnessIndicator`
- **Target:**
  - `src/components/ui/ProvenanceTag.module.css`
  - `src/components/ui/FreshnessIndicator.module.css` + `FreshnessIndicator.tsx`
  - `src/components/ui/AsOfTimestamp.module.css` + `AsOfTimestamp.tsx`
  - `src/components/ui/SourceCoverageLine.module.css` + `SourceCoverageLine.tsx`
  - `src/components/ui/ConfidenceScoreChip.module.css` + `ConfidenceScoreChip.tsx`
  - `src/components/ui/VerificationStatusFlag.module.css` + `VerificationStatusFlag.tsx`
  - `src/components/ui/DataGapNotice.module.css` + `DataGapNotice.tsx`
  - `src/components/intelligence/TrustBand.module.css` + `TrustBand.tsx`
  - `src/components/intelligence/ClaimRow.module.css` + `ClaimRow.tsx`
  - `src/components/intelligence/ReceiptCallout.module.css` + `ReceiptCallout.tsx`
- **Risk:** High — strict dependency chain. A single Phase 2 agent must own the full chain in order. ClaimRow and ReceiptCallout are greenfield complex components; the CSS is ~60 lines each but the React state logic (expandable, loading, error, correction) is non-trivial. Scope this cluster as Wave 2 work only. Depends on Cluster A trust-band tokens landing first.

---

### Cluster D — Form Controls (6 entries)

- **Entries:** Switch, Segmented, GlanceCell, GlanceRow (pattern), FormRow (pattern), SurfaceMasthead (pattern)
- **Source modules to refactor:**
  - `src/features/settings-ui/NotificationSection.module.css` — remove `.switch`/`.switchThumb` classes (~lines 47-77) after `Switch.module.css` lands; update NotificationSection to import `<Switch>` component
  - `src/features/settings-ui/styles.ts` — replace all inline JS style objects with FormRow + appropriate sub-component CSS modules once they land
  - `src/pages/SettingsPage.module.css` — masthead section may migrate to SurfaceMasthead
  - `src/components/meeting/meeting-intel.module.css:10-25` — `.meeting-intel_heroTitle` block should migrate to SurfaceMasthead "rich" variant (Wave 4)
- **Target:**
  - `src/components/ui/Switch.module.css` + `Switch.tsx`
  - `src/components/ui/Segmented.module.css` + `Segmented.tsx`
  - `src/components/ui/GlanceCell.module.css` + `GlanceCell.tsx`
  - `src/components/layout/GlanceRow.module.css` + `GlanceRow.tsx`
  - `src/components/settings/FormRow.module.css` + `FormRow.tsx`
  - `src/components/layout/SurfaceMasthead.module.css` + `SurfaceMasthead.tsx`
- **Risk:** Medium. Create greenfield primitives first (Segmented, GlanceCell, GlanceRow) before tackling the migrations (Switch, FormRow, SurfaceMasthead). SurfaceMasthead migration requires visual parity verification across two existing surfaces (Settings + MeetingDetail) — highest-risk item in this cluster. FormRow migration touches all settings section files in `src/features/settings-ui/`.

---

### Cluster E — Briefing Patterns (4 entries)

- **Entries:** DayChart, EntityPortraitCard, ThreadMark, AskAnythingDock
- **Source modules to refactor:** None — all four are greenfield with no existing `src/` implementation.
- **Target:**
  - `src/components/dashboard/DayChart.module.css` + `DayChart.tsx`
  - `src/components/dashboard/EntityPortraitCard.module.css` + `EntityPortraitCard.tsx`
  - `src/components/ui/ThreadMark.module.css` + `ThreadMark.tsx`
  - `src/components/dashboard/AskAnythingDock.module.css` + `AskAnythingDock.tsx`
- **Risk:** Medium-High for DayChart (absolute-position bar math) and AskAnythingDock (rotating placeholder, scope footer, global context seeding from ThreadMark). ThreadMark is trivial CSS but has a cross-component hover cascade concern (see Edge Cases). All four are unblocked by other clusters.

---

## Keep-embedded entries

### TypeBadge — keep embedded

- **Reason:** TypeBadge renders correctly today using composed classes from `src/components/account/AccountHero.module.css` (`.customerBadge` lines 47-49, `.internalBadge` lines 51-54, `.partnerBadge` lines 56-59) and `src/components/entity/EntityHeroBase.module.css` (`.heroBadge`). The showcases.html correctly marks it "Canonical module." The badge's editable dropdown variant (type-picker, chevron, dropdown positioning) is tightly coupled to AccountHero's layout CSS (`.AccountHero_typeBadgeWrapper`, `.AccountHero_typeBadgeButton`, `.AccountHero_typeBadgeDropdown`, `.AccountHero_typeBadgeOption*` — lines 141-203 of AccountHero.module.css). AccountDetail is TypeBadge's only production consumer. Extracting a standalone `TypeBadge.module.css` would either duplicate the badge color rules (creating drift) or retain the `composes:` chain (adding complexity without clarity). The value of extraction is zero until TypeBadge gains consumers outside AccountDetail.
- **Spec update needed:** Add a "Source location" note to `.docs/design/primitives/TypeBadge.md` reading: "Canonical CSS lives inside `src/components/account/AccountHero.module.css` (badge color classes at lines 47-59) and `src/components/entity/EntityHeroBase.module.css` (badge base layout). The editable dropdown variant is in `AccountHero_typeBadge*` classes (lines 141-203). A standalone `TypeBadge.module.css` is not warranted until TypeBadge gains consumers outside AccountDetail."

---

## Edge cases + risks

1. **ProvenanceTag global-class cleanup is mandatory.** After Cluster C extracts `ProvenanceTag.module.css`, the agent must remove `.provenance-tag` and `.provenance-discrepancy` from `src/index.css:352-365`. Leaving the global classes creates a silent naming collision: any surface that imports the module without migrating its `className` string will pick up the global rule instead, making breakage invisible until a visual diff.

2. **ChapterFreshness is a component refactor, not just CSS extraction.** `ChapterFreshness.tsx` uses `style={{}}` inline objects for every CSS property — there is no CSS to "extract" since none exists in a file. The Phase 2 Cluster C agent must rewrite the component: add a CSS module, convert all inline props to className assignments, and rename the component to `FreshnessIndicator`. Consumers (AccountDetail, ProjectDetail, PersonDetail chapter freshness strips) must update their import paths.

3. **EntityChip inline-style migration is a DOM change.** Both `meeting-entity-chips.tsx` and `email-entity-chip.tsx` apply chip visuals via `style={{...}}` objects, not className. The Cluster B agent cannot just add a CSS module import — it must change the render path from inline styles to className props. Run a visual comparison before and after to confirm the chip dimensions, token colors, and icon alignment are unchanged.

4. **Switch dead-class cleanup in NotificationSection.** After `Switch.tsx` is extracted and NotificationSection migrates to use it, the `.switch`/`.switchThumb` classes in `NotificationSection.module.css` must be removed. Leaving them is dead CSS that misleads the next engineer into thinking NotificationSection has a local switch implementation.

5. **ThreadMark parent-hover cascade.** ThreadMark must be revealed by its parent element's `:hover` state. A CSS module cannot express cross-component `:hover` cascades natively. The Phase 2 Cluster E agent must establish the canonical pattern before implementing — either a data-attribute approach (`[data-addressable]:hover .threadMark { opacity: 1 }` in a shared global rule) or a CSS variable trick. Tailwind's `group` + `group-hover` utilities conflict with the CSS-module-first architecture. Document the chosen approach in `ThreadMark.md` before shipping.

6. **SurfaceMasthead visual parity requirement.** Migrating SettingsPage and MeetingDetail to use SurfaceMasthead requires verifying that the new component reproduces the exact existing visual output for both surfaces. MeetingDetail's current `.meeting-intel_heroTitle` is 76px serif at `max-width: 780px` — SurfaceMasthead's "rich" variant must match this exactly, or the MeetingDetail migration will produce a visual regression. The Phase 2 Cluster D agent must run a side-by-side comparison on both surfaces before marking done.

7. **Trust-band tokens must land in Cluster A, not Cluster C.** `--color-trust-likely-current`, `--color-trust-use-with-caution`, and `--color-trust-needs-verification` are consumed by both TrustBandBadge (Cluster A) and ConfidenceScoreChip (Cluster C). The Cluster A agent must add these tokens to `src/styles/design-tokens.css` as part of the TrustBandBadge extraction. Cluster C agents depend on them already being present.

8. **Cluster C strict sequencing.** The trust chain is TrustBandBadge + ProvenanceTag + FreshnessIndicator → TrustBand → ClaimRow → ReceiptCallout. If Cluster C is split across multiple agents, each hand-off must confirm the upstream primitives are merged before the downstream pattern begins. Parallelizing ClaimRow and ReceiptCallout is never valid — ReceiptCallout wraps ClaimRow.
