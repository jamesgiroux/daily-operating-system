/**
 * AccountDetailPage — Clean rebuild of the account detail page.
 *
 * Single flat route, state-based view switching, no child routes.
 * Built step by step per plan at ~/.claude/plans/deep-wiggling-hearth.md.
 *
 * Step 5: All 3 views rendered, inactive hidden via display:none.
 * Preserves scroll + form state + pending fetches on tab switch.
 */
import { useParams } from "@tanstack/react-router";
import { useAccountDetailPage } from "@/hooks/useAccountDetailPage";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { ChapterFreshness } from "@/components/editorial/ChapterFreshness";
import { QuoteWallPlaceholder } from "@/components/editorial/QuoteWallPlaceholder";
import { AboutThisDossier } from "@/components/context/AboutThisDossier";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import { AccountBreadcrumbs } from "@/components/account/AccountBreadcrumbs";
import { AccountRolloverPrompt } from "@/components/account/AccountRolloverPrompt";
import { AccountDialogs } from "@/components/account/AccountDialogs";
import { AccountViewSwitcher } from "@/components/account/AccountViewSwitcher";
// View 1 — Health & Outlook
// AccountOutlook (legacy 3-section component) is no longer rendered on the
// Health tab — OutlookPanel is the replacement. The component still lives in
// the codebase for possible Context-tab reuse, but it is not imported here.
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
import { AccountPortfolioSection } from "@/components/account/AccountPortfolioSection";
import { AccountProductsSection } from "@/components/account/AccountProductsSection";
import { SentimentHero } from "@/components/health/SentimentHero";
// DOS-203: Health-tab chapter components.
import { TriageSection, hasTriageContent } from "@/components/health/TriageSection";
import { DivergenceSection, hasDivergenceContent } from "@/components/health/DivergenceSection";
import { OutlookPanel, renewalCallVerdict } from "@/components/health/OutlookPanel";
import { SupportingTension } from "@/components/health/SupportingTension";
import { AboutIntelligence } from "@/components/health/AboutIntelligence";
import { OnTrackChapter } from "@/components/health/OnTrackChapter";
import { RiskBriefingStatus } from "@/components/health/RiskBriefingStatus";
// View 2 — Context
import { AccountPullQuote } from "@/components/account/AccountPullQuote";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { StakeholderGrid } from "@/components/entity/StakeholderGrid";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
import { CommercialShape } from "@/components/context/CommercialShape";
import { RelationshipFabric } from "@/components/context/RelationshipFabric";
// View 3 — The Work (DOS-13: workbench, not todo list)
import {
  NumberedFocusList,
  ProgramPill,
  ProgramPillRow,
  CommitmentCard,
  SuggestionCard,
  RecentlyLandedList,
  RecentlyLandedRow,
  ReportCard,
  ReportGrid,
  ReportFooterNote,
  NudgeList,
  NudgeRow,
  NudgeLeaveAsIs,
  WorkButton,
  type FocusItem,
} from "@/components/work/WorkSurface";
import { getAccountReports } from "@/lib/report-config";
import { buildAccountVitals } from "@/components/account/account-detail-utils";
import { formatShortDate } from "@/lib/utils";

