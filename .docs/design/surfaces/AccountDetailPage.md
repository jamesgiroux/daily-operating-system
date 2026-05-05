# AccountDetailPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `AccountDetailPage`
**`data-ds-spec`:** `surfaces/AccountDetailPage.md`
**Source files:**
- `src/pages/AccountDetailPage.tsx`
- `src/pages/AccountDetailPage.module.css`
- `src/components/account/*`

## Job

AccountDetailPage is the customer-account dossier: health, context, room, work, reports, and trust/evidence framing for the selected account.

## Layout regions

1. Folio chrome and account-aware atmosphere.
2. Account hero with identity, account type, stakeholder/account metadata, and view switching.
3. Context and health chapters that summarize current state with evidence.
4. Relationship and stakeholder sections.
5. Work/report sections for actions, commitments, and generated reports.
6. Source/evidence affordances that expose freshness, provenance, and correction paths.

## Patterns and primitives

Consumes `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `ChapterHeading`, `FreshnessIndicator`, `ProvenanceTag`, `TrustBand`, `ClaimRow`, `ReceiptCallout`, and account-local dossier modules. Account-local modules stay local until reused by another surface.

## States

Supports loading, no-data, degraded intelligence, stale-source, and correction states. Evidence gaps must be visible rather than silently hidden.

## Notes

`AccountDetailEditorial` is legacy coverage only. The routed source of truth is `AccountDetailPage.tsx`.
