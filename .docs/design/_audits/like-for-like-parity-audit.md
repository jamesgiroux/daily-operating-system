# Like-for-like design system parity audit

**Date:** 2026-05-05
**Worktree:** `.codex/worktrees/design-system-audit`
**Scope:** `.docs/design` against runtime UI under `src/`

> **Status:** Baseline audit captured before the parity fix pass. Keep this file as the evidence trail for what was found. Current machine-verifiable status lives in `reference-fidelity.md` and `audit-reference.py --strict`.

## Post-fix status

- Token exports now resolve to the runtime source of truth: `src/styles/design-tokens.css`, `.docs/design/reference/_shared/styles/design-tokens.css`, and the compatibility entrypoint `.docs/design/reference/_shared/tokens.css` match after import resolution.
- Reference `data-ds-*` metadata is complete: no missing required attributes, no broken `data-ds-spec` links, and linked specs round-trip their declared `data-ds-name` / `data-ds-spec`.
- Router coverage is explicit in `surface-manifest.json`: the previous `ActionDetailPage`, `EmailsPage`, `HistoryPage`, and generic `ReportPage` gaps now have standalone references and specs. Dedicated report routes also have canonical specs.
- Splash/progress references now participate in the same manifest fidelity audit as routed pages, backed by `surfaces/StartupBriefingScreen.md`.
- Runtime promoted primitives/patterns now include `data-ds-tier` alongside existing `data-ds-name` and `data-ds-spec` attributes.
- Primitive and pattern specs now distinguish `canonical` shipped entries from `roadmap/planned` entries.

## Summary

The concern is valid. The design system is not yet a reliable like-for-like mirror of the product UI.

The biggest problem is not that counts are wildly wrong. It is that the artifacts do not share one source of truth:

- runtime tokens, docs tokens, and reference tokens disagree
- some "faithful" reference files point at deprecated or wrong source components
- many specs say "to be implemented" even though runtime code now exists
- other specs advertise components that still do not exist
- `data-ds-*` inspectability is claimed as required, but runtime adoption is partial and missing `data-ds-tier` entirely
- the fidelity checker only audits manifest entries, so it can mark the wrong thing clean

This creates exactly the failure mode you described: generated-from-scratch docs and references can look plausible while drifting from the product.

## High-priority findings

### 1. Reference tokens are not a direct lift from runtime

`.docs/design/reference/_shared/tokens.css` says it is a "Direct lift from `src/styles/design-tokens.css`" and "Single source of truth" (`reference/_shared/tokens.css:1-4`). It is not.

Examples:

- reference keeps legacy `--color-entity-*` tokens (`reference/_shared/tokens.css:41-46`)
- runtime uses `--color-account`, `--color-project`, `--color-person`, `--color-action`, `--color-self` (`src/styles/design-tokens.css:86-90`)
- reference `--page-max-width` is `1100px` (`reference/_shared/tokens.css:109`)
- runtime `--page-max-width` is `1180px` (`src/styles/design-tokens.css:241`)
- reference is missing runtime trust tokens (`src/styles/design-tokens.css:128-145`)
- reference defines values absent from runtime, including `--color-desk-espresso`, `--color-entity-*`, `--radius-sm/md/lg/xl`

Mechanical count:

- runtime `src/styles/design-tokens.css`: 166 unique custom properties
- docs token markdown: 101 unique custom-property mentions
- reference `_shared/tokens.css`: 95 unique custom properties
- reference `_shared/styles/design-tokens.css`: 166 unique custom properties

There are two token export files, and only one matches runtime. That is a source-of-truth bug.

### 2. Token docs are stale against shipped runtime

`tokens/color.md` still says trust tokens are proposed and "when added" (`tokens/color.md:88`, `tokens/color.md:118`, `tokens/color.md:125`). Runtime already defines them (`src/styles/design-tokens.css:128-145`) and components consume them:

- `src/components/ui/TrustBandBadge.module.css`
- `src/components/ui/ConfidenceScoreChip.module.css`
- `src/components/intelligence/ReceiptCallout.module.css`

The tint docs also overclaim completeness. `tokens/color.md` says every accent has standard tint stops `4, 5, 6, 7, 8, 10, 12, 15, 18, 20, 25, 30, 60`, but runtime is partial. Example: saffron has only `8, 10, 12, 15`; black has only `2, 3, 4, 8`.

### 3. The current account route has no faithful reference

The routed account page is `AccountDetailPage` (`src/pages/AccountDetailPage.tsx:1-8`). The old `AccountDetailEditorial` file explicitly says it is deprecated and no longer a route target (`src/pages/AccountDetailEditorial.tsx:1-6`).

But the manifest maps `account.html` to `AccountDetailEditorial`, and the reference itself declares:

```html
data-ds-name="AccountDetailEditorial"
data-ds-spec="surfaces/AccountDetailEditorial.md"
```

