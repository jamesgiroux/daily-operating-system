# Design System Changelog

All notable changes to the DailyOS design system. See `VERSION.md` for bump rules.

Format:

```
## [version] — YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Removed
- ...

### Notes
- Migration notes, deprecations, things consumers should know
```

---

## [0.6.0] — 2026-05-06

### Added

**Daily Briefing redesign substrate (DOS-413, BriefingViewModel contract):**

Primitives:
- `SignalDot` (`primitives/SignalDot.md`) — tinted-dot signal-feed bullet with eight `kind` variants (meeting, action, email, lifecycle, gongCall, zendeskTicket, slackThread, linearIssue). Carries `LifecycleMixin` correction states. Reference: `briefing-redesign.html` Moving section.
- `ProvenanceStat` (`primitives/ProvenanceStat.md`) — labeled metric with optional trend tint (up / down / flat). Carries `TrustMixin` for per-stat provenance. Reference: `briefing-redesign.html` MovingRow right column.

Patterns:
- `MovingRow` (`patterns/MovingRow.md`) — three-column entity-movement row (identity / lede + signals / stacked stats). Five `kind` variants (customer, person, project, internal, lifecycle). Composes `Pill`, `SignalDot`, `ProvenanceStat`, `EntityChip`. Click target: whole row via `role="link"` + tabindex (not wrapping `<a>`).
- `WatchRow` (`patterns/WatchRow.md`) — adaptive triage row, four `kind` variants (`suggestedAction`, `openAction`, `parked`, `aging`) selecting different right-column affordances. Composes `InferredActionSelector`.
- `PredictionsSection` (`patterns/PredictionsSection.md`) — collapsed-by-default predictions list within a `MarginGrid`. Composes `TrustBandBadge` per item. Restraint contract: collapsed default <32px vertical.
- `BriefingLoadingState` (`patterns/BriefingLoadingState.md`) — centered editorial holding state with optional pulsing dot. Surface-specific copy passed via props.
- `BriefingErrorState` (`patterns/BriefingErrorState.md`) — centered editorial error frame with retry / diagnostics affordances and optional `code` / `service` meta line.
- `BriefingEmptyState` (`patterns/BriefingEmptyState.md`) — left-aligned cold-start frame with eyebrow / headline / lede / optional checklist / optional CTA.

Tokens (`tokens/color.md` → "Signal kind"):
- `--color-signal-meeting` → `--color-garden-larkspur` (shared paint with `--color-person`)
- `--color-signal-action` → `--color-spice-saffron`
- `--color-signal-email` → `--color-garden-sage`
- `--color-signal-lifecycle` → `--color-spice-turmeric`
- `--color-signal-gong-call` → `--color-spice-terracotta`
- `--color-signal-zendesk-ticket` → `--color-text-tertiary`
- `--color-signal-slack-thread` → `--color-garden-eucalyptus`
- `--color-signal-linear-issue` → `--color-garden-olive`

Eight aliases mapped to existing paint tokens; tint variants (`-8`, `-15`, etc.) are not aliased — surfaces that need tinted SignalDot backgrounds reference the underlying paint family directly (e.g. `--color-garden-larkspur-15`). Use the signal alias for the dot fill only.

Reference renders:
- `briefing-redesign.html` — DailyBriefing redesign success state
- `briefing-redesign-loading.html` — loading state stub
- `briefing-redesign-error.html` — error state stub
- `briefing-redesign-empty.html` — empty state stub

ADR:
- `decisions/0129-briefing-view-model-contract.md` — full contract + typography appendix + auth-error disambiguation + future-tax follow-up.

### Notes

- All eight new specs at `proposed` status. TSX ships across W1 / W3 / W5 of the Daily Briefing redesign waves (DOS-422..429).
- Three state patterns originally drafted as `Briefing{Loading,Error,Empty}State` — renamed to `Editorial{...}State` per `NAMING.md` rule "patterns are named for the pattern, not the surface." Surface-specific copy now passes through props instead of being baked in.
- Reference HTMLs use provisional `.dspine-*` class names until W1 cuts over to scoped `MovingRow_*` / `WatchRow_*` names; pre-implementation entries land in `_audits/surface-manifest.json` so the audit doesn't fail before TSX exists.

---

## [0.5.0] — 2026-05-03

### Added

