# v1.4.4 WP Visual Parity Inventory

**Branch:** `feat/v1.4.4-wp-visual-parity` (forked from `public/dev` at `d2d5f7ce`)
**Reference:** `.docs/design/reference/surfaces/account.html` (canonical magazine layout, 1016 lines)
**Current WP render:** `http://localhost:8884/accounts/bring-a-trailer/`
**Live screenshot baseline:** `/tmp/wp-account-current.png` (captured 2026-05-22)

## Purpose

For each magazine chapter in the reference, characterize the gap vs the current WP render along three layers:

- **L1 — Markup/CSS parity** (mechanical port from reference HTML to block `render-functions.php`)
- **L2 — Prose composition** (substrate produces narrative, not claim-row dumps)
- **L3 — Trust attribution** (every value is claim-backed; no "unscored" surfaces)

`Status` legend: `✅` = at parity, `🟡` = partial, `❌` = absent, `n/a` = not applicable / out of scope.

## Chapter → WP block map

| # | Reference Chapter | DS Section Class | Primary WP Block | Inner / Related Blocks |
|---|---|---|---|---|
| 1 | Headline | `AccountHero` + `EditableVitalsStrip` + `IntelligenceQualityBadge` | `account-hero` | `sentiment-hero` (sub-strip) |
| 2 | Outlook | `AccountOutlook_section` (3 sub-sections) | `outlook-panel` | — |
| 3 | Products & Entitlements | `AccountDetailEditorial_productList` | `account-technical-footprint` | `commercial-shape` |
| 4 | State of Play | `UnifiedTimeline_section` | `unified-timeline` | `touchpoints-feed`, `on-track-chapter`, `triage-section`, `divergence-section` |
| 5 | The Room | `StakeholderGallery_section` + `StakeholderGrid` | `stakeholder-grid` | `relationship-fabric` |
| 6 | Watch List | `WatchList_section` | (no dedicated block — likely should be one) | — |
| 7 | Value & Commitments | `ValueCommitments_section` | `value-commitments` | `quote-wall` |
| 8 | Competitive & Strategic | `StrategicLandscape_section` | `strategic-landscape` | `supporting-tension`, `divergence-section` |
| 9 | The Record | `FileListSection_section` | `file-list` | `account-pull-quote` |
| 10 | The Work | `TheWork_section` | `linear-issues-chapter` | `recommended-actions`, `open-loops-feed` |
| 11 | Reports | `FolioReportsDropdown` | (no dedicated block — drop-down on chrome) | — |
| 12 | About This Dossier | `AboutThisDossier_section` | `about-this-dossier` | `about-intelligence`, `finis-marker` |

## Per-chapter inventory

### 1. Headline (AccountHero)
- **Reference:** Hero with type badge button (chevron pop-up), editable account name (`EditableText`), `EditableVitalsStrip` with 6 vital items (ARR / Health / Lifecycle / Renewal / NPS / activity), each with `data-highlight`, source attribution chips (`REDACTED` / `Glean CRM`), `IntelligenceQualityBadge` "fresh" with hover title.
- **WP block:** `account-hero/render-functions.php` (294 lines)
- **DS sections present in WP:** `AccountHero`, `EditableText`, `EditableVitalsStrip`, `IntelligenceQualityBadge` ✅
- **L1 markup:** likely close. Audit needed.
- **L2 prose composition:** n/a — hero is structured fields, not narrative.
- **L3 trust attribution:** unknown — needs verification that vitals strip values are claim-backed with `data-claim-id` and `data-trust-band` per item.

### 2. Outlook ⭐ (exemplar)
- **Reference (lines 161–240):** Three sub-sections:
  - `AccountOutlook_renewalSection` — *Renewal prose* (`Renewal confidence is high`) + editable text + `IntelligenceFeedback` thumbs + `AccountOutlook_renewalStart` ("Start the conversation by June 3, 2026")
  - `AccountOutlook_growthSection` — Growth Opportunities signal list, each item with prose text + meta (badge + ARR impact + `ProvenanceTag`) + dismiss button
  - `AccountOutlook_commercialSection` — Commercial Reality contract grid (4 cells: Type / Auto-Renew / Renewal / Current ARR)