import shared from "@/styles/entity-detail.module.css";
import pageStyles from "./AccountDetailPage.module.css";

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const page = useAccountDetailPage(accountId);

  if (page.loading) return <EditorialLoading />;
  if (page.error || !page.detail) return <EditorialError message={page.error ?? "Account not found"} onRetry={page.acct.load} />;

  const { detail, intelligence, acct, preset, activeView } = page;
  const fb = page.feedback;

  // ─── View 1: Health & Outlook ───────────────────────────────────────────
  // DOS-203: editorial IA matching .docs/mockups/account-health-*.html:
  //   1. Your Assessment (sentiment hero, id="your-assessment")
  //   2. Needs attention — triage cards + divergence findings
  //        (or On Track, id="on-track", in the fine state)
  //   3. Outlook: renewal — OutlookPanel + IntelligenceCorrection slot
  //   4. Supporting — computed score vs signal trend + dimension bars
  //        (id="relationship-health", renders only when intelligence.health)
  //   5. Portfolio rollup (parents only, id="portfolio")
  //   6. Products (id="products", only when detail.products.length > 0)
  //   7. About this intelligence — source manifest + freshness meta card
  //   8. Finis marker
  //
  // Fine state: when triage + divergences are both empty, chapter 2 collapses
  // into the editorial "On Track" chapter per fine mockup.
  //
  // The legacy AccountOutlook 3-section component (renewal confidence / growth
  // opportunities / contract context) has been removed from this view —
  // OutlookPanel is the mockup-faithful replacement. AccountOutlook is retained
  // for potential reuse elsewhere.
  const renderHealthView = () => {
    const findings = intelligence?.consistencyFindings ?? [];
    const glean = acct.gleanSignals;
    const showTriage = hasTriageContent(intelligence, glean);
    const showDivergence = hasDivergenceContent(findings, glean);
    const isFineState = !!intelligence && !showTriage && !showDivergence;

    return (
      <>
        {/* DOS-228 Wave 0e Fix 4: pinned risk-briefing status at top of
            Health tab. Renders nothing when there's no active job; shows
            running/failed states with a retry affordance on failure. */}
        <RiskBriefingStatus
          job={acct.riskBriefingJob}
          onRetry={acct.retryRiskBriefing}
        />
        {/* Chapter 1: Sentiment hero — "Your Assessment" in the mockup.
            Wrapped in a section so the chapter-nav anchor (id matches
            buildHealthChapters → "your-assessment") resolves cleanly. */}
        <section id="your-assessment">
          <SentimentHero
            view={acct.sentiment}
            onSetSentiment={acct.setUserHealthSentiment}
            onAcknowledgeStale={acct.acknowledgeSentimentStale}
          />
        </section>

        {/* Chapters 2-3 (full state) OR On Track chapter (fine state) */}
        {isFineState ? (
          <MarginSection id="on-track" label={<>On<br/>Track</>}>
            <OnTrackChapter intelligence={intelligence} accountSizeLabel={detail.lifecycle ?? detail.accountType ?? null} />
          </MarginSection>
        ) : (
          <MarginSection id="needs-attention" label={<>Needs<br/>attention</>}>
            {showTriage && (
              <TriageSection intelligence={intelligence} gleanSignals={glean} />
            )}
            {showDivergence && <DivergenceSection findings={findings} gleanSignals={glean} />}
          </MarginSection>
        )}

        {/* Chapter 4: Outlook — the chapter title IS the verdict
            ("The Call: Renewal" / "Churn risk" / "Expansion"), computed
            from renewalOutlook.confidence + expansionPotential. The gutter
            "Outlook" stays as the orientation marker. */}
        {intelligence && (intelligence.renewalOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
          <MarginSection id="outlook" label="Outlook">
            <ChapterHeading title={`The Call: ${renewalCallVerdict(intelligence.renewalOutlook)}`} />
            <OutlookPanel intelligence={intelligence} />
          </MarginSection>
        ) : null}

        {/* Chapter 5: The Read — the computed-score-vs-signal-trend chapter.
            Gutter "The Read" (pairs with "The Call" in Outlook as the two
            verdict chapters). Inline 28px serif h2 "Health Score vs. Signals"
            is descriptive, not jargon. AccountHealthSection removed — its
            dimension block is a legacy duplicate of SupportingTension's
            own dimension grid. ChapterFreshness strip removed (not in
            mockup). */}
        {intelligence?.health && (
          <MarginSection id="relationship-health" label="The Read">
            <ChapterHeading title="Health Score vs. Signals" />
            <SupportingTension intelligence={intelligence} gleanSignals={glean} />
          </MarginSection>
        )}

        {/* Portfolio rollup (parent accounts only) — continuity chapter. */}
        {detail.isParent && detail.children.length > 0 && (
          <AccountPortfolioSection children={detail.children} intelligence={intelligence} />
        )}

        {/* Products previously rendered here — moved to the Context tab where
            it sits alongside Technical shape / Commercial shape. Products are
            a contractual/technical surface, not a health signal. */}

        {/* Chapter 6: About this intelligence */}
        <MarginSection id="about-intelligence" label={<>About this<br/>intelligence</>} reveal={false}>
          <ChapterHeading
            title="About this intelligence"
            variant="reference"
          />
          <AboutIntelligence intelligence={intelligence} gleanSignals={glean} fine={isFineState} />
        </MarginSection>

        {/* Chapter 7: Finis */}
        <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
      </>
    );
  };

  // ─── View 2: Context ────────────────────────────────────────────────────
  // DOS-18: 9-chapter IA — Thesis / The Room / What matters / What we've built /
  // Their voice / Commercial shape / Technical shape / Relationship fabric /
  // About this dossier. Timeline + Files stay inline to preserve existing
  // scroll affordances until The Work migration (DOS-13).
  const renderContextView = () => {
    // Freshness fragment helpers derived from existing data. No new schema.
    const manifest = intelligence?.sourceManifest ?? [];
    // DOS-233 Codex fix: prefer the backend COUNT(*) when available; fall
    // back to the manifest-derived count for older snapshots.
    const transcriptCount =
      detail.transcriptTotalCount
      ?? manifest.filter((m) => (m.format ?? "").toLowerCase().includes("transcript")).length;
    // DOS-233: About-this-dossier counts previously used `acct.events` (lifecycle
    // events — churn/renewal records) instead of meetings, producing obviously
    // wrong figures like "0 meetings on record" on active accounts. The source
    // of truth for meetings linked to the account is `meeting_entities` joined
    // with `meetings` (see db/accounts.rs).
    //
    // DOS-233 Codex fix: `recentMeetings` is capped at 10 for preview rendering,
    // so an account with 47 meetings previously stalled at "10 meetings on
    // record". The backend now exposes `meetingTotalCount` /
    // `transcriptTotalCount` (unbounded COUNT(*) queries). Fall back to
    // `recentMeetings.length` only when the total is not yet available.
    const meetingCount =
      detail.meetingTotalCount ?? detail.recentMeetings?.length ?? 0;
    const thesisFragments: string[] = [];
    if (meetingCount) thesisFragments.push(`Synthesized from ${meetingCount} meeting${meetingCount === 1 ? "" : "s"}`);
    if (transcriptCount) thesisFragments.push(`${transcriptCount} transcript${transcriptCount === 1 ? "" : "s"}`);

    const stakeholders = detail.stakeholdersFull ?? [];
    const stakeholdersAssessed = stakeholders.filter((s) => s.assessment && s.assessment.trim().length > 0).length;
    const stakeholdersNeedingVerification = stakeholders.length - stakeholdersAssessed;
    const roomFragments: (string | { text: string; stale?: boolean })[] = [];
    if (stakeholders.length) roomFragments.push(`${stakeholders.length} people`);
    if (stakeholdersAssessed) roomFragments.push(`${stakeholdersAssessed} with assessments`);
    if (stakeholdersNeedingVerification > 0) roomFragments.push({ text: `${stakeholdersNeedingVerification} need verification`, stale: true });

    const priorityCount = intelligence?.strategicPriorities?.length ?? 0;
    const competitorCount = intelligence?.competitiveContext?.length ?? 0;
    const expansionCount = intelligence?.expansionSignals?.length ?? 0;
    const whatMattersFragments: string[] = [];
    if (priorityCount) whatMattersFragments.push(`${priorityCount} strategic priorit${priorityCount === 1 ? "y" : "ies"}`);
    if (competitorCount) whatMattersFragments.push(`${competitorCount} competitive mention${competitorCount === 1 ? "" : "s"}`);
    if (expansionCount) whatMattersFragments.push(`${expansionCount} expansion signal${expansionCount === 1 ? "" : "s"}`);

    const valueCount = intelligence?.valueDelivered?.length ?? 0;
    const metricsCount = intelligence?.successMetrics?.length ?? 0;
    const builtFragments: string[] = [];
    if (valueCount) builtFragments.push(`${valueCount} value statement${valueCount === 1 ? "" : "s"}`);
    if (metricsCount) builtFragments.push(`${metricsCount} success metric${metricsCount === 1 ? "" : "s"}`);

    const featureAdoption = intelligence?.productAdoption?.featureAdoption ?? [];
    const technicalFragments: string[] = [];
    if (detail.technicalFootprint?.openTickets != null) technicalFragments.push(`${detail.technicalFootprint.openTickets} open ticket${detail.technicalFootprint.openTickets === 1 ? "" : "s"}`);
    if (featureAdoption.length) technicalFragments.push(`${featureAdoption.length} features active`);

    const hasWhatMatters = !!(priorityCount || competitorCount || intelligence?.organizationalChanges?.length || intelligence?.blockers?.length);
    const hasBuilt = !!(valueCount || metricsCount || intelligence?.openCommitments?.length);

    return (
      <>
        {/* Chapter 1: Thesis — pull quote + synthesized-from meta */}
        {intelligence && (
          <section id="thesis">
            <AccountPullQuote
              intelligence={intelligence}
              variant="thesis"
              freshnessFragments={thesisFragments}
            />
          </section>
        )}

        {/* Chapter 2: The Room — v1.2.1 rebuild matching the Context mockup:
            primary/secondary grid, multi-role chip editor wired to atomic
            add/remove, "+N more associated" tier-2 row, internal team grid. */}
        <MarginSection id="the-room" label={<>The<br/>Room</>}>
          <StakeholderGrid
            stakeholders={detail.stakeholdersFull}
            accountTeam={detail.accountTeam}
            accountName={detail.name ?? undefined}
            chapterTitle="The Room"
            chapterFreshness={
              <ChapterFreshness
                enrichedAt={intelligence?.enrichedAt}
                fragments={roomFragments}
              />
            }
            onAddRole={acct.addStakeholderRole}
            onRemoveRole={acct.removeStakeholderRole}
            onRemoveTeamMember={acct.handleRemoveTeamMember}
            suggestions={acct.suggestions}
            onAcceptSuggestion={acct.acceptSuggestion}
            onDismissSuggestion={acct.dismissSuggestion}
          />
        </MarginSection>

        {/* Chapter 3: What matters to them */}
        {intelligence && hasWhatMatters && (
          <MarginSection id="what-matters" label={<>What<br/>matters</>}>
            <ChapterHeading
              title="What matters to them"
              freshness={<ChapterFreshness enrichedAt={intelligence.enrichedAt} fragments={whatMattersFragments} />}
              feedbackSlot={
                page.accountId ? (
                  <IntelligenceCorrection
                    entityId={page.accountId}
                    entityType="account"
                    field="strategic_priorities"
                  />
                ) : null
              }
            />
            <StrategicLandscape
              intelligence={intelligence}
              onUpdateField={page.handleUpdateIntelField}
              getItemFeedback={fb.get}
              onItemFeedback={fb.submit}
            />
          </MarginSection>
        )}

        {/* Chapter 4: What we've built together */}
        {intelligence && hasBuilt && (
          <MarginSection id="value-commitments" label={<>What we've<br/>built</>}>
            <ChapterHeading
              title="What we've built together"
              freshness={<ChapterFreshness enrichedAt={intelligence.enrichedAt} fragments={builtFragments} />}
            />
            <ValueCommitments
              intelligence={intelligence}
              onUpdateField={page.handleUpdateIntelField}
              getItemFeedback={fb.get}
              onItemFeedback={fb.submit}
            />
          </MarginSection>
        )}

        {/* Chapter 5: Their voice — quote wall placeholder (DOS-205) */}
        <MarginSection id="their-voice" label={<>Their<br/>voice</>}>
          <ChapterHeading
            title="Their voice"
            freshness={
              <ChapterFreshness
                enrichedAt={intelligence?.enrichedAt}
                fragments={["Pull-quote wall in progress"]}
              />
            }
          />
          <QuoteWallPlaceholder />
        </MarginSection>

        {/* Chapter 6: Commercial shape — reference weight, most fields are gaps today */}
        <MarginSection id="commercial-shape" label={<>Commercial<br/>shape</>}>
          <ChapterHeading
            title="Commercial shape"
            variant="reference"
            freshness={
              <ChapterFreshness
                enrichedAt={intelligence?.enrichedAt}
                fragments={[
                  { text: "Several fields unverified — see gaps below", stale: true },
                ]}
              />
            }
          />
          <CommercialShape detail={detail} onUpdateField={page.saveAccountField} />
        </MarginSection>

        {/* Chapter 7: Technical shape — promoted footprint + feature list (reference weight). */}
        {/* Always renders: when footprint is null, AccountTechnicalFootprint emits gap rows. */}
        <MarginSection id="technical-shape" label={<>Technical<br/>shape</>}>
          <ChapterHeading
            title="Technical shape"
            variant="reference"
            freshness={
              <ChapterFreshness
                at={detail.technicalFootprint?.sourcedAt ?? intelligence?.enrichedAt}
                fragments={technicalFragments}
              />
            }
          />
          <AccountTechnicalFootprint
            footprint={detail.technicalFootprint ?? null}
            variant="chapter"
            featureAdoption={featureAdoption}
            // DOS-231: gap rows expose a "Capture now" affordance. Until
            // the structured editor lands with DOS-207, we prompt inline
            // for the value, persist through
            // `update_technical_footprint_field`, and refresh. This gives
            // the Intelligence Loop a real signal + updated footprint
            // immediately instead of a silent console log.
            onCaptureGap={(field) => { void page.captureTechnicalFootprintField(field); }}
          />
        </MarginSection>

        {/* Products — moved here from the Health tab. Technical/commercial
            surface of the account; belongs alongside Technical shape. */}
        {(detail.products?.length ?? 0) > 0 && (
          <MarginSection id="products" label="Products">
            <AccountProductsSection
              accountId={detail.id}
              products={detail.products ?? []}
              getFeedback={fb.get}
              onFeedback={fb.submit}
              onRefresh={acct.load}
              silentRefresh={acct.silentRefresh}
            />
          </MarginSection>
        )}

        {/* Chapter 8: Relationship fabric — advocacy, beta, NPS history */}
        <MarginSection id="relationship-fabric" label={<>Relationship<br/>fabric</>}>
          <ChapterHeading
            title="Relationship fabric"
            freshness={
              <ChapterFreshness
                enrichedAt={intelligence?.enrichedAt}
                fragments={[
                  { text: "Most fields not captured — known gap", stale: true },
                ]}
              />
            }
          />
          <RelationshipFabric detail={detail} accountName={detail.name ?? undefined} />
        </MarginSection>

        {/* The record — timeline continuity (preserved to avoid regression). */}
        <MarginSection id="the-record" label={<>The<br/>Record</>}>
          <UnifiedTimeline
            data={{
              ...detail,
              accountEvents: acct.events,
              lifecycleChanges: detail.lifecycleChanges,
              autoCompletedMilestones: detail.autoCompletedMilestones,
              contextEntries: page.entityCtx.entries,
            }}
            sectionId=""
            actionSlot={<AddToRecord onAdd={(title, content) => page.entityCtx.createEntry(title, content)} />}
          />
        </MarginSection>

        {acct.files.length > 0 && (
          <MarginSection id="files" label="Files" reveal={false}>
            <FileListSection files={acct.files} />
          </MarginSection>
        )}

        {/* Chapter 9: About this dossier — always renders; our own data-quality story */}
        <MarginSection id="about-dossier" label={<>About the<br/>dossier</>} reveal={false}>
          <AboutThisDossier
            intelligence={intelligence}
            meetingCount={meetingCount}
            transcriptCount={transcriptCount}
            uncharacterizedStakeholders={(detail.stakeholdersFull ?? [])
              .filter((s) => {
                const count = s.meetingCount ?? 0;
                const hasAssessment = Boolean(s.assessment && s.assessment.trim().length > 0);
                return count > 0 && !hasAssessment;
              })
              .map((s) => ({ personName: s.personName, meetingCount: s.meetingCount ?? null }))}
          />
        </MarginSection>

        <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
      </>
    );
  };

  // ─── View 3: The Work ───────────────────────────────────────────────────
  // DOS-13: 8-chapter workbench IA matches account-work-globex.html mockup.
  // Zero-guilt patterns throughout: "Still active?" replaces OVERDUE, Private/Shared
  // pills are orthogonal to draft/done, Dismiss is equal-valid with Mark done,
  // Suggestions carry "Dismiss (teaches system)", Nudges always offer "Leave as-is"
  // and the chapter hides entirely when the list is empty.
  const renderWorkView = () => {
    const recommendedActions = intelligence?.recommendedActions ?? [];
    const openCommitments = intelligence?.openCommitments ?? [];
    const wins = intelligence?.recentWins ?? [];
    const programs = acct.programs ?? [];
    const accountEvents = acct.events ?? [];

    // ── Chapter 1: 90-day focus ─────────────────────────────────────────
    // Editorial synthesis of the 3–4 most important threads over the next 90
    // days. Drafted from commitments + recommendations + programs (no new
    // schema — read-only projection). Falls back to a graceful empty when the
    // underlying sources are thin.
    const focusItems: FocusItem[] = [];

    openCommitments.slice(0, 2).forEach((c) => {
      const citations: FocusItem["citations"] = [];
      if (c.source) citations.push({ label: c.source });
      citations.push({ label: "Commitment" });
      focusItems.push({
        headline: c.description,
        paragraph: [
          c.owner ? `Owner: ${c.owner}.` : null,
          c.dueDate ? `Due ${formatShortDate(c.dueDate)}.` : "No date set — carry forward until it closes.",
        ].filter(Boolean).join(" "),
        citations,
      });
    });

    recommendedActions.slice(0, 4 - focusItems.length).forEach((r) => {
      focusItems.push({
        headline: r.title,
        paragraph: r.rationale,
        citations: [{ label: "Recommendation" }],
      });
    });

    // Programs pad remaining slots up to 4 items alongside commitments +
    // recommendations. Mockup interleaves programs with commitments rather
    // than holding them as a last-resort fallback.
    programs.slice(0, 4 - focusItems.length).forEach((p) => {
      if (!p.name) return;
      focusItems.push({
        headline: p.name,
        paragraph: p.notes || `Standing motion in ${p.status.toLowerCase()} state. Revisit when context shifts.`,
        citations: [{ label: `Program · ${p.status}` }],
      });
    });

    const focusSynthesisFragments: string[] = [];
    if (openCommitments.length) focusSynthesisFragments.push(`${openCommitments.length} commitment${openCommitments.length === 1 ? "" : "s"}`);
    if (recommendedActions.length) focusSynthesisFragments.push(`${recommendedActions.length} suggestion${recommendedActions.length === 1 ? "" : "s"}`);
    if (programs.length) focusSynthesisFragments.push(`${programs.length} program${programs.length === 1 ? "" : "s"}`);

    // ── Chapter 2: Programs & motions ────────────────────────────────────
    // Standing states only — no due dates, not todos.
    const activePrograms = programs.filter((p) => p.name);

    // ── Chapter 3: Commitments ───────────────────────────────────────────
    // v1.2.1 scope: real tracker provenance (structured trackerLink with
    // system/externalId/href) lands in DOS-75 (v1.2.2). Until then we render
    // every commitment as Private. Regex-sniffing the free-text `source`
    // string for "Linear"/"Salesforce"/etc. generated false Shared labels
    // with no authoritative link to back them up, so we stop guessing.
    //
    // Audience classification similarly waits for structured data — we
    // default to customer-facing unless the description or source string
    // explicitly says "internal".
    //
    // stillActiveNote is synthesized only when a due date has passed —
    // copy always opens with "Still active?" never "OVERDUE".
    const AGED_COMMITMENT_THRESHOLD_DAYS = 45;
    const commitmentCards = openCommitments.map((c, idx) => {
      const sourceLower = (c.source ?? "").toLowerCase();
      const isInternal = /internal|^program\b|team/i.test(c.description) || /internal/.test(sourceLower);

      // Structured tracker link (DOS-75) takes precedence over the free-text
      // source string: when present, the commitment renders as Shared with
      // the tracker anchor. Backend wiring lands with DOS-75; forward-compatible
      // today.
      const trackerLink = (c as { trackerLink?: { system?: string; href?: string; externalId?: string } }).trackerLink;
      const visibility: "shared" | "private" = trackerLink?.href ? "shared" : "private";

      const provenance: { label: string; href?: string }[] = [];
      if (trackerLink?.href) {
        provenance.push({
          label: trackerLink.system ? `${trackerLink.system}${trackerLink.externalId ? ` · ${trackerLink.externalId}` : ""}` : "Tracker",
          href: trackerLink.href,
        });
      } else if (c.source) {
        provenance.push({ label: c.source });
      }

      let stillActiveNote: string | undefined;
      if (c.dueDate) {
        const due = new Date(c.dueDate);
        const diff = Math.round((Date.now() - due.getTime()) / (1000 * 60 * 60 * 24));
        if (diff > 0) {
          stillActiveNote = `Due date passed ${diff} day${diff === 1 ? "" : "s"} ago. Worth a glance, not a panic.`;
        }
      } else {
        // Aged no-due-date commitments: synthesize a soft "Still active?" note
        // when the item has been carried for > threshold days. Uses authoritative
        // sourcedAt; skip when the timestamp is missing (rather than guess).
        const sourcedAt = c.itemSource?.sourcedAt;
        const ts = sourcedAt ? new Date(sourcedAt).getTime() : Number.NaN;
        if (Number.isFinite(ts)) {
          const ageDays = Math.floor((Date.now() - ts) / (1000 * 60 * 60 * 24));
          if (ageDays >= AGED_COMMITMENT_THRESHOLD_DAYS) {
            stillActiveNote = `Carried for ${ageDays} days without a date — still active?`;
          }
        }
      }

      const doneBusy = acct.commitmentDoneInFlight.has(idx);
      const dismissBusy = acct.commitmentDismissInFlight.has(idx);
      return (
        <CommitmentCard
          key={idx}
          headline={c.description}
          provenance={provenance.length > 0 ? provenance : undefined}
          owner={c.owner ?? null}
          due={c.dueDate ? formatShortDate(c.dueDate) : null}
          audience={isInternal ? "internal" : "customer"}
          visibility={visibility}
          stillActiveNote={stillActiveNote}
          actions={
            <>
              <WorkButton
                kind="primary"
                disabled={doneBusy || dismissBusy}
                onClick={() => acct.handleMarkCommitmentDone(idx)}
              >
                {doneBusy ? "Marking done…" : "Mark done"}
              </WorkButton>
              <WorkButton
                kind="muted"
                disabled={doneBusy || dismissBusy}
                onClick={() => acct.handleDismissCommitment(idx, c.description)}
              >
                {dismissBusy ? "Dismissing…" : "Dismiss"}
              </WorkButton>
            </>
          }
        />
      );
    });

    const commitmentFragments: string[] = [];
    if (openCommitments.length) {
      commitmentFragments.push(`${openCommitments.length} open · all private in v1.2.1`);
    }

    // ── Chapter 6: Recently landed ───────────────────────────────────────
    // 30-day tail from wins + lifecycle events.
    //
    // Contract: only items with a real timestamp inside the 30-day window
    // render here. Undated wins are excluded outright — rendering them as
    // "Delivered" under a 30-day tail label is a false claim. If a win is
    // missing sourcedAt it drops off this surface; it can still be picked
    // up by Context's value-delivered list.
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;
    // Wins get a cross-reference to the Context "What we've built" chapter
    // so the user can trace a landed item back to the value-commitments
    // record. Non-win events have no xref.
    const contextValueHref = page.accountId
      ? `/accounts/${page.accountId}?view=context#value-commitments`
      : null;
    type RecentRow = {
      key: string;
      date: string;
      event: string;
      source: React.ReactNode;
      ts: number;
    };
    const recentWinsRows: RecentRow[] = wins
      .map((w, i) => {
        const sourcedAt = w.itemSource?.sourcedAt;
        const ts = sourcedAt ? new Date(sourcedAt).getTime() : Number.NaN;
        const origin = w.source ? `Came from ${w.source}` : null;
        const xref = contextValueHref ? (
          <>
            {origin && <>{origin} · </>}
            <a href={contextValueHref}>See Context value →</a>
          </>
        ) : origin;
        return {
          key: `win-${i}`,
          date: sourcedAt ? formatShortDate(sourcedAt).toUpperCase() : "",
          event: w.text,
          source: xref as React.ReactNode,
          ts,
        };
      })
      .filter((row) => Number.isFinite(row.ts) && row.ts >= thirtyDaysAgo);
    const recentEventRows: RecentRow[] = accountEvents
      .filter((e) => {
        const t = new Date(e.eventDate).getTime();
        return !Number.isNaN(t) && t >= thirtyDaysAgo;
      })
      .map((e) => ({
        key: `evt-${e.id}`,
        date: formatShortDate(e.eventDate).toUpperCase(),
        event: e.notes || `${e.eventType.replace(/_/g, " ")} recorded`,
        source: null,
        ts: new Date(e.eventDate).getTime(),
      }));
    const recentlyLanded = [...recentWinsRows, ...recentEventRows]
      .sort((a, b) => b.ts - a.ts)
      .slice(0, 8);

    // ── Chapter 7: Outputs ───────────────────────────────────────────────
    // Generated reports. Links out to the Report Engine. Full-plan export is
    // deferred to the Report Engine project — keep this a jumping-off point.
    const reports = getAccountReports(preset?.id);
    const navigateToReport = (reportType: string) => {
      if (reportType === "risk_briefing" || reportType === "account_health" || reportType === "ebr_qbr") {
        page.navigate({
          to: `/accounts/$accountId/reports/${reportType}` as "/accounts/$accountId/reports/account_health",
          params: { accountId: page.accountId },
        });
      } else {
        page.navigate({
          to: "/accounts/$accountId/reports/$reportType",
          params: { accountId: page.accountId, reportType },
        });
      }
    };

    // ── Chapter 8: Nudges ────────────────────────────────────────────────
    // Hidden when empty. Every nudge must offer "Leave as-is".
    type Nudge = { headline: string; body: string; actions: React.ReactNode };
    const nudges: Nudge[] = [];

    // Private-for-too-long nudge: private commitment with no due date that
    // has sat untouched for at least PRIVATE_NUDGE_THRESHOLD_DAYS. Age is
    // computed from authoritative itemSource.sourcedAt — a brand-new private
    // commitment is NOT nagged. If sourcedAt is missing we skip the nudge
    // entirely rather than guess.
    const PRIVATE_NUDGE_THRESHOLD_DAYS = 45;
    const MS_PER_DAY = 24 * 60 * 60 * 1000;
    const nowMs = Date.now();
    const agedPrivate = openCommitments
      .map((c, originalIndex) => {
        if (c.dueDate) return null;
        const sourcedAt = c.itemSource?.sourcedAt;
        const ts = sourcedAt ? new Date(sourcedAt).getTime() : Number.NaN;
        const ageDays = Number.isFinite(ts)
          ? Math.floor((nowMs - ts) / MS_PER_DAY)
          : Number.NaN;
        return { c, ageDays, originalIndex };
      })
      .filter((x): x is { c: typeof openCommitments[number]; ageDays: number; originalIndex: number } =>
        x !== null && Number.isFinite(x.ageDays) && x.ageDays >= PRIVATE_NUDGE_THRESHOLD_DAYS,
      )
      .sort((a, b) => b.ageDays - a.ageDays)[0];
    if (agedPrivate) {
      const { c: oldestPrivate, ageDays, originalIndex } = agedPrivate;
      const dismissBusy = acct.commitmentDismissInFlight.has(originalIndex);
      nudges.push({
        headline: "A commitment has been kept private",
        body: `"${oldestPrivate.description}"${oldestPrivate.owner ? ` (owner: ${oldestPrivate.owner})` : ""} has been kept private for ${ageDays} day${ageDays === 1 ? "" : "s"} with no due date — leave as-is, or dismiss if it's no longer live.`,
        actions: (
          <>
            <WorkButton
              disabled={dismissBusy}
              onClick={() => acct.handleDismissCommitment(originalIndex, oldestPrivate.description)}
            >
              {dismissBusy ? "Dismissing…" : "Dismiss"}
            </WorkButton>
            {/*
              Option B (Wave 0g Finding 1): "Leave as-is" is the zero-guilt
              default, not a separate action. Rendering it as italic prose
              rather than a no-op button keeps the editorial voice honest —
              doing nothing IS leaving it as-is. A real snooze mechanism
              would need a per-nudge-type backend surface (nudge_snoozes
              table + service + IL wiring); disproportionate for one button.
            */}
            <NudgeLeaveAsIs />
          </>
        ),
      });
    }

    // Stale-shared-writeback nudge removed in v1.2.1: it was synthesized from
    // regex-inferred Shared state with no authoritative tracker link. Real
    // writeback-freshness nudges return with DOS-75's trackerLink schema.

    const hasFocus = focusItems.length > 0;
    const hasPrograms = activePrograms.length > 0;
    const hasCommitments = openCommitments.length > 0;
    const hasSuggestions = recommendedActions.length > 0;
    const hasRecentlyLanded = recentlyLanded.length > 0;
    const hasReports = reports.length > 0;
    const hasNudges = nudges.length > 0;
    // Shared chapter is gated on structured tracker provenance (DOS-75, v1.2.2).
    // Forward-compatible — checks for a trackerLink field on any commitment.
    // Evaluates to false in v1.2.1 since the backend doesn't emit the field yet.
    const hasSharedData = openCommitments.some(
      (c) => !!(c as { trackerLink?: { href?: string } }).trackerLink?.href,
    );

    return (
      <>
        {/* Chapter 1: 90-day focus — editorial numbered list */}
        {hasFocus && (
          <MarginSection id="focus" label={<>90-day<br/>focus</>}>
            <ChapterHeading
              title="90-day focus"
              epigraph="Our active plan · editable"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={
                    focusSynthesisFragments.length
                      ? [`Drafted from ${focusSynthesisFragments.join(" + ")}`]
                      : ["Our active plan · editable"]
                  }
                />
              }
            />
            <NumberedFocusList items={focusItems} />
          </MarginSection>
        )}

        {/* Chapter 2: Programs & motions — standing states, not to-dos */}
        {hasPrograms && (
          <MarginSection id="programs" label={<>Programs<br/>&amp; motions</>}>
            <ChapterHeading
              title="Programs & motions"
              epigraph="Standing motions · not a todo list"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[`${activePrograms.length} motion${activePrograms.length === 1 ? "" : "s"} active`]}
                />
              }
            />
            <ProgramPillRow>
              {activePrograms.map((p, i) => (
                <ProgramPill
                  key={i}
                  state={p.status ? `In ${p.status.toLowerCase()}` : p.name}
                  description={p.notes || p.name}
                />
              ))}
            </ProgramPillRow>
          </MarginSection>
        )}

        {/* Chapter 3: Commitments — what we've said we'll do */}
        {hasCommitments && (
          <MarginSection id="commitments" label={<>Commit-<br/>ments</>}>
            <ChapterHeading
              title="Commitments"
              epigraph="What we've said we'll do"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[...commitmentFragments, "Natural sort by recency"]}
                />
              }
            />
            <div style={{ display: "flex", flexDirection: "column", gap: 32 }}>
              {commitmentCards}
            </div>
          </MarginSection>
        )}

        {/* Chapter 4: Suggestions — AI proposals, saffron background */}
        {hasSuggestions && (
          <MarginSection id="suggestions" label={<>Sugges-<br/>tions</>}>
            <ChapterHeading
              title="Suggestions"
              epigraph="AI proposals · accept or dismiss"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[
                    `${recommendedActions.length} suggestion${recommendedActions.length === 1 ? "" : "s"}`,
                    "Accept or dismiss — dismissals teach the system",
                  ]}
                />
              }
            />
            <div style={{ display: "flex", flexDirection: "column", gap: 32 }}>
              {recommendedActions.map((r, i) => (
                <SuggestionCard
                  key={i}
                  headline={r.title}
                  rationale={r.rationale}
                  provenance={[{ label: "Account intelligence" }]}
                  accepting={acct.suggestionAcceptInFlight.has(i)}
                  dismissing={acct.suggestionDismissInFlight.has(i)}
                  onAccept={() => acct.handleAcceptSuggestion(i)}
                  onDismiss={() => acct.handleDismissSuggestion(i)}
                />
              ))}
            </div>
          </MarginSection>
        )}

        {/*
          Chapter 5: Shared with the team — honest degradation (Wave 0g
          Finding 2). The chapter (and its nav-island pill) are suppressed
          entirely until real tracker provenance exists. A commitment is
          "shared" only when it carries a structured trackerLink payload
          (system + externalId + href), which lands in v1.2.2 / DOS-75.
          Rendering an always-empty chapter put a dead pill in the IA.
        */}
        {hasSharedData && (
          <MarginSection id="shared" label={<>Shared<br/>with<br/>team</>}>
            <ChapterHeading
              title="Shared with the team"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={["Tracker writeback · live status"]}
                />
              }
            />
            <EditorialEmpty
              title="Nothing is shared to a tracker yet."
              message="Commitments with a real external link appear here once a tracker is wired."
            />
          </MarginSection>
        )}

        {/* Chapter 6: Recently landed — 30-day completion tail */}
        {hasRecentlyLanded && (
          <MarginSection id="recently-landed" label={<>Recently<br/>landed</>}>
            <ChapterHeading
              title="Recently landed"
              epigraph="30-day completion tail"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[
                    `${recentlyLanded.length} item${recentlyLanded.length === 1 ? "" : "s"} delivered`,
                    "30-day tail · promotes to Context \"value delivered\"",
                  ]}
                />
              }
            />
            <RecentlyLandedList>
              {recentlyLanded.map((row) => (
                <RecentlyLandedRow
                  key={row.key}
                  date={row.date}
                  event={row.event}
                  source={row.source}
                />
              ))}
            </RecentlyLandedList>
          </MarginSection>
        )}

        {/* Chapter 7: Outputs — generated reports, link out to Report Engine */}
        {hasReports && (
          <MarginSection id="outputs" label={<>Out-<br/>puts</>}>
            <ChapterHeading
              title="Outputs"
              epigraph="Generated reports · open to regenerate"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[`${reports.length} report${reports.length === 1 ? "" : "s"} available for this account`]}
                />
              }
            />
            <ReportGrid>
              {reports.map((r) => (
                <ReportCard
                  key={r.reportType}
                  type={r.label}
                  title={`${detail.name ?? "Account"} — ${r.label}`}
                  generatedAt={intelligence?.enrichedAt ? formatShortDate(intelligence.enrichedAt) : undefined}
                  trigger="on-demand"
                  onOpen={() => navigateToReport(r.reportType)}
                  onRefresh={() => navigateToReport(r.reportType)}
                />
              ))}
            </ReportGrid>
            <ReportFooterNote>
              Full-plan synthesis and export lives in the Report Engine. Open any report above to generate a fresh copy from current intelligence.
            </ReportFooterNote>
          </MarginSection>
        )}

        {/* Chapter 8: Nudges — soft meta, hidden when empty */}
        {hasNudges && (
          <MarginSection id="nudges" label={<>Nudges</>}>
            <ChapterHeading
              title="Nudges"
              epigraph="Soft meta · leave as-is is fine"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[
                    `${nudges.length} soft surface${nudges.length === 1 ? "" : "s"}`,
                    "Zero-guilt · leave anything as-is",
                  ]}
                />
              }
            />
            <NudgeList>
              {nudges.map((n, i) => (
                <NudgeRow key={i} headline={n.headline} body={n.body} actions={n.actions} />
              ))}
            </NudgeList>
          </MarginSection>
        )}

        <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
      </>
    );
  };

  return (
    <>
      <AccountBreadcrumbs ancestors={page.ancestors} currentName={detail.name ?? ""} />

      <section id="headline" className={shared.chapterHeadline}>
        <AccountHero detail={detail} intelligence={intelligence}
          editName={acct.editName} setEditName={(v) => { acct.setEditName(v); acct.setDirty(true); }}
          editHealth={acct.editHealth} setEditHealth={(v) => { acct.setEditHealth(v); acct.setDirty(true); }}
          editLifecycle={acct.editLifecycle} setEditLifecycle={(v) => { acct.setEditLifecycle(v); acct.setDirty(true); }}
          onSave={acct.handleSave} onSaveField={page.saveAccountField}
          vitalsSlot={detail.accountType !== "internal" ? (preset
            ? <EditableVitalsStrip fields={preset.vitals.account} entityData={detail} metadata={page.metadataValues}
                onFieldChange={(key, col, source, value) => {
                  if (source === "metadata") page.handleMetadataChange(key, value);
                  else if (source === "column") void page.saveAccountField(col ?? key, value);
                }} conflicts={page.conflictsForStrip} sourceRefs={detail.sourceRefs} />
            : <VitalsStrip vitals={buildAccountVitals(detail)} sourceRefs={detail.sourceRefs} />
          ) : undefined}
          provenanceSlot={undefined} />
        {preset && preset.metadata.account.length > 0 && (
          <div className={`editorial-reveal ${shared.presetFieldsReveal}`}>
            <PresetFieldsEditor fields={preset.metadata.account} values={page.metadataValues} onChange={page.handleMetadataChange} />
          </div>
        )}
        {detail.renewalDate && !page.rolloverDismissed && (
          <AccountRolloverPrompt renewalDate={detail.renewalDate}
            onRenewed={() => { acct.setNewEventType("renewal"); acct.setNewEventDate(detail.renewalDate!); acct.handleRecordEvent(); page.setRolloverDismissed(true); }}
            onChurned={() => { acct.setNewEventType("churn"); acct.setNewEventDate(detail.renewalDate!); acct.handleRecordEvent(); page.setRolloverDismissed(true); }}
            onDismiss={() => page.setRolloverDismissed(true)} />
        )}
      </section>

      {/* All 3 views rendered, inactive hidden with display:none */}
      <div className={pageStyles.view} style={{ display: activeView === "health" ? "block" : "none" }}>
        {renderHealthView()}
      </div>
      <div className={pageStyles.view} style={{ display: activeView === "context" ? "block" : "none" }}>
        {renderContextView()}
      </div>
      <div className={pageStyles.view} style={{ display: activeView === "work" ? "block" : "none" }}>
        {renderWorkView()}
      </div>

      <AccountViewSwitcher activeView={page.activeView} onViewChange={page.setActiveView} />

      <AccountDialogs accountId={page.accountId} accountName={detail.name} accountType={detail.accountType}
        archiveDialogOpen={page.archiveDialogOpen} onArchiveDialogChange={page.setArchiveDialogOpen} onArchive={acct.handleArchive}
        createChildOpen={acct.createChildOpen} onCreateChildOpenChange={acct.setCreateChildOpen}
        childName={acct.childName} onChildNameChange={acct.setChildName}
        childDescription={acct.childDescription} onChildDescriptionChange={acct.setChildDescription}
        creatingChild={acct.creatingChild} onCreateChild={acct.handleCreateChild}
        mergeDialogOpen={page.mergeDialogOpen} onMergeDialogChange={page.setMergeDialogOpen}
        onMerged={() => page.navigate({ to: "/accounts" })} />
    </>
  );
}
