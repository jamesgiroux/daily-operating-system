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

- **Tokens** (4 specs): `color`, `typography`, `spacing`, `motion`. Color spec includes 5 entity color aliases (`--color-entity-{account,project,person,action,user}`) reintroduced via DOS-357.
- **Primitives** (7 specs): `Pill` (canonical), `TrustBandBadge` (proposed, new per v1.4.0 substrate), `IntelligenceQualityBadge` (canonical, existing), `FreshnessIndicator` (proposed, new), `ProvenanceTag` (canonical, existing), `EntityChip` (canonical, post-DOS-357), `TypeBadge` (canonical).
- **Patterns** (11 specs): `FolioBar`, `FloatingNavIsland` (dual-pill production component, the canonical local-nav pattern per D2), `AtmosphereLayer`, `MarginGrid`, `ChapterHeading` (5 chrome/layout); `Lead`, `DayChart`, `MeetingSpineItem`, `EntityPortraitCard`, `ThreadMark`, `AskAnythingDock` (6 D-spine briefing patterns).
- **Surfaces** (1 spec): `DailyBriefing` — first canonical surface spec, documents chapter inventory for FloatingNavIsland adoption.
- Tier README indexes for tokens, primitives, patterns, surfaces.
- Production token reconciliation (DOS-357): `src/styles/design-tokens.css` regains entity color aliases; explicit entity-type-to-color maps in `meeting-entity-chips.tsx` + `email-entity-chip.tsx` migrated to use them (also fixes `--color-sky-larkspur` typo).

### Notes

- Wave 1 closes the v1.4.3 (Briefing) substrate prep. v1.4.3 implementation can begin against these specs.
- Trust band CSS tokens (`--color-trust-{likely-current,use-with-caution,needs-verification}`) are proposed in `tokens/color.md` but not yet added to runtime CSS — added during Wave 1 implementation.
- Local-nav decision: `FloatingNavIsland` (production dual-pill) is canonical. Mockup `DayStrip` and `SectionTabbar` are rejected per D2; surfaces provide chapters to FloatingNavIsland instead.
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
- `.docs/mockups/` demoted to exploration-only with `current/` and `_archive/` subdirs

### Notes

- No canonical entries yet. The four foundational audits are running in parallel; their findings will populate the first canonical entries and trigger a bump to `0.1.0`.
- Existing `.docs/design/*.md` files (DESIGN-SYSTEM.md, COMPONENT-INVENTORY.md, etc.) remain in place pending audit synthesis. They will move to `_archive/` and per-entry specs will become canonical.