- **WP block:** `outlook-panel/render-functions.php` (166 lines)
- **WP renders:** chapter heading + `AccountOutlook_section` wrapper + ONLY `AccountOutlook_growthSection` (claim rows from `agreementOutlook`, `contractContext`, `expansionSignals`).
- **L1 markup:** 🟡 partial. Missing entire renewal prose section + entire commercial reality contract grid.
- **L2 prose composition:** ❌ absent. Renders claim rows via `dailyos_outlook_panel_render_row` → just `rendered_text + trust_band badge`. No prose composition. No editable text. No `IntelligenceFeedback` widgets. No `ProvenanceTag`. No ARR impact decoration.
- **L3 trust attribution:** 🟡 partial. Claim rows carry `data-claim-id` and `data-trust-band`. Renewal prose + commercial grid aren't claim-backed at all (because they don't render).
- **Substrate gap:** producer needs to emit:
  - `renewal_confidence` claim with rendered prose ("Renewal confidence is high")
  - `renewal_start_date` claim
  - `contract_details` claim group (type / auto-renew / renewal date / current ARR)

### 3. Products & Entitlements
- **Reference (lines 242+):** Product list (`AccountDetailEditorial_productRow`) — product name + details.
- **WP block:** `account-technical-footprint` (187 lines), DS sections: `AccountTechnicalFootprint`, `ChapterHeading`, `ReferenceGrid`
- **L1 markup:** unknown — DS class name (`AccountTechnicalFootprint`) doesn't match reference (`AccountDetailEditorial_productList`). Likely a naming drift; audit needed.
- **L2 prose composition:** n/a — list structure.
- **L3 trust attribution:** unknown.

### 4. State of Play (UnifiedTimeline)
- **Reference (lines 303+):** Timeline of touchpoints / events with chapter dividers (on-track / triage / divergence).
- **WP outer:** `unified-timeline` (187 lines)
- **WP inner blocks:** `touchpoints-feed`, `on-track-chapter`, `triage-section`, `divergence-section` — these may be ordered as nested chapters in the reference's timeline.
- **L1 markup:** likely close.
- **L2 prose composition:** unknown — timeline entries may need prose summaries vs raw event lists.
- **L3 trust attribution:** unknown.

### 5. The Room (Stakeholders)
- **Reference (lines 392+):** `StakeholderGallery_section` with attendee cards.
- **WP block:** `stakeholder-grid` (192 lines), DS sections: `ChapterHeading`, `StakeholderGrid`
- **L1 markup:** reference uses `StakeholderGallery`, WP uses `StakeholderGrid`. Possible naming drift.
- **L2 prose composition:** likely n/a — attendee cards.
- **L3 trust attribution:** unknown.

### 6. Watch List
- **Reference (lines 522+):** `WatchList_section` with watch items.
- **WP block:** **no dedicated block.** ❌
- **Gap:** likely a missing block. Either fold into another (e.g., `open-loops-feed`) or scaffold a `watch-list` block.

### 7. Value & Commitments
- **Reference (lines 587+):** `ValueCommitments_section` with editorial blocks (and `quote-wall` for testimonial-style content).
- **WP outer:** `value-commitments` (160 lines), `quote-wall` (separate block — likely embedded here)
- **L1 markup:** unknown.
- **L2 prose composition:** likely needed — value statements + commitment narratives.
- **L3 trust attribution:** unknown.

### 8. Competitive & Strategic
- **Reference (lines 640+):** `StrategicLandscape_section` with positioning + competitor mentions + supporting/divergence tensions.
- **WP outer:** `strategic-landscape` (163 lines)
- **WP inner:** `supporting-tension`, `divergence-section`
- **L1 markup:** plausible.
- **L2 prose composition:** likely needed for positioning narrative.
- **L3 trust attribution:** unknown.

