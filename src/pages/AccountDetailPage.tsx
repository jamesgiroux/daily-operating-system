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
// View 2 — Context
import { AccountExecutiveSummary } from "@/components/account/AccountExecutiveSummary";
import { AccountPullQuote } from "@/components/account/AccountPullQuote";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
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

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const page = useAccountDetailPage(accountId);

  if (page.loading) return <EditorialLoading />;
  if (page.error || !page.detail) return <EditorialError message={page.error ?? "Account not found"} onRetry={page.acct.load} />;

  const { detail, intelligence, acct, preset, activeView } = page;
  const fb = page.feedback;

  // ─── View 1: Health & Outlook ───────────────────────────────────────────
  const renderHealthView = () => (
    <>
      {intelligence?.health && (
        <AccountHealthSection health={intelligence.health} consistencyFindings={intelligence.consistencyFindings} />
      )}

      {intelligence && (intelligence.renewalOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
        <MarginSection id="outlook" label="Outlook">
          <ChapterHeading title="Outlook" />
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

  // ─── View 2: Context ────────────────────────────────────────────────────
  const renderContextView = () => (
    <>
      <AccountExecutiveSummary intelligence={intelligence} />

      {intelligence && <AccountPullQuote intelligence={intelligence} />}

      <MarginSection id="state-of-play" label={<>State of<br/>Play</>}>
        <StateOfPlay intelligence={intelligence} sectionId="" onUpdateField={page.handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        {detail.technicalFootprint && <AccountTechnicalFootprint footprint={detail.technicalFootprint} />}
      </MarginSection>

      {intelligence && (intelligence.strategicPriorities?.length || intelligence.competitiveContext?.length || intelligence.organizationalChanges?.length || intelligence.blockers?.length) ? (
        <MarginSection id="strategic-landscape" label={<>Competitive &amp;<br/>Strategic</>}>
          <ChapterHeading title="Competitive & Strategic" />
          <StrategicLandscape intelligence={intelligence} onUpdateField={page.handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

      <MarginSection id="the-room" label={<>The<br/>Room</>}>
        <StakeholderGallery intelligence={intelligence} linkedPeople={detail.linkedPeople}
          accountTeam={detail.accountTeam} stakeholdersFull={detail.stakeholdersFull} sectionId=""
          entityId={page.accountId} entityType="account" onIntelligenceUpdated={acct.silentRefresh}
          onRemoveTeamMember={acct.handleRemoveTeamMember} onChangeTeamRole={acct.changeTeamMemberRole}
          onAddTeamMember={acct.addTeamMemberDirect} onCreateTeamMember={acct.createTeamMemberDirect}
          teamSearchQuery={acct.teamSearchQuery} onTeamSearchQueryChange={acct.setTeamSearchQuery}
          teamSearchResults={acct.teamSearchResults} suggestions={acct.suggestions}
          onAcceptSuggestion={acct.acceptSuggestion} onDismissSuggestion={acct.dismissSuggestion}
          onUpdateEngagement={acct.updateStakeholderEngagement} onUpdateAssessment={acct.updateStakeholderAssessment}
          onAddRole={acct.addStakeholderRole} onRemoveRole={acct.removeStakeholderRole} />
      </MarginSection>

      {intelligence && (intelligence.valueDelivered?.length || intelligence.successMetrics?.length || intelligence.openCommitments?.length) ? (
        <MarginSection id="value-commitments" label={<>Value &amp;<br/>Commitments</>}>
          <ChapterHeading title="Value & Commitments" />
          <ValueCommitments intelligence={intelligence} onUpdateField={page.handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

      <MarginSection id="the-record" label={<>The<br/>Record</>}>
        <UnifiedTimeline data={{ ...detail, accountEvents: acct.events, lifecycleChanges: detail.lifecycleChanges,
          autoCompletedMilestones: detail.autoCompletedMilestones, contextEntries: page.entityCtx.entries }} sectionId=""
          actionSlot={<AddToRecord onAdd={(title, content) => page.entityCtx.createEntry(title, content)} />} />
      </MarginSection>

      {acct.files.length > 0 && (
        <MarginSection id="files" label="Files" reveal={false}>
          <FileListSection files={acct.files} />
        </MarginSection>
      )}

      <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
    </>
  );

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
      <div style={{ display: activeView === "health" ? "block" : "none" }}>
        {renderHealthView()}
      </div>
      <div style={{ display: activeView === "context" ? "block" : "none" }}>
        {renderContextView()}
      </div>
      <div style={{ display: activeView === "work" ? "block" : "none" }}>
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
