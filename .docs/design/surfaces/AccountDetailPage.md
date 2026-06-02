# AccountDetailPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-06-01
**`data-ds-name`:** `AccountDetailPage`
**`data-ds-spec`:** `surfaces/AccountDetailPage.md`
**Source files:**
- `src/pages/AccountDetailPage.tsx`
- `src/pages/AccountDetailPage.module.css`
- `src/components/composition/*`
- `src/components/account/*`

## Job

AccountDetailPage is the account dossier rendered from a substrate-authored composition. The Account producer decides the section outline and block sequence; the routed React page renders projected blocks inside the magazine shell.

The surface is one composed scroll. Health, context, room, work, reports, trust, and evidence framing are sections in the same document rather than tabs or independently mounted views.

## Layout Regions

1. Folio chrome, account-aware atmosphere, and floating chapter navigation derived from projected section ids.
2. Headline masthead with account identity, account type, health, freshness, trust summary, lifecycle, renewal, and primary source summary.
3. Outlook section for renewal confidence, health snapshot, top risks, and growth signals.
4. State of Play section for claim summaries, evidence rows, correction affordances, and unknown-block fallback.
5. The Room section for relationship map, stakeholder roles, review gaps, and render-policy masking.
6. What's Next section for projected next-step actions and source labels.
7. Watch List section for background risk signals, stale-source treatment, and optional semantic collapse when dense.
8. Value & Commitments section for delivered value, commitments, evidence, and feedback targets.
9. Strategic Landscape section for safe text/markdown summaries with source tags.
10. The Record section for timeline evidence and manual user-authored entries.
11. The Work and Reports sections for service-owned work rows, empty/degraded states, and report availability.
12. Finite ending using `FinisMarker`.

## Composition Contract

- The rendered chapter list comes from `ProjectedComposition.sections`; hardcoded Health/Context/Work chapter arrays are retired.
- Known blocks use typed React block components. Unknown/custom blocks render through the safe fallback banner, never raw dropped payload.
- Blocks display trust, freshness, provenance, and correction affordances when the projected payload carries those fields.
- Account snapshot facts are either explicitly non-sensitive identity/display fields or field-wrapped with sensitivity, source freshness, provenance kind, and trust treatment.
- Background salience sections may collapse only with semantic `<button>` controls, visible focus state, `aria-expanded`, and content preserved in document order.

## Patterns And Primitives

Consumes `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `FinisMarker`, `FreshnessIndicator`, `ProvenanceTag`, `TrustBandBadge`, `HealthBadge`, `StatusDot`, `EntityChip`, `Pill`, and account-local implementation components when their API matches the projected block payload.

`AccountViewSwitcher` is retired for the routed Account Detail surface. It can remain in source during migration only as legacy coverage; it is not the target pattern for this page.

## States

Supports loading, command error, empty composition, no-data sections, degraded provenance, masked fields, stale-source caution, unknown/custom fallback, and correction states. Evidence gaps must render visibly rather than being hidden by the frontend.

## Reference

`.docs/design/reference/surfaces/account.html` is the variant-D composed-scroll target for v1.5.0 W1. It intentionally leads source migration: until W1 implementation lands, source may still contain the old three-view template.