at `reference/surfaces/account.html:74`.

This is the clearest example of a reference render being clean against the wrong source.

### 4. Surface inventory and references disagree

`INVENTORY.md` marks report pages as gaps with no references (`INVENTORY.md:46`, `:50`, `:52`, `:59`, `:65`, `:67`, `:69`), but report reference HTML exists:

- `reference/surfaces/reports/account-health.html`
- `reference/surfaces/reports/book-of-business.html`
- `reference/surfaces/reports/ebr-qbr.html`
- `reference/surfaces/reports/monthly-wrapped.html`
- `reference/surfaces/reports/risk-briefing.html`
- `reference/surfaces/reports/swot.html`
- `reference/surfaces/reports/weekly-impact.html`

At baseline, the inventory also called `OnboardingFlow` a gap and treated the startup splash/progress references as outside the fidelity audit while chapter and splash references already existed.

This means the current inventory cannot be trusted to answer "do we have a reference for this surface?"

### 5. Routed page coverage has been closed

Resolved routed-page gaps from the baseline audit:

- `ActionDetailPage` (`/actions/$actionId`)
- `EmailsPage` (`/emails`)
- `HistoryPage` (`/history`)
- generic `ReportPage`
- `MeetingHistoryDetailPage` wrapper route, now explicitly classified as covered by `MeetingDetail`

The manifest now reports zero router routes missing manifest coverage and zero router routes acknowledged by gap status.

### 6. Reference surfaces contain generated HTML/style invention

The report references contain 1021 inline `style="..."` occurrences under `reference/surfaces/reports/*.html`. That is not a like-for-like mirror of CSS modules. It is hand-built or generated markup.

Non-report references also have 93 inline styles. The existing `audit-reference.py` catches some inline-style invention, but only for manifest entries and only in narrow conditions.

There are also invalid `data-ds-spec` links in references:

- `surfaces/AccountDetailEditorial.md`
- `surfaces/ActionsPage.md`
- `surfaces/EbrQbrPage.md`
- `surfaces/MePage.md`
- `surfaces/MeetingDetailPage.md`
- `surfaces/ProjectsPage.md`
- `surfaces/SettingsPage.md`
- `surfaces/WeekPage.md`
- `surfaces/WeeklyImpactPage.md`
- `tokens/glass.md`
- `tokens/layout.md`
- `tokens/radius.md`
- `tokens/shadows.md`
- `tokens/z-index.md`

The reference library points users to specs that do not exist.

### 7. Runtime `data-ds-*` adoption is incomplete

`SYSTEM-MAP.md` says every rendered design system element in reference and `src/` should carry required attributes (`data-ds-tier`, `data-ds-name`, `data-ds-spec`) (`SYSTEM-MAP.md:52-87`).

Mechanical source scan:

- `data-ds-name` in `src/components src/pages src/features`: 28 matches
- `data-ds-spec` in `src/components src/pages src/features`: 28 matches
- `data-ds-tier` in `src/components src/pages src/features`: 0 matches

Several canonical runtime roots are missing instrumentation:

- `FolioBar`
- `FloatingNavIsland`
- `AtmosphereLayer`
- `MarginSection` / `MarginGrid`
- `ChapterHeading`
- `IntelligenceQualityBadge`
- local `TypeBadge` inside `AccountHero`

The reference layer also uses non-canonical names/specs, for example:

- `MeetingDetailPage` / `surfaces/MeetingDetailPage.md` (`reference/surfaces/meeting.html:51`)
- `SettingsPage` / `surfaces/SettingsPage.md` (`reference/surfaces/settings.html:51`)
- `AccountDetailEditorial` / `surfaces/AccountDetailEditorial.md` (`reference/surfaces/account.html:74`)

But the actual surface specs are `MeetingDetail.md`, `Settings.md`, and no `AccountDetailEditorial.md` exists.

### 8. Primitive and pattern docs are semantically stale

The primitive/pattern indexes count correctly enough, but many statuses and source sections are stale.

Implemented runtime exists while docs still say "to be implemented":

- `Pill` doc says `src/components/ui/Pill.tsx` is future work (`primitives/Pill.md:67`); runtime exists and emits `data-ds-name` / `data-ds-spec` (`src/components/ui/Pill.tsx:69-73`)
- `TrustBand` doc says future implementation (`patterns/TrustBand.md:75`); runtime exists (`src/components/intelligence/TrustBand.tsx:76-83`)
- same stale pattern applies to `ClaimRow`, `ReceiptCallout`, `SurfaceMasthead`, `GlanceRow`, `DayChart`, `AskAnythingDock`, `EntityPortraitCard`, `ThreadMark`, `MeetingStatusPill`, `SourceCoverageLine`, `ConfidenceScoreChip`, `VerificationStatusFlag`, `DataGapNotice`, `AsOfTimestamp`, `Switch`, `Segmented`, `RemovableChip`, `GlanceCell`, and `EntityChip`