**Substrate promotion (DOS-358):**
- `_shared/{tokens,primitives,chrome,fonts}.css + chrome.js` moved from `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/` to canonical `.docs/design/reference/_shared/`. Mockup `<link>` and `<script>` paths updated.
- `chrome.js` enhanced with `data-nav-base` support: when set on body, `FloatingNavIsland` renders nav items as anchor tags linking to peer surface files (`<base>/<id>.html`). When unset, items render as buttons (mockup default — backward compatible).

**Reference renders (17 HTML files):**
- `reference/index.html` — hub linking every surface + system showcase, with surface-card grid grouped by area.
- `reference/surfaces/` — 13 surface clones with mock data, all interlinked via `data-nav-base="."`:
  - `briefing.html`, `week.html`, `accounts.html`, `projects.html`, `people.html`, `settings.html`, `me.html` (codex-written)
  - `account.html`, `project.html`, `person.html`, `meeting.html`, `inbox.html`, `actions.html` (hand-written after codex agents hung)
- `reference/system/` — 3 design-system showcase pages:
  - `tokens.html` — color swatches by family, type ramps, spacing scale, shadows, z-index
  - `primitives.html` — gallery of all 18 primitives with variants, grouped by wave
  - `patterns.html` — gallery of all 31 patterns with representative renders, grouped by wave (jump-nav)
- All reference renders carry `data-ds-name` + `data-ds-tier` + `data-ds-spec` attributes per the inspector convention; press `?` on any page to toggle inspect mode.

### Notes

- Mock-data palette: Acme Corp / Globex Inc / Northwind Traders / Meridian Harbor / Stark Industries; Jen Park / Dan Mitchell / Priya Raman / Marco Devine / Aoife Murphy / Liu Kang / Sara Wu / Kevin Otieno; subsidiary.com / parent.com / example.com domains. No real customer data anywhere.
- Codex spec writing for waves 2-4 worked reliably with bounded prompts; reference render generation hit hangs again on 5 of 8 agents — fell back to hand-writing the missing surfaces (account, project, person, meeting, inbox, actions, system pages). Pattern: HTML generation at this size is at the edge of bounded-codex reliability.
- v1.4.3 / v1.4.4 / Settings redesign / Meeting Detail redesign now have working visual references designers + reviewers can open in browser and navigate as if they were the app.

---

## [0.4.0] — 2026-05-03

### Added

**Wave 4 — Meeting Detail substrate (DOS-356):**
- 1 primitive: `MeetingStatusPill` (wrapped / processing / failed)
- 11 patterns: `SuggestedActionRow`, `FolioActions`, `AgendaThreadList`, `EscalationQuote`, `RoleTransitionRow`, `PredictionsVsRealityGrid`, `SignalGrid`, `FindingsTriad`, `ChampionHealthBlock`, `CommitmentRow`
- 1 surface: `MeetingDetail` (chapters: 7 sections via FloatingNavIsland)

### Notes

- `MeetingHero` subsumed into `SurfaceMasthead` (Wave 3); MeetingDetail composes SurfaceMasthead with `MeetingStatusPill` accessory rather than introducing a separate hero pattern.
- `FolioActions` is documented as a separate pattern from `FloatingNavIsland` per D2: actions toolbar, not navigation.
- All `cur-pm-*` and `cur-folio-*` prefixed mockup classes consolidated into the canonical patterns above; do not promote mockup class names.

---

## [0.3.0] — 2026-05-03

### Added

**Wave 3 — Settings substrate (DOS-355):**
- 5 primitives: `InlineInput`, `Switch`, `Segmented`, `RemovableChip`, `GlanceCell`
- 3 patterns: `SurfaceMasthead`, `FormRow`, `GlanceRow`
- 1 surface: `Settings` (chapters: Identity / Connectors / Briefing / Data / Activity / System / Diagnostics via FloatingNavIsland)

### Notes

- `SectionTabbar` (mockup proposal) explicitly NOT introduced as a separate pattern — Settings provides chapters to `FloatingNavIsland` per D2; the numbered labels (`01 You / 02 Connectors / …`) live in the chapter labels rendered by FloatingNavIsland's local pill.
- `SurfaceMasthead` (Wave 3) generalizes from Settings' masthead AND MeetingDetail's hero — used in both Wave 3 and Wave 4 surfaces.
- `RemovableChip` is intentionally distinct from `Pill` (different interaction — × removal vs. labeled state).

---

## [0.2.0] — 2026-05-03

### Added

**Wave 2 — Trust UI substrate (DOS-354):**
- 5 primitives: `SourceCoverageLine`, `ConfidenceScoreChip`, `VerificationStatusFlag`, `DataGapNotice`, `AsOfTimestamp`
- 7 patterns: `TrustBand`, `ClaimRow`, `ReceiptCallout`, `AboutThisIntelligencePanel`, `DossierSourceCoveragePanel`, `StaleReportBanner`, `ConsistencyFindingBanner`

