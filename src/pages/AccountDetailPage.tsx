/**
 * AccountDetailPage — Clean rebuild of the account detail page.
 *
 * Single flat route, state-based view switching, no child routes.
 * Built step by step per plan at ~/.claude/plans/deep-wiggling-hearth.md.
 *
 * Step 5: All 3 views rendered, inactive hidden via display:none.
 * Preserves scroll + form state + pending fetches on tab switch.
 */
import { useEffect, useState } from "react";
import { useParams } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useAccountDetailPage } from "@/hooks/useAccountDetailPage";
import { useEntitySuppressions } from "@/hooks/useEntitySuppressions";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { ChapterFreshness } from "@/components/editorial/ChapterFreshness";
import { QuoteWallPlaceholder } from "@/components/editorial/QuoteWallPlaceholder";
import { AboutThisDossier } from "@/components/context/AboutThisDossier";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
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
import { AccountPortfolioSection } from "@/components/account/AccountPortfolioSection";
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
import { PendingStakeholderQueue } from "@/components/entity/PendingStakeholderQueue";
import { usePendingStakeholders } from "@/hooks/usePendingStakeholders";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
import { CommercialShape } from "@/components/context/CommercialShape";
import { RelationshipFabric } from "@/components/context/RelationshipFabric";
// View 3 — The Work (DOS-13: workbench, not todo list)
import {
  ProgramPill,
  ProgramPillRow,
  CommitmentCard,
  SuggestionCard,
  RecentlyLandedList,
  RecentlyLandedRow,
  ReportCard,
  ReportGrid,
  ReportFooterNote,
  WorkButton,
} from "@/components/work/WorkSurface";
import { getAccountReports } from "@/lib/report-config";
import { buildAccountVitals } from "@/components/account/account-detail-utils";
import { formatShortDate } from "@/lib/utils";