Documented entries still missing runtime:

- `Lead`
- `MeetingSpineItem`
- `FolioActions`
- `AgendaThreadList`
- `PredictionsVsRealityGrid`
- `SignalGrid`
- `EscalationQuote`
- `FindingsTriad`
- `ChampionHealthBlock`
- `CommitmentRow`
- `RoleTransitionRow`
- `InlineInput`
- `ConsistencyFindingBanner`

Runtime components acting like primitives/patterns but lacking specs include:

- `AccuracyPrompt`
- `ProvenanceLabel`
- `IntelligenceCorrection`
- `IntelligenceFeedback`
- `HealthBadge`
- `StatusDot`
- `DimensionBar`
- `TalkBalanceBar`
- `MeetingRow`
- `MeetingCard`
- `ActionRow`
- `EntityRow`
- `AccountViewSwitcher`

Resolved since this audit was first written:

- `VitalsStrip` is now promoted as `patterns/VitalsStrip.md` with a canonical reference card covering read-only and editable variants.

### 9. Runtime still has hardcoded design values

The source still has many raw values outside token declarations:

- 127 raw hex matches outside `src/styles/design-tokens.css`
- 206 raw `rgb/rgba` matches outside `src/styles/design-tokens.css`
- 275 numeric `border-radius` matches
- 31 numeric `z-index` matches

Examples:

- `src/index.css` hardcodes Tailwind v4 bridge colors instead of aliasing canonical tokens (`src/index.css:16-36`, `:52-59`)
- `src/pages/meeting-intel.module.css:1296-1297` uses legacy/undefined `--color-turmeric` and `--color-cream-wash` fallbacks
- `src/components/work/WorkSurface.module.css` hardcodes accent colors
- `src/components/entity/StrategicLandscape.module.css:83` hardcodes `#4a6186`
- `src/lib/entity-utils.ts` hardcodes rgba tints that match existing design tokens

Some raw values are intentional local geometry, but the current volume is too high for "never hardcode colors" to be true.

## Tooling blind spots

`audit-reference.py` is useful, but incomplete:

1. It only audits entries in `surface-manifest.json`.
2. It can mark `account.html` clean because the manifest points at deprecated `AccountDetailEditorial`.
3. It does not cross-check router routes against manifest primaries.
4. It does not fail on broken `data-ds-spec` links.
5. It does not check source `data-ds-tier` adoption.
6. It does not police duplicated token exports.

The current generated `reference-fidelity.md` says 34 surfaces are clean against the manifest.

## Recommended fix order

1. **Restore token source-of-truth.** Replace or delete `reference/_shared/tokens.css`; make every reference import the exact runtime token file copy, or generate it from `src/styles/design-tokens.css`.
2. **Fix route-to-reference mapping.** Completed for `AccountDetailPage`, redirect/generic routes, `ActionDetailPage`, `EmailsPage`, `HistoryPage`, and wildcard `ReportPage`.
3. **Regenerate inventory from router + manifest.** `INVENTORY.md` should not be hand-maintained separately from reference coverage.
4. **Normalize `data-ds-*`.** Add `data-ds-tier/name/spec` to canonical runtime roots first, then mirror those exact names in reference HTML.
5. **Update implemented specs.** Promote implemented primitives/patterns from "proposed/to be implemented" and update source paths.
6. **Separate roadmap entries from canonical entries.** Missing runtime entries should be clearly marked roadmap, hidden from "yours probably exists" language, or implemented.
7. **Purge generated reference invention.** Regenerate `_shared/styles/*.module.css` strictly from scoped source CSS, and move report reference inline styles into source-mirrored classes or explicitly label them as mockups.
8. **Tokenize runtime hardcodes by category.** Start with undefined/legacy token fallbacks, then colors, then z-index/shadows/radius/motion.
9. **Tighten the audit gate.** Make `audit-reference.py` cross-check router routes, manifest primaries, broken spec links, duplicate token exports, and required `data-ds-*` attributes.

## Suggested acceptance criteria

- `diff -u src/styles/design-tokens.css .docs/design/reference/_shared/styles/design-tokens.css` is the only token parity check needed, or there is a documented generated artifact path.
- No reference `data-ds-spec` points to a missing file.
- Every route in `src/router.tsx` is either in `surface-manifest.json` or explicitly classified as covered-by/redirect/internal.
- `account.html` audits against `AccountDetailPage`, not deprecated `AccountDetailEditorial`.
- `rg "data-ds-tier" src/components src/pages src/features` returns promoted primitives, patterns, and surfaces.
- `audit-reference.py --strict --enforce-baseline` fails on route/manifest drift, not just class drift.
- `INVENTORY.md` can be regenerated without changing meaning.