### Changed

- `FreshnessIndicator` (Wave 1 primitive) updated: `FreshnessChip` (Wave 2 audit candidate) consolidated into FreshnessIndicator. Single canonical name; FreshnessChip dropped from spec scope.

### Notes

- Wave 2 establishes the v1.4.4 inspection layer: `TrustBand` for the surface-level trust render, `ClaimRow` for the unit-of-claim API, `ReceiptCallout` for inspection drill-in.
- Resolver-band primitive (`ResolverConfidenceBadge`, `Resolved / ResolvedWithFlag / Suggestion / NoMatch`) deferred from this wave; surfaces inside `ReceiptCallout` for the receipt/inspection experience but not yet specified — file separately when v1.4.4 implementation reveals concrete need.
- `MeetingHero` reconciliation tracked into Wave 4 (subsumed into Wave 3's `SurfaceMasthead`).

---

## [0.1.0] — 2026-05-02

### Added

- **Tokens** (9 specs): `color`, `typography`, `spacing`, `motion`, `layout`, `radius`, `shadows`, `glass`, `z-index`. Color spec includes named surface/entity color aliases for account, project, person, action, and self.
- **Primitives** (7 specs): `Pill` (canonical), `TrustBandBadge` (proposed, new per v1.4.0 substrate), `IntelligenceQualityBadge` (canonical, existing), `FreshnessIndicator` (proposed, new), `ProvenanceTag` (canonical, existing), `EntityChip` (canonical, post-DOS-357), `TypeBadge` (canonical).
- **Patterns** (11 specs): `FolioBar`, `FloatingNavIsland` (dual-pill production component, the canonical local-nav pattern per D2), `AtmosphereLayer`, `MarginGrid`, `ChapterHeading` (5 chrome/layout); `Lead`, `DayChart`, `MeetingSpineItem`, `EntityPortraitCard`, `ThreadMark`, `AskAnythingDock` (6 Daily Briefing redesign briefing patterns).
- **Surfaces** (1 spec): `DailyBriefing` — first canonical surface spec, documents chapter inventory for FloatingNavIsland adoption.
- Tier README indexes for tokens, primitives, patterns, surfaces.
- Production token reconciliation (DOS-357): `src/styles/design-tokens.css` regains entity color aliases; explicit entity-type-to-color maps in `meeting-entity-chips.tsx` + `email-entity-chip.tsx` migrated to use them (also fixes `--color-sky-larkspur` typo).

### Notes

- Wave 1 closes the v1.4.3 (Briefing) substrate prep. v1.4.3 implementation can begin against these specs.
- Trust band CSS tokens (`--color-trust-{likely-current,use-with-caution,needs-verification}` plus `8`, `10`, `12`, `15` alpha aliases) are shipped in runtime CSS and documented in `tokens/color.md`.
- Local-nav decision: `FloatingNavIsland` (production dual-pill) is canonical. `SectionTabbar` remains rejected per D2; `DayStrip` is now represented only as a proposed `DailyBriefingRedesign` exception pending v1.4.0 review.
- 16 Linear issues (DOS-353 through DOS-361) track the remaining waves and cross-cutting work.

---

## [0.0.0] — 2026-05-02

### Added

- Initial scaffolding: directory structure for `tokens/`, `primitives/`, `patterns/`, `surfaces/`, `reference/`, `_archive/`
- `SYSTEM-MAP.md` — taxonomy, lifecycle, conventions
- `NAMING.md` — naming policy + first rename candidate (`Dashboard` → `DailyBriefing`)
- `_TEMPLATE-entry.md` — entry template
- `VERSION.md`, `CHANGELOG.md` — versioning ground truth
- `reference/_shared/inspector.js` + `inspector.css` — opt-in hover inspector for reference renders
- `data-ds-*` convention documented in `SYSTEM-MAP.md`
- `.docs/_archive/mockups/` demoted to exploration-only with `current/` and `_archive/` subdirs

### Notes

- No canonical entries yet. The four foundational audits are running in parallel; their findings will populate the first canonical entries and trigger a bump to `0.1.0`.
- Existing `.docs/design/*.md` files (DESIGN-SYSTEM.md, COMPONENT-INVENTORY.md, etc.) remain in place pending audit synthesis. They will move to `_archive/` and per-entry specs will become canonical.
