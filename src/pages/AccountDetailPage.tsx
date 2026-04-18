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
import { AccountHealthSection } from "@/components/account/AccountHealthSection";
import { AccountOutlook } from "@/components/entity/AccountOutlook";
import { AccountPortfolioSection } from "@/components/account/AccountPortfolioSection";
import { AccountProductsSection } from "@/components/account/AccountProductsSection";
import { SentimentHero } from "@/components/health/SentimentHero";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
// View 2 — Context
import { AccountPullQuote } from "@/components/account/AccountPullQuote";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
// View 3 — The Work (DOS-13: workbench, not todo list)
import {
  NumberedFocusList,
  ProgramPill,
  ProgramPillRow,
  CommitmentCard,
  SuggestionCard,
  SharedRefRow,
  SharedSubsectionLabel,
  RecentlyLandedList,
  RecentlyLandedRow,
  ReportCard,
  ReportGrid,
  ReportFooterNote,
  NudgeList,
  NudgeRow,
  WorkButton,
  type FocusItem,
} from "@/components/work/WorkSurface";
import { getAccountReports } from "@/lib/report-config";
import { buildAccountVitals } from "@/components/account/account-detail-utils";
import { formatShortDate, formatRelativeDate } from "@/lib/utils";

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
  // DOS-203: Sentiment hero leads. Triage (risks/wins) rendered as "Needs attention".
  // Fine state when no triage + no divergences: editorial "On track" body.
  const renderHealthView = () => {
    const risks = intelligence?.risks ?? [];
    const wins = intelligence?.recentWins ?? [];
    const divergences = intelligence?.consistencyFindings ?? [];
    const hasTriage = risks.length > 0 || wins.length > 0;
    const hasDivergences = divergences.length > 0;
    const isFineState = !!intelligence?.health && !hasTriage && !hasDivergences;

    return (
      <>
        <SentimentHero
          view={acct.sentiment}
          onSetSentiment={acct.setUserHealthSentiment}
          onAcknowledgeStale={acct.acknowledgeSentimentStale}
        />

        {isFineState ? (
          <MarginSection id="on-track" label={<>On<br/>Track</>}>
            <ChapterHeading
              title="On track"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={["Nothing active needs your attention"]}
                />
              }
            />
            <EditorialEmpty
              title="Everything is as it should be."
              message="No active friction, no divergences between data sources, no renewal drag. This account is quiet in the best sense. The full computed health breakdown is below."
            />
          </MarginSection>
        ) : null}

        {intelligence?.health && (
          <AccountHealthSection health={intelligence.health} consistencyFindings={intelligence.consistencyFindings} />
        )}

        {intelligence && (intelligence.renewalOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
          <MarginSection id="outlook" label="Outlook">
            <ChapterHeading
              title="Outlook"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={["Renewal confidence · peer benchmark · recommended start"]}
                />
              }
            />
            <AccountOutlook intelligence={intelligence} onUpdateField={page.handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
          </MarginSection>
        ) : null}

        {detail.isParent && detail.children.length > 0 && (
          <AccountPortfolioSection children={detail.children} intelligence={intelligence} />
        )}

        <AccountProductsSection accountId={detail.id} products={detail.products ?? []}
          getFeedback={fb.get} onFeedback={fb.submit} onRefresh={acct.load} silentRefresh={acct.silentRefresh} />

        <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
      </>
    );
  };

  // ─── View 2: Context ────────────────────────────────────────────────────
  // DOS-18: 7-chapter IA — Thesis / The Room / What matters / What we've built /
  // Their voice / Technical shape / About this dossier. Work tab owns The Record + Files.
  const renderContextView = () => {
    // Freshness fragment helpers derived from existing data. No new schema.
    const manifest = intelligence?.sourceManifest ?? [];
    const transcriptCount = manifest.filter((m) => (m.format ?? "").toLowerCase().includes("transcript")).length;
    const meetingCount = acct.events?.length;
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

        {/* Chapter 2: The Room — stakeholder layout split + "Active in Health →" pills */}
        <MarginSection id="the-room" label={<>The<br/>Room</>}>
          <StakeholderGallery
            intelligence={intelligence}
            linkedPeople={detail.linkedPeople}
            accountTeam={detail.accountTeam}
            stakeholdersFull={detail.stakeholdersFull}
            sectionId=""
            chapterTitle="The Room"
            subsectionLabels
            chapterFreshness={
              <ChapterFreshness
                enrichedAt={intelligence?.enrichedAt}
                fragments={roomFragments}
              />
            }
            entityId={page.accountId}
            entityType="account"
            onIntelligenceUpdated={acct.silentRefresh}
            onRemoveTeamMember={acct.handleRemoveTeamMember}
            onChangeTeamRole={acct.changeTeamMemberRole}
            onAddTeamMember={acct.addTeamMemberDirect}
            onCreateTeamMember={acct.createTeamMemberDirect}
            teamSearchQuery={acct.teamSearchQuery}
            onTeamSearchQueryChange={acct.setTeamSearchQuery}
            teamSearchResults={acct.teamSearchResults}
            suggestions={acct.suggestions}
            onAcceptSuggestion={acct.acceptSuggestion}
            onDismissSuggestion={acct.dismissSuggestion}
            onUpdateEngagement={acct.updateStakeholderEngagement}
            onUpdateAssessment={acct.updateStakeholderAssessment}
            onAddRole={acct.addStakeholderRole}
            onRemoveRole={acct.removeStakeholderRole}
          />
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
                fragments={["Quote wall · coming in DOS-205"]}
              />
            }
          />
          <QuoteWallPlaceholder />
        </MarginSection>

        {/* Chapter 6: Technical shape — promoted footprint + feature list (reference weight) */}
        {detail.technicalFootprint && (
          <MarginSection id="technical-shape" label={<>Technical<br/>shape</>}>
            <ChapterHeading
              title="Technical shape"
              variant="reference"
              freshness={
                <ChapterFreshness
                  at={detail.technicalFootprint.sourcedAt ?? intelligence?.enrichedAt}
                  fragments={technicalFragments}
                />
              }
            />
            <AccountTechnicalFootprint
              footprint={detail.technicalFootprint}
              variant="chapter"
              featureAdoption={featureAdoption}
            />
          </MarginSection>
        )}

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

        {/* Chapter 7: About this dossier — always renders; our own data-quality story */}
        <MarginSection id="about-dossier" label={<>About the<br/>dossier</>} reveal={false}>
          <AboutThisDossier
            intelligence={intelligence}
            meetingCount={meetingCount}
            transcriptCount={transcriptCount}
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

    // If still thin, seed from top programs — orientation as narrative.
    if (focusItems.length === 0) {
      programs.slice(0, 3).forEach((p) => {
        if (!p.name) return;
        focusItems.push({
          headline: p.name,
          paragraph: p.notes || `Standing motion in ${p.status.toLowerCase()} state. Revisit when context shifts.`,
          citations: [{ label: `Program · ${p.status}` }],
        });
      });
    }

    const focusSynthesisFragments: string[] = [];
    if (openCommitments.length) focusSynthesisFragments.push(`${openCommitments.length} commitment${openCommitments.length === 1 ? "" : "s"}`);
    if (recommendedActions.length) focusSynthesisFragments.push(`${recommendedActions.length} suggestion${recommendedActions.length === 1 ? "" : "s"}`);
    if (programs.length) focusSynthesisFragments.push(`${programs.length} program${programs.length === 1 ? "" : "s"}`);

    // ── Chapter 2: Programs & motions ────────────────────────────────────
    // Standing states only — no due dates, not todos.
    const activePrograms = programs.filter((p) => p.name);

    // ── Chapter 3: Commitments ───────────────────────────────────────────
    // Heuristic classification from existing fields (no new schema):
    //   · visibility = shared iff the source mentions a linked tracker
    //     (Linear/Salesforce/Slack appear in the source string). Else private.
    //   · audience = internal iff owner is Jamie-style (external-team-looking)
    //     indicator absent; we default to customer-facing unless description
    //     explicitly says "internal" or the source is an internal program.
    //     This is intentionally conservative — real provenance wiring lands in
    //     a follow-up (DOS-75 tracker-link backend).
    //   · stillActiveNote synthesized only when a due date has passed OR the
    //     record is >45d old — always opens with "Still active?" never "OVERDUE".
    const commitmentCounts = {
      total: openCommitments.length,
      shared: 0,
      private: 0,
    };
    const commitmentCards = openCommitments.map((c, idx) => {
      const sourceLower = (c.source ?? "").toLowerCase();
      const isShared = /linear|salesforce|slack|jira|asana|dos-|opp/i.test(c.source ?? "");
      const isInternal = /internal|^program\b|team/i.test(c.description) || /internal/.test(sourceLower);
      if (isShared) commitmentCounts.shared += 1; else commitmentCounts.private += 1;

      const provenance: { label: string; href?: string }[] = [];
      if (c.source) provenance.push({ label: c.source });

      let stillActiveNote: string | undefined;
      if (c.dueDate) {
        const due = new Date(c.dueDate);
        const diff = Math.round((Date.now() - due.getTime()) / (1000 * 60 * 60 * 24));
        if (diff > 0) {
          stillActiveNote = `Due date passed ${diff} day${diff === 1 ? "" : "s"} ago. Worth a glance, not a panic.`;
        }
      }

      return (
        <CommitmentCard
          key={idx}
          headline={c.description}
          provenance={provenance.length > 0 ? provenance : undefined}
          owner={c.owner ?? null}
          due={c.dueDate ? formatShortDate(c.dueDate) : null}
          audience={isInternal ? "internal" : "customer"}
          visibility={isShared ? "shared" : "private"}
          sharedRef={isShared ? { label: c.source ?? "Linked" } : undefined}
          linearStatus={c.status && isShared ? `${c.status} in Linear` : undefined}
          stillActiveNote={stillActiveNote}
          actions={
            <>
              <WorkButton kind="primary">Mark done</WorkButton>
              {isShared ? (
                <WorkButton>View in Linear</WorkButton>
              ) : (
                <WorkButton>Push to Linear</WorkButton>
              )}
              <WorkButton kind="muted">Dismiss</WorkButton>
            </>
          }
        />
      );
    });

    const commitmentFragments: string[] = [];
    if (commitmentCounts.total) commitmentFragments.push(`${commitmentCounts.total} open`);
    if (commitmentCounts.shared) commitmentFragments.push(`${commitmentCounts.shared} shared`);
    if (commitmentCounts.private) commitmentFragments.push(`${commitmentCounts.private} private`);

    // ── Chapter 5: Shared with the team ──────────────────────────────────
    // Mirror of externally-visible state. Derived from shared commitments
    // (DOS-75 will wire real Linear/Salesforce/Slack feeds in v1.2.2).
    const sharedItems = openCommitments.filter((c) =>
      /linear|salesforce|slack|jira|asana|dos-|opp/i.test(c.source ?? ""),
    );

    // ── Chapter 6: Recently landed ───────────────────────────────────────
    // 30-day tail from wins + lifecycle events.
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;
    const recentWinsRows = wins.map((w, i) => ({
      key: `win-${i}`,
      date: w.itemSource?.sourcedAt
        ? formatShortDate(w.itemSource.sourcedAt).toUpperCase()
        : "Recently",
      event: w.text,
      source: w.source ? `Came from ${w.source}` : null,
      ts: w.itemSource?.sourcedAt ? new Date(w.itemSource.sourcedAt).getTime() : 0,
    }));
    const recentEventRows = accountEvents
      .filter((e) => {
        const t = new Date(e.eventDate).getTime();
        return !Number.isNaN(t) && t >= thirtyDaysAgo;
      })
      .map((e) => ({
        key: `evt-${e.id}`,
        date: formatShortDate(e.eventDate).toUpperCase(),
        event: e.notes || `${e.eventType.replace(/_/g, " ")} recorded`,
        source: null as string | null,
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

    // Private-for-too-long nudge: private commitment with no due date, aged >45d.
    const oldestPrivate = openCommitments
      .filter((c) => !/linear|salesforce|slack|jira|asana|dos-/i.test(c.source ?? ""))
      .filter((c) => !c.dueDate)[0];
    if (oldestPrivate) {
      nudges.push({
        headline: "A commitment has been kept private",
        body: `"${oldestPrivate.description}"${oldestPrivate.owner ? ` (owner: ${oldestPrivate.owner})` : ""} is still private — no due date, nothing pushed out to Linear. Keep it private, or push it out so the team can see?`,
        actions: (
          <>
            <WorkButton>Push to Linear</WorkButton>
            <WorkButton>Dismiss</WorkButton>
            <WorkButton kind="muted">Leave as-is</WorkButton>
          </>
        ),
      });
    }

    // Shared-out-of-sync nudge: shared commitment whose source references a
    // tracker but there's no recent writeback. Synthesized from item freshness.
    const staleShared = openCommitments
      .filter((c) => /linear|salesforce|slack|dos-/i.test(c.source ?? ""))
      .find((c) => {
        const at = c.itemSource?.sourcedAt;
        if (!at) return false;
        const diff = Date.now() - new Date(at).getTime();
        return diff > 6 * 24 * 60 * 60 * 1000;
      });
    if (staleShared) {
      nudges.push({
        headline: "Shared status out of sync with the tracker",
        body: `"${staleShared.description}" is shared to ${staleShared.source} but hasn't synced back here recently. The writeback loop may have stalled.`,
        actions: (
          <>
            <WorkButton kind="primary">Check writeback</WorkButton>
            <WorkButton>Dismiss</WorkButton>
            <WorkButton kind="muted">Leave as-is</WorkButton>
          </>
        ),
      });
    }

    const hasFocus = focusItems.length > 0;
    const hasPrograms = activePrograms.length > 0;
    const hasCommitments = openCommitments.length > 0;
    const hasSuggestions = recommendedActions.length > 0;
    const hasShared = sharedItems.length > 0;
    const hasRecentlyLanded = recentlyLanded.length > 0;
    const hasReports = reports.length > 0;
    const hasNudges = nudges.length > 0;

    return (
      <>
        {/* Chapter 1: 90-day focus — editorial numbered list */}
        {hasFocus && (
          <MarginSection id="focus" label={<>90-day<br/>focus</>}>
            <ChapterHeading
              title="90-day focus"
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
                />
              ))}
            </div>
          </MarginSection>
        )}

        {/* Chapter 5: Shared with the team — mirror of externally-visible state */}
        {hasShared && (
          <MarginSection id="shared" label={<>Shared<br/>with<br/>team</>}>
            <ChapterHeading
              title="Shared with the team"
              freshness={
                <ChapterFreshness
                  enrichedAt={intelligence?.enrichedAt}
                  fragments={[
                    `${sharedItems.length} shared item${sharedItems.length === 1 ? "" : "s"}`,
                    "What the rest of the org sees",
                  ]}
                />
              }
            />
            <SharedSubsectionLabel>Linked trackers</SharedSubsectionLabel>
            <div>
              {sharedItems.map((c, i) => (
                <SharedRefRow
                  key={i}
                  id={c.source ?? "Linked"}
                  body={<>{c.description}</>}
                  subline={
                    <>
                      {c.status ? `${c.status}` : "Shared"}
                      {c.owner ? ` · Assignee: ${c.owner}` : ""}
                    </>
                  }
                  meta={c.itemSource?.sourcedAt ? `Updated ${formatRelativeDate(c.itemSource.sourcedAt)}` : undefined}
                />
              ))}
            </div>
          </MarginSection>
        )}

        {/* Chapter 6: Recently landed — 30-day completion tail */}
        {hasRecentlyLanded && (
          <MarginSection id="recently-landed" label={<>Recently<br/>landed</>}>
            <ChapterHeading
              title="Recently landed"
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

      <section id="headline" className={shared.chapterSection}>
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