### 9. The Record
- **Reference (lines 699+):** `FileListSection` style — files + their last touched dates.
- **WP block:** `file-list` (186 lines), DS sections: `FileListSection`
- **L1 markup:** likely close.
- **L2 prose composition:** n/a — file list.
- **L3 trust attribution:** unknown.

### 10. The Work
- **Reference (lines 753+):** `TheWork_section` with Linear-style work items, recommended actions, open loops.
- **WP outer:** `linear-issues-chapter` (194 lines)
- **WP inner:** `recommended-actions`, `open-loops-feed`
- **L1 markup:** likely needs audit.
- **L2 prose composition:** likely n/a — list / status.
- **L3 trust attribution:** unknown — each work item should be claim-backed.

### 11. Reports
- **Reference (line 832):** `FolioReportsDropdown` — chrome dropdown, not chapter body.
- **WP:** chrome-level, not a content block.
- **Action:** n/a for inventory — chrome is separate work.

### 12. About This Dossier
- **Reference (line 925):** `AboutThisDossier_section` with provenance + author attribution.
- **WP outer:** `about-this-dossier` (184 lines)
- **WP inner:** `about-intelligence`, `finis-marker`
- **L1 markup:** likely close.
- **L2 prose composition:** dossier-level narrative.
- **L3 trust attribution:** dossier provenance.

## Cross-cutting findings (from one-section deep dive)

These show up everywhere; treat as class-pattern work, not per-block patches:

