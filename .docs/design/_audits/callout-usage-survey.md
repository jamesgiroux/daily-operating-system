# Callout-Shaped Surface Usage Survey

**Date:** 2026-05-10  
**Scope:** `src/components/`, `.docs/design/patterns/`, `.docs/design/primitives/`, `.docs/design/reference/`  
**Definition:** Tinted/bordered boxes that frame label + body ± footer, used to draw attention.

---

## A. Already-Spec'd Callout-Family Patterns

| Pattern | Job | Tone Color | Label | Footer/Signoff | Dismiss/Action | Status |
|---------|-----|-----------|-------|----------------|---|--------|
| **ReceiptCallout** | Inspectable receipt for claim drill-in; surfaces resolver confidence, consistency findings, source attribution, freshness, contradictions | Per band: sage-12, saffron-12, terracotta-12 | Yes (mono kicker) | Freshness chain + contradictions | Confirm / Correct / Dismiss / Flag | proposed |
| **StaleReportBanner** | Warn when generated report based on stale intelligence; explain staleness with regeneration affordance | saffron-12 (info) / terracotta-8 (warning) | Yes (primary copy) | AsOfTimestamp comparing report vs. enrichment | Regenerate / Refresh action | integrated |
| **ConsistencyFindingBanner** | Surface consistency finding inline beside claim; explain whether corrected or flagged | terracotta-8 (flagged) / saffron-12 (corrected) | Yes (VerificationStatusFlag) | Finding code + evidence text | Inspect / Correct / Dismiss | integrated |
| **EscalationQuote** | Lift turning-point quote with attribution; editorial emphasis for meeting analysis | saffron-15 background, turmeric border | Yes (speaker + role) | Timestamp with optional transcript link | Transcript deep-link (passive) | proposed |
| **TrustBand** | Default trust render; composition of band judgment + provenance + freshness | Per band colors | Via TrustBandBadge | Via FreshnessIndicator | Drill to ReceiptCallout | proposed |
| **AboutThisIntelligencePanel** | Explain chapter-level trust; sources, enrichment, gaps without requiring receipt inspection | Neutral / caution / incomplete tone variants | TrustBandBadge leading | Timestamp + source lines + gap notices | None (informational) | integrated |
| **DossierSourceCoveragePanel** | Explain dossier-level source coverage across account/project/person; show capture gaps | Neutral variant | SourceCoverageLine labels | Timestamp + source list + DataGapNotice | None (informational) | integrated |

---

## B. Component-Local Callouts (CSS Modules Without Spec)

| Component | Location | Treatment | Tone | Label | Footer | Actions | Job |
|-----------|----------|-----------|------|-------|--------|---------|-----|
| **TemplateSuggestionBanner** | `entity/TemplateSuggestionBanner.module.css` | Flex container; `--color-rule-light` border; `--color-paper-warm-white` bg; padding lg | warm-white + light rule | Serif title + sans body | None | Muted + primary actions | Suggest template adoption |
| **StateBlock** | `editorial/StateBlock.module.css` | Left-border accent (3px, color-bound via `--state-block-color`); item rows with left padding; comment on hover | Token-driven (sage / terracotta) | Mono uppercase label | None | Dismiss chevron on hover | Contextualize working / struggling state |
| **DataGapNotice** | `ui/DataGapNotice.module.css` | Inline flex; pill-shaped (border-radius 999px); mono label; `--color-desk-charcoal-4` (info) or `--color-trust-use-with-caution-8` (warning) | Info: charcoal-4 / Warning: saffron | Mono compact message | None | None (inline) | Inline gap warning |
| **AboutThisDossier.card** | `context/AboutThisDossier.module.css` | `.card`: charcoal-4 bg; left 2px tertiary border; rounded right corners; padding xl | desk-charcoal-4 + tertiary left border | Mono uppercase eyebrow | Mono text body | None | Meta explanation card |
| **ReportShell.staleBanner** | `reports/report-shell.module.css` | Flex row; turmeric-15 bg; padding, no explicit border | turmeric-15 | Implicit in message text | None | Regenerate button (right-aligned) | Warn of report staleness |
| **ReportShell.errorBanner** | `reports/report-shell.module.css` | Flex row; terracotta-8 bg; padding, no explicit border | terracotta-8 | Implicit in message text | None | None | Error display |

---

## C. Inline Callout Treatment in JSX

| Location | Treatment Pattern | Tone | Label | Composition | Job |
|----------|------------------|------|-------|-------------|-----|
| `ClaimRow` expansion state | 1px border + `--color-paper-warm-white` bg; left border 3px per band color | Per trust band (likely-current / caution / verification) | Resolved via receipt | Label (mono) + body + freshness + trust signals | Inline receipt inspection trigger |
| `Pull-quote-left` (primitives.css) | 3px turmeric left border; turmeric-5 gradient background; padding | turmeric | Mono uppercase pq-label | Label + serif italic quote text | Editorial pull quote with accent |
| Pill variants (Pill.module.css) | Pill-shaped (border-radius 100px); background tints (sage-15 / turmeric-15 / terracotta-15 / larkspur-15 / olive-10 / eucalyptus-10); inline-flex | Tone-driven (6 variants) | None (status indicator only) | Icon + compact sans text | Status signaling (compact) |
| ConfidenceScoreChip | Background tints (trust band colors at -15 opacity) | Per band | None | Numeric confidence + optional icon | Trust signal chip |
| Type badges (in reference.css) | Inline pill; background tone-specific; padding xs | customer / internal / partner | None | Icon + text + dropdown chev | Entity type signaling |

---

## Synthesis

**Distinct callout variants in codebase:** ~12–15

**Key dimensions that vary:**
1. **Tone/color:** Trust bands (sage-12 / saffron-12 / terracotta-12) + desk-charcoal-4 + -15 tints for pill backgrounds
2. **Has label?** Most do (mono uppercase for metadata, serif for titles); DataGapNotice and pills omit
3. **Has footer/signoff?** Depends on scope: ReceiptCallout has freshness chain; StaleReportBanner has timestamp; TrustBand omits
4. **Has dismiss/action?** Receipt has Confirm/Correct/Dismiss/Flag; Banner has Regenerate/Refresh; StateBlock has hover dismiss; most panels are read-only
5. **Size/density:** Compact (DataGapNotice, pills) vs. expanded (ReceiptCallout, panels) vs. full-width (banners)
6. **Border treatment:** Left-accent 3px (StateBlock, ReceiptCallout); full 1px borders (TemplateSuggestion); gradient background (pull-quote); no border (banner, pill)
7. **Shape:** Rounded corners (cards, receipt, template suggestion) vs. pill-shaped (DataGapNotice, pills) vs. angular (banners)

**Minimum-viable Callout primitive API must accommodate:**

- **Tone variant:** oneOf(trust-band, neutral, warning, caution, verification, custom-color)
- **Composition slots:** label (optional, mono-ish), body/content (required), footer (optional, secondary typography), actions (optional, button group)
- **Border treatment:** full, left-accent-only, gradient-bg, or none
- **Density:** compact, default, expanded
- **Dismissibility:** optional dismiss affordance (on hover or always-visible)
- **Use in context:** Within panels (AboutThisIntelligence, DossierSourceCoverage), inline (ClaimRow expansion), full-width (banners), and as composed primitives (Pills, Badges)

The Callout primitive should abstract the common "tinted container with labeled content" pattern and expose these dimensions as props, allowing existing patterns (ReceiptCallout, StaleReportBanner, AboutThisIntelligencePanel) to compose and style without reimplementing the base callout geometry or token logic.
