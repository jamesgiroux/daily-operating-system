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
// View 3 — The Work
import { RecommendedActions } from "@/components/entity/RecommendedActions";
import { TheWork } from "@/components/entity/TheWork";
import { WatchList } from "@/components/entity/WatchList";
import { WatchListPrograms } from "@/components/account/WatchListPrograms";
import { AccountReportsSection } from "@/components/account/AccountReportsSection";
import { buildAccountVitals } from "@/components/account/account-detail-utils";

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
  const renderWorkView = () => (
    <>
      <MarginSection id="the-work" label={<>The<br/>Work</>}>
        {intelligence?.recommendedActions && intelligence.recommendedActions.length > 0 && (
          <RecommendedActions entityId={detail.id} entityType="account"
            actions={intelligence.recommendedActions} onRefresh={acct.silentRefresh} />
        )}
        <TheWork data={{ ...detail, accountId: detail.id }} sectionId="" addingAction={acct.addingAction}
          setAddingAction={acct.setAddingAction} newActionTitle={acct.newActionTitle}
          setNewActionTitle={acct.setNewActionTitle} creatingAction={acct.creatingAction}
          onCreateAction={acct.handleCreateAction} onRefresh={acct.silentRefresh} />
      </MarginSection>

      <MarginSection id="watch-list" label={<>Watch<br/>List</>}>
        <WatchList intelligence={intelligence} sectionId="" onUpdateField={page.handleUpdateIntelField}
          getItemFeedback={fb.get} onItemFeedback={fb.submit}
          bottomSection={<WatchListPrograms programs={acct.programs} onProgramUpdate={acct.handleProgramUpdate}
            onProgramDelete={acct.handleProgramDelete} onAddProgram={acct.handleAddProgram} />} />
      </MarginSection>

      <AccountReportsSection accountId={page.accountId} presetId={preset?.id} />

      <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
    </>
  );

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