1. **Prose composition is the dominant gap.** Producer emits atomic claim rows; reference shows narrative composed from claims (renewal confidence statements, growth opportunity prose, value statements). Substrate needs a PTY-enriched composition step per section, emitting one prose claim per narrative beat (per memory: `intelligence.rs +634` got us 10 typed projections but they're atomic, not composed).

2. **Decorations missing across the board:**
   - `IntelligenceFeedback` thumbs (helpful / not-helpful) on prose
   - `ProvenanceTag` source chips ("from Glean", etc.)
   - `EditableText` markers (`data-editable-text`) on user-editable surfaces
   - `data-highlight` color cues on `EditableVitalsStrip` items

3. **Section drift:** several WP blocks use DS class names that don't match the reference (`StakeholderGrid` vs `StakeholderGallery`, `AccountTechnicalFootprint` vs `AccountDetailEditorial_productList`). Audit + decide canonical names per NAMING.md before porting more markup.

4. **Empty-state pattern works.** Every block emits `dailyos-empty-chip` with `data-empty-reason` per the §10 invariant. Good baseline.

5. **`Watch List` chapter has no WP block.** Either fold into adjacent or scaffold new.

## Recommended exemplar-first path

Per memory ("Don't swing past center; pick a single section as the parity exemplar"):

1. **Exemplar: Outlook chapter.** Three sub-sections, rich variety (prose + signals + grid), already partial. Walk all three layers on it: port reference markup, define producer composition, ensure trust attribution. That's the template.
2. **Then fan out** to Headline (no L2 work — easiest), Stakeholder, Value & Commitments, Competitive, The Work, Timeline, etc.
3. **Substrate work — separate track:** PTY composition ability scaffolded as a new ability (likely `compose_section_narrative` or similar) that runs over claims and emits prose claims per section.

## Scope (set 2026-05-22 per human gate)

**Out of scope on this branch:** PTY composition / `get_entity_intelligence` producer-side schema work. Substrate-level claim coverage is owned by a parallel session; until that lands, prose-composition substrate work is blocked.

**Parity policy:** if it's in the Tauri app reference, it's in the WP surface. No "should we" questions on per-feature inclusion.

**DS naming:** `.docs/design/reference/` is canonical. Any drift is a bug — correct it mechanically.

## Work plan (this branch)

In priority order; each lands independently:

1. **Finis-marker dedup.** The `finis-marker` inner block duplicates the FinisMarker already rendered in `wp/dailyos/theme/parts/footer.html`. Remove from the account-detail render path; keep the site footer's instance.

2. **AccountViewSwitcher tabs** on account-detail. Reference (line 944-947) has 3 tabs: Health / Context / Work. WP CSS exists (`wp/dailyos/theme/assets/styles/patterns/AccountViewSwitcher.module.css`) but markup is absent. Add to default pattern.

3. **NavIsland global + local with chapter definitions.** `chrome.js` reads `body.dataset.chapters` (pipe-separated) and `body.dataset.activeChapter`. Currently WP single-dailyos_account.html doesn't set these. Wire via theme filter (functions.php) so account pages emit chapter specs that match the rendered chapter IDs.

4. **DS naming drift sweep.** Known cases:
   - `StakeholderGrid` (WP) → `StakeholderGallery` (reference canonical)
   - `AccountTechnicalFootprint` (WP) → `AccountDetailEditorial_productList` (reference canonical) — verify before changing
   - Audit all account-detail inner blocks for additional drift
   - Mechanical rename + CSS file rename + theme.json sync

## Deferred (other session)

- PTY composition substrate work — see substrate track.
- Editable affordances (`EditableText`, `data-highlight`) — depend on producer emitting editable claim refs.

## Follow-ups discovered while wiring NavIsland (this branch)

- **No WP block for `Watch List` chapter** — NavIsland anchor `#watch-list` lands on nothing. Need to scaffold a new `watch-list` inner block.
- **No WP block for `Reports` chapter** — same. Reports chrome (FolioReportsDropdown) handles per-report selection but the chapter anchor lands nowhere.
- **`The Record` chapter has no dedicated WP block.** Reference's "The Record" is a separate `<section>` (no class) with TimelineEntry primitives, distinct from `UnifiedTimeline_section` (State of Play). Scaffold a `the-record` block or split `unified-timeline` into two.
- **`The Work` chapter needs structural rework.** Current `linear-issues-chapter` block emits heading "Linear Issues" inside a chapter that should be "The Work" wrapping `TheWork_section` (objectives, milestones, header actions). Anchor now lands (`id="the-work"`); the inner structure does not match reference.
- **3 chrome.js icons missing** (telescope, clock, award). Account chapter spec uses fallbacks (target, calendar, star). Add SVG paths to `wp/dailyos/theme/assets/chrome/chrome.js` `ICONS` map for canonical icon parity.

## DS drift sweep — landed (this branch)

- `stakeholder-grid` block: `StakeholderGrid_*` → `StakeholderGallery_*` on chapter wrapper + inner card classes (reference uses `StakeholderGallery_section` / `_grid` / `_card` / `_cardHeader` / `_avatarRing` / `_name` / `_titleLine` per `.docs/design/reference/surfaces/account.html` line 392+). `data-ds-name` + `data-ds-spec` aligned.
- `account-technical-footprint` block: `AccountTechnicalFootprint_*` → `AccountDetailEditorial_*` on chapter wrapper, product list, product rows (reference line 251+). `data-ds-name` + `data-ds-spec` aligned.

Heuristic post-check: WP class prefixes in account-detail inner blocks are now ⊆ reference `_shared/styles/*.module.css` set (only `LinearIssuesChapter` flagged, and that's a valid React component whose CSS lives at `src/components/entity/LinearIssuesChapter.module.css` — reference is curated, not exhaustive). Inner-class modifier-level drift would need visual L4 to catch.

## Per-chapter L1/L2/L3 detail (preserved for context)

The chapter inventory above documents the deeper gap for each chapter. L1 markup port + L2 prose composition + L3 trust attribution remain the long-term parity framework — but execution is blocked behind substrate and the work plan above is what this branch does today.