import shared from "@/styles/entity-detail.module.css";
import pageStyles from "./AccountDetailPage.module.css";

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const page = useAccountDetailPage(accountId);
  const suppressions = useEntitySuppressions(page.detail?.id ?? accountId);

  // v1.2.1 QA fix: gate the "Push to Linear" button on actual Linear
  // configuration so users don't land on a dead picker.
  const [linearConfigured, setLinearConfigured] = useState(false);
  useEffect(() => {
    invoke<{ enabled: boolean; apiKeySet: boolean }>("get_linear_status")
      .then((s) => setLinearConfigured(s.enabled && s.apiKeySet))
      .catch(() => setLinearConfigured(false));
  }, []);

  // DOS-258 Lane F: pending stakeholder review queue for the Context tab.
  const pendingStakeholders = usePendingStakeholders(accountId);


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
    const showTriage = hasTriageContent(intelligence, glean, page.sentiment.current);
    const showDivergence = hasDivergenceContent(findings, glean);
    const isFineState = !!intelligence && !showTriage && !showDivergence;

    return (
      <>
        {/* DOS-228 Wave 0e Fix 4: pinned risk-briefing status at top of
            Health tab. Renders nothing when there's no active job; shows
            running/failed states with a retry affordance on failure. */}
        <RiskBriefingStatus
          job={acct.riskBriefingJob}
          accountId={detail.id}
          onRetry={acct.retryRiskBriefing}
        />
        {/* Chapter 1: Sentiment hero — "Your Assessment" in the mockup.
            Wrapped in a section so the chapter-nav anchor (id matches
            buildHealthChapters → "your-assessment") resolves cleanly. */}
        <section id="your-assessment">
          <SentimentHero
            view={page.sentiment}
            onSetSentiment={acct.setUserHealthSentiment}
            onAcknowledgeStale={acct.acknowledgeSentimentStale}
            onUpdateNote={acct.updateSentimentNote}
          />
        </section>

        {/* Chapters 2-3 (full state) OR On Track chapter (fine state) */}
        {isFineState ? (
          <MarginSection id="on-track" label={<>On<br/>Track</>}>
            <OnTrackChapter intelligence={intelligence} accountSizeLabel={detail.lifecycle ?? detail.accountType ?? null} />
            {/* DOS-41: "Is this accurate?" after the AI fine-state summary. */}
            <IntelligenceCorrection
              entityId={detail.id}
              entityType="account"
              field="on_track_assessment"
              variant="correct"
              currentValue={intelligence?.executiveAssessment ?? null}
              onCorrected={acct.silentRefresh}
            />
          </MarginSection>
        ) : (
          <MarginSection id="needs-attention" label={<>Needs<br/>attention</>}>
            {showTriage && (
              <TriageSection
                intelligence={intelligence}
                gleanSignals={glean}
                sentiment={page.sentiment.current}
                accountId={detail.id}
              />
            )}
            {showDivergence && (
              <DivergenceSection
                findings={findings}
                gleanSignals={glean}
                accountId={detail.id}
              />
            )}
          </MarginSection>
        )}

        {/* Chapter 4: Outlook — the chapter title IS the verdict
            ("The Call: Renewal" / "Churn risk" / "Expansion"), computed
            from agreementOutlook.confidence + expansionPotential. The gutter
            "Outlook" stays as the orientation marker. */}
        {intelligence && (intelligence.agreementOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
          <MarginSection id="outlook" label="Outlook">
            <ChapterHeading title={`The Call: ${renewalCallVerdict(intelligence.agreementOutlook)}`} />
            <OutlookPanel intelligence={intelligence} />
            {/* DOS-41: "Is this accurate?" after the AI renewal assessment. */}
            <IntelligenceCorrection
              entityId={detail.id}
              entityType="account"
              field="renewal_outlook"
              variant="correct"
              currentValue={intelligence.renewalOutlook?.expansionPotential ?? null}
              onCorrected={acct.silentRefresh}
            />
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
            onRemoveStakeholder={async (personId, personName) => {
              // Full delete of the person entity — unlinks from every
              // account AND removes from the global people list. Use
              // case: synthetic / bot email addresses that shouldn't
              // exist anywhere. Permanent; guarded with a native
              // confirm so an accidental click can't nuke a real
              // stakeholder.
              const ok = window.confirm(
                `Delete ${personName || "this person"} permanently?\n\n` +
                  `This removes them from this account and from the people list across every account. Use for bot addresses or duplicate entries — not for real stakeholders you just want to hide.`,
              );
              if (!ok) return;
              try {
                await invoke("delete_person", { personId });
                // Silent refresh so the card disappears without a full
                // loading state / scroll jump.
                acct.silentRefresh();
              } catch (e) {
                toast.error(`Failed to delete: ${e}`);
              }
            }}
          />
          {/* DOS-258 Lane F: pending_review rows from account_stakeholders.
              Rendered immediately after the confirmed-stakeholder grid so the
              user sees "what we know" then "what needs review" in one scan.
              Hidden when the queue is empty — no placeholder clutter. */}
          <PendingStakeholderQueue queue={pendingStakeholders} />
        </MarginSection>

        {/* Chapter 3: What matters to them */}
        {intelligence && hasWhatMatters && (
          <MarginSection id="what-matters" label={<>What<br/>matters</>}>
            <ChapterHeading
              title="What matters to them"
              freshness={<ChapterFreshness enrichedAt={intelligence.enrichedAt} fragments={whatMattersFragments} />}
            />
            <StrategicLandscape
              intelligence={intelligence}
              onUpdateField={page.handleUpdateIntelField}
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
            products={detail.products ?? []}
          />
        </MarginSection>

        {/* Products folded into Technical shape as a dotted list (DOS-251
            tracks full edit UX + Services subsection for v1.2.2). */}

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

        {/* The Record + Files moved to the Work tab where they belong —
            they're operational/workbench surfaces, not narrative context. */}

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
    const programs = acct.programs ?? [];
    const work = acct.work;
    const visibleSuggestions = work.suggestions.filter(
      (r) => !suppressions.isSuppressed(`work_suggestion:${r.id}`, r.title),
    );

    // ── Programs & motions ──────────────────────────────────────────────
    // Standing states only — no due dates, not todos.
    const activePrograms = programs.filter((p) => p.name);

    // ── Chapter 1: Commitments ───────────────────────────────────────────
    // DOS Work-tab Phase 3: sourced from the `actions` table via
    // `useAccountWorkData` (action_kind='commitment', status in backlog /
    // unstarted / started). Dispatch by stable action.id — no index-based
    // handlers.
    //
    // Top-4 visual weight: the first four cards carry `.emphasis` (heavier
    // serif headline). Items 5+ render at the default weight. This gives a
    // "big three or four" reading order without turning the rest of the
    // list into second-class citizens.
    //
    // Soft "Still active?" rules:
    //   status='started' + due_date past       → "Due date passed N days ago."
    //   status='backlog' + age > 45d            → "Carried for N days — still active?"
    //   status='unstarted' + no due + age > 45d → "Carried for N days without a date — still active?"
    const AGED_COMMITMENT_THRESHOLD_DAYS = 45;
    const commitmentCards = work.commitments.map((c, idx) => {
      const contextLower = (c.context ?? "").toLowerCase();
      const isInternal = /internal|^program\b|\bteam\b/i.test(c.title) || /internal/.test(contextLower);

      // Linear link on the action row = shared with the team. Only the
      // linear_identifier + linear_url flavour is wired today; Salesforce
      // / Slack writeback lands later.
      const linearHref = c.linearUrl;
      const visibility: "shared" | "private" = linearHref ? "shared" : "private";

      const provenance: { label: string; href?: string }[] = [];
      if (c.linearIdentifier && linearHref) {
        provenance.push({
          label: `Linear · ${c.linearIdentifier}`,
          href: linearHref,
        });
      } else if (c.sourceLabel) {
        provenance.push({ label: c.sourceLabel });
      } else if (c.sourceType) {
        provenance.push({ label: c.sourceType });
      }

      // Soft-nudge copy table. At most one rule fires — due-date-past wins
      // when present; aged fallback only runs for dateless commitments or
      // backlog items.
      let stillActiveNote: string | undefined;
      const now = Date.now();
      const createdAt = c.createdAt ? new Date(c.createdAt).getTime() : Number.NaN;
      const ageDays = Number.isFinite(createdAt)
        ? Math.floor((now - createdAt) / (1000 * 60 * 60 * 24))
        : null;

      if (c.status === "started" && c.dueDate) {
        const due = new Date(c.dueDate).getTime();
        if (!Number.isNaN(due) && due < now) {
          const diff = Math.round((now - due) / (1000 * 60 * 60 * 24));
          stillActiveNote = `Due date passed ${diff} day${diff === 1 ? "" : "s"} ago. Worth a glance, not a panic.`;
        }
      } else if (c.status === "backlog" && ageDays !== null && ageDays > AGED_COMMITMENT_THRESHOLD_DAYS) {
        stillActiveNote = `Carried for ${ageDays} days — still active?`;
      } else if (
        c.status === "unstarted" &&
        !c.dueDate &&
        ageDays !== null &&
        ageDays > AGED_COMMITMENT_THRESHOLD_DAYS
      ) {
        stillActiveNote = `Carried for ${ageDays} days without a date — still active?`;
      }

      const doneBusy = work.commitmentDoneInFlight.has(c.id);
      const dismissBusy = work.commitmentDismissInFlight.has(c.id);
      const isEmphasized = idx < 4;
      // Owner is stashed in action.context as "owner: <name>" by the
      // commitment_bridge service until DbAction grows its own owner column.
      const ownerMatch = /^owner:\s*(.+)$/i.exec(c.context ?? "");
      const ownerValue = ownerMatch?.[1]?.trim() ?? null;
      return (
        <CommitmentCard
          key={c.id}
          emphasis={isEmphasized}
          headline={c.title}
          provenance={provenance.length > 0 ? provenance : undefined}
          owner={ownerValue}
          due={c.dueDate ? formatShortDate(c.dueDate) : null}
          dueDateRaw={c.dueDate ?? null}
          audience={isInternal ? "internal" : "customer"}
          visibility={visibility}
          sharedRef={linearHref && c.linearIdentifier ? { label: c.linearIdentifier, href: linearHref } : undefined}
          stillActiveNote={stillActiveNote}
          onEditHeadline={(title) => work.handleUpdateCommitment(c.id, { title })}
          onEditOwner={(owner) =>
            work.handleUpdateCommitment(c.id, {
              context: owner.trim().length > 0 ? `owner: ${owner.trim()}` : "",
            })
          }
          onEditDueDate={(dueDate) => work.handleUpdateCommitment(c.id, { dueDate })}
          actions={
            <>
              <WorkButton
                kind="primary"
                disabled={doneBusy || dismissBusy}
                onClick={() => work.handleMarkCommitmentDone(c.id)}
              >
                {doneBusy ? "Marking done…" : "Mark done"}
              </WorkButton>
              <WorkButton
                kind="muted"
                disabled={doneBusy || dismissBusy}
                onClick={() => work.handleDismissCommitment(c.id)}
              >
                {dismissBusy ? "Dismissing…" : "Dismiss"}
              </WorkButton>
              {!linearHref && linearConfigured && (
                <WorkButton kind="muted" onClick={() => work.handlePushToLinear(c.id)}>
                  Push to Linear
                </WorkButton>
              )}
            </>
          }
        />
      );
    });

    const commitmentFragments: string[] = [];
    if (work.commitments.length) {
      commitmentFragments.push(`${work.commitments.length} open · sourced from actions`);
    }

    // ── Chapter 6: Recently landed ───────────────────────────────────────
    // DOS Work-tab Phase 3: sourced from actions with status='completed'
    // AND completed_at >= now - 30d (cap 20). Cross-reference to Context
    // "Value delivered" anchors so a landed item can be traced back to the
    // value-commitments record.
    const contextValueHref = page.accountId
      ? `/accounts/${page.accountId}?view=context#value-commitments`
      : null;
    const recentlyLanded = work.recentlyLanded.map((a) => {
      const completedAt = a.completedAt;
      const origin = a.sourceLabel
        ? `Came from ${a.sourceLabel}`
        : a.sourceType
          ? `Came from ${a.sourceType}`
          : null;
      const xref = contextValueHref ? (
        <>
          {origin && <>{origin} · </>}
          <a href={contextValueHref}>See Context value →</a>
        </>
      ) : origin;
      return {
        id: a.id,
        date: completedAt ? formatShortDate(completedAt).toUpperCase() : "",
        event: a.title,
        source: xref as React.ReactNode,
      };
    });

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

    // Nudges chapter removed — aged-private soft-nudge renders inline on
    // Commitments cards via stillActiveNote. Integration-status nudges
    // (writeback stalled, etc.) surface at the folio level, not as a chapter.

    const hasPrograms = activePrograms.length > 0;
    const hasCommitments = work.commitments.length > 0;
    const hasSuggestions = visibleSuggestions.length > 0;
    const hasRecentlyLanded = recentlyLanded.length > 0;
    const hasReports = reports.length > 0;
    // Shared chapter: any open commitment with a Linear link present on the
    // action row surfaces the "Shared with the team" chapter. Salesforce /
    // Slack writeback sources will extend this check when wired.
    const hasSharedData = work.commitments.some((c) => !!c.linearUrl);

    return (
      <>
        {/* 90-day Focus chapter dropped — its editorial roll-up duplicated
            Commitments + Suggestions + Programs. Commitments is now the
            opener; top-N visual weighting lives on that chapter instead.
            Narrative focus synthesis may return in v1.2.2 if AI writes it
            well. */}

        {/* Chapter 1: Commitments — what we've said we'll do.
            Opener chapter: plain section + noRule ChapterHeading so the
            chapter flows from the hero rather than reading as a mid-page
            chapter break. Matches Context "Thesis" + Health "Your
            Assessment" first-chapter treatment. */}
        {hasCommitments && (
          <section id="commitments" className="editorial-reveal">
            <ChapterHeading
              title="Commitments"
              epigraph="What we've said we'll do"
              noRule
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
          </section>
        )}

        {/* Chapter 2: Suggestions — AI proposals, saffron background.
            DOS Work-tab Phase 3: backed by actions with status='backlog'.
            Accepting promotes to 'unstarted' (and surfaces in Commitments
            when action_kind='commitment'); "No" archives the suggestion
            and feeds back into the quality loop. */}
        {hasSuggestions && (
          <MarginSection id="suggestions" label={<>Sugges-<br/>tions</>}>
            <ChapterHeading
              title="Suggestions"
              epigraph="AI proposals · accept or validate"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[
                    `${visibleSuggestions.length} suggestion${visibleSuggestions.length === 1 ? "" : "s"}`,
                    "Use Yes / No to train quality; Accept promotes to commitments",
                  ]}
                />
              }
            />
            <div style={{ display: "flex", flexDirection: "column", gap: 32 }}>
              {visibleSuggestions.map((r) => {
                const provenance: { label: string; href?: string }[] = [];
                if (r.sourceLabel) provenance.push({ label: r.sourceLabel });
                else if (r.sourceType) provenance.push({ label: r.sourceType });
                else provenance.push({ label: "Account intelligence" });
                return (
                  <SuggestionCard
                    key={r.id}
                    headline={r.title}
                    rationale={r.context ?? ""}
                    provenance={provenance}
                    accepting={work.suggestionAcceptInFlight.has(r.id)}
                    onAccept={() => work.handleAcceptSuggestion(r.id)}
                    feedbackSlot={
                      <IntelligenceCorrection
                        entityId={detail.id}
                        entityType="account"
                        field={`work_suggestion:${r.id}`}
                        itemKey={r.title}
                        onDismissed={() => {
                          suppressions.markSuppressed(`work_suggestion:${r.id}`, r.title);
                          return work.handleDismissSuggestion(r.id);
                        }}
                      />
                    }
                  />
                );
              })}
            </div>
          </MarginSection>
        )}

        {/* Chapter 3: Programs & motions — standing states, not to-dos */}
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
                  key={row.id}
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

        {/* Nudges chapter dropped — redundant with soft "Still active?"
            rendered inline on Commitments cards. Integration-level cross-
            cutting nudges (writeback stalled, etc.) surface at the folio
            level, not as a chapter. */}

        {/* The Record — timeline continuity. Migrated from the Context tab.
            Design refresh tracked separately; here we preserve the live surface. */}
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
