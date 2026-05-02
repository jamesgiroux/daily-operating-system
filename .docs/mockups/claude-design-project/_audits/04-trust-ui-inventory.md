# Audit 04 — Trust UI Inventory (current state)

## Files touching trust/provenance/freshness in src/
/Users/jamesgiroux/Documents/dailyos-repo/src/components/account/AccountHero.module.css
/Users/jamesgiroux/Documents/dailyos-repo/src/components/account/AccountHero.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/account/AccountProductsSection.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/account/AccountPullQuote.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/account/account-detail-utils.test.ts
/Users/jamesgiroux/Documents/dailyos-repo/src/components/context/AboutThisDossier.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/dashboard/DailyBriefing.test.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/dashboard/DailyBriefing.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/editorial/ChapterFreshness.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/editorial/ChapterHeading.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/AccountOutlook.test.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/IntelligenceQualityBadge.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/PersonCard.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/StakeholderGallery.module.css
/Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/StakeholderGallery.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/StakeholderGrid.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/health/AboutIntelligence.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/health/OnTrackChapter.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/health/SupportingTension.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/onboarding/chapters/DashboardTour.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/reports/ReportShell.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/ui/ProvenanceTag.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/components/work/WorkSurface.module.css
/Users/jamesgiroux/Documents/dailyos-repo/src/components/work/WorkSurface.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/features/settings-ui/SystemStatus.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/hooks/useConnectivity.ts
/Users/jamesgiroux/Documents/dailyos-repo/src/hooks/useDashboardData.ts
/Users/jamesgiroux/Documents/dailyos-repo/src/index.css
/Users/jamesgiroux/Documents/dailyos-repo/src/pages/AccountDetailEditorial.module.css
/Users/jamesgiroux/Documents/dailyos-repo/src/pages/AccountDetailEditorial.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/pages/AccountDetailPage.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/parity/phase3ContractRegistry.ts
/Users/jamesgiroux/Documents/dailyos-repo/src/router.tsx
/Users/jamesgiroux/Documents/dailyos-repo/src/types/index.ts

## Top 8 trust-related components
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/editorial/ChapterFreshness.tsx
  - Exported component(s) / function(s): ChapterFreshness
  - One-line role: Renders a compact chapter-level freshness strip with metadata fragments and an updated/enriched timestamp.
  - Trust concepts handled: freshness, stale fragment emphasis, enrichment recency.
  - Key props/inputs: enrichedAt, at, fragments
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/ui/ProvenanceTag.tsx
  - Exported component(s) / function(s): ProvenanceTag
  - One-line role: Shows a muted source label for intelligence items when the source is meaningful to users.
  - Trust concepts handled: provenance, source attribution, source disagreement.
  - Key props/inputs: itemSource, discrepancy
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/entity/IntelligenceQualityBadge.tsx
  - Exported component(s) / function(s): IntelligenceQualityBadge
  - One-line role: Displays a dot/label badge for structured quality or fallback enrichment freshness.
  - Trust concepts handled: freshness, completeness, sparse/developing/ready/fresh quality, new-signal indication.
  - Key props/inputs: quality, enrichedAt, showLabel
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/context/AboutThisDossier.tsx
  - Exported component(s) / function(s): AboutThisDossier
  - One-line role: Explains dossier source coverage, freshness, and data capture gaps in the account context tab.
  - Trust concepts handled: source coverage, freshness, data capture gap, verification need.
  - Key props/inputs: intelligence, meetingCount, uncharacterizedStakeholders
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/health/AboutIntelligence.tsx
  - Exported component(s) / function(s): AboutIntelligence
  - One-line role: Summarizes whether enrichment ran, what sources contributed, and whether source metadata is missing.
  - Trust concepts handled: enrichment freshness, source manifest, data capture gap, upstream Glean contribution.
  - Key props/inputs: intelligence, gleanSignals, fine
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/dashboard/DailyBriefing.tsx
  - Exported component(s) / function(s): DailyBriefing
  - One-line role: Composes the daily briefing and surfaces readiness, stale-cache handling, email sync as-of time, and lifecycle confidence.
  - Trust concepts handled: briefing readiness, cached freshness, as-of timestamp, confidence, stale lifecycle filtering.
  - Key props/inputs: data, freshness, workflowStatus
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/components/reports/ReportShell.tsx
  - Exported component(s) / function(s): ReportShell
  - One-line role: Wraps generated reports with stale-context warnings, generation state, and generated-at footer metadata.
  - Trust concepts handled: stale report warning, regeneration affordance, generated-at timestamp.
  - Key props/inputs: report, reportType, onReportGenerated
- File path: /Users/jamesgiroux/Documents/dailyos-repo/src/pages/AccountDetailEditorial.tsx
  - Exported component(s) / function(s): AccountDetailEditorial
  - One-line role: Deprecated account detail route that still wires account trust surfaces through hero, vitals, sections, and finis metadata.
  - Trust concepts handled: provenance slot plumbing, enrichment freshness, source references, intelligence feedback.
  - Key props/inputs: route accountId, useAccountDetail(), useIntelligenceFeedback()

## Documented intent (from the two design docs)
- Resolver trust is banded: Resolved starts at 0.85, ResolvedWithFlag spans 0.60-0.85, Suggestion spans 0.30-0.60, and NoMatch stays below 0.30.
- Persistence is stricter than display confidence: auto-linking requires Resolved, confidence at least 0.85, source-specific minimums, and at most one auto-linked entity per type.
- Context trust depends on scoped retrieval: account context must use stable entity IDs, not names, and prep context applies source and confidence guardrails before use.
- Consistency trust is explicit metadata: intelligence can be ok, corrected, or flagged, with findings carrying code, severity, claim text, evidence text, and auto-fix state.
- Repair is transparent but non-blocking: deterministic repair runs first, unresolved high-severity findings get one retry, and remaining flags persist for the UI to surface.

## Candidate primitives (PascalCase, job-named)
- TrustBandBadge
- FreshnessIndicator
- ProvenanceTag
- SourceCoverageLine
- ConfidenceScoreChip
- VerificationStatusFlag
- DataGapNotice
- AsOfTimestamp

## Candidate patterns (PascalCase, job-named)
- AboutThisIntelligencePanel
- DossierSourceCoveragePanel
- StaleReportBanner
- MeetingPrepReadinessStrip
- LifecycleVerificationRow
- ConsistencyFindingBanner
- EvidenceBackedClaimRow
- DataCaptureGapPanel

## Notes (max 5 bullets)
- The exact typed `rg --type tsx` form was unavailable in this environment; the collected list used the same pattern with explicit `*.ts`, `*.tsx`, and `*.css` globs.
- `DailyBriefing.tsx` accepts `freshness` but the visible read shows it aliased to `_freshness`; trust signals there are currently expressed through readiness, sync time, and lifecycle confidence.
- `ProvenanceTag` intentionally suppresses `pty_synthesis`, so synthesized intelligence has no visible provenance tag by default.
- `AccountDetailEditorial.tsx` is marked deprecated but still wires several trust-related slots and metadata surfaces.
- The strongest reusable candidates are already present as local one-off UI jobs rather than a consolidated trust vocabulary.
