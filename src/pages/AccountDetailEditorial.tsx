import { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { useEntityContextEntries } from "@/hooks/useEntityContextEntries";
import { useAccountFieldSave } from "@/hooks/useAccountFieldSave";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { FolioReportsDropdown } from "@/components/folio/FolioReportsDropdown";
import { FolioToolsDropdown } from "@/components/folio/FolioToolsDropdown";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { AccountOutlook } from "@/components/entity/AccountOutlook";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
import { WatchListPrograms } from "@/components/account/WatchListPrograms";
import { AccountBreadcrumbs } from "@/components/account/AccountBreadcrumbs";
import { AccountRolloverPrompt } from "@/components/account/AccountRolloverPrompt";
import { AccountProductsSection } from "@/components/account/AccountProductsSection";
import { AccountPortfolioSection } from "@/components/account/AccountPortfolioSection";
import { AccountHealthSection } from "@/components/account/AccountHealthSection";
import { AccountPullQuote } from "@/components/account/AccountPullQuote";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";
import { AccountReportsSection } from "@/components/account/AccountReportsSection";
import { AccountDialogs } from "@/components/account/AccountDialogs";
import { buildAccountVitals, buildChapters } from "@/components/account/account-detail-utils";

import shared from "@/styles/entity-detail.module.css";

export default function AccountDetailEditorial() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  const preset = useActivePreset();
  useRevealObserver(!acct.loading && !!acct.detail);

  const { updateField: handleUpdateIntelField, saveStatus, setSaveStatus: setFolioSaveStatus,
  } = useIntelligenceFieldUpdate("account", accountId, acct.silentRefresh);

  const { saveMetadata, saveAccountField, conflictsForStrip } = useAccountFieldSave({
    accountId, detail: acct.detail, load: acct.load, silentRefresh: acct.silentRefresh, setFolioSaveStatus,
  });

  const chapters = useMemo(
    () => buildChapters(acct.detail?.isParent ?? false, !!acct.intelligence?.health),
    [acct.detail?.isParent, acct.intelligence?.health],
  );

  const shellConfig = useMemo(() => ({
    folioLabel: acct.detail?.accountType === "internal" ? "Internal" : acct.detail?.accountType === "partner" ? "Partner" : "Account",
    atmosphereColor: acct.detail?.accountType === "internal" ? "larkspur" as const : "turmeric" as const,
    activePage: "accounts" as const,
    backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/accounts" }) },
    chapters,
  }), [navigate, acct.detail?.accountType, chapters]);
  useRegisterMagazineShell(shellConfig);

  const [mergeDialogOpen, setMergeDialogOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  useUpdateFolioVolatile({
    folioStatusText: saveStatus === "saving" ? "Saving\u2026" : saveStatus === "saved" ? "\u2713 Saved" : undefined,
    folioActions: (
      <div className={shared.folioActions}>
        {acct.detail && !acct.detail.archived && (
          <FolioRefreshButton onClick={acct.handleEnrich} loading={!!acct.enriching}
            loadingProgress={acct.enriching ? acct.enrichmentPercentage != null ? `${acct.enrichmentPercentage}%` : `${acct.enrichSeconds ?? 0}s` : undefined} />
        )}
        <FolioReportsDropdown accountId={accountId!} />
        <FolioToolsDropdown onCreateChild={() => acct.setCreateChildOpen(true)} onMerge={() => setMergeDialogOpen(true)}
          onArchive={() => setArchiveDialogOpen(true)} onUnarchive={acct.handleUnarchive} onIndexFiles={acct.handleIndexFiles}
          isArchived={!!acct.detail?.archived} isIndexing={acct.indexing} hasDetail={!!acct.detail} />
      </div>
    ),
  }, accountId);

  const [rolloverDismissed, setRolloverDismissed] = useState(false);
  const [metadataValues, setMetadataValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!accountId) return;
    invoke<string>("get_entity_metadata", { entityType: "account", entityId: accountId })
      .then((json) => { try { setMetadataValues(JSON.parse(json) ?? {}); } catch { setMetadataValues({}); } })
      .catch(() => setMetadataValues({}));
  }, [accountId]);

  const [ancestors, setAncestors] = useState<{ id: string; name: string }[]>([]);
  useEffect(() => {
    if (!accountId) return;
    invoke<{ id: string; name: string }[]>("get_account_ancestors", { accountId })
      .then(setAncestors).catch(() => setAncestors([]));
  }, [accountId]);

  const feedback = useIntelligenceFeedback(accountId, "account");
  const entityCtx = useEntityContextEntries("account", accountId ?? null);
  const detail = acct.detail;
  const intelligence = detail?.intelligence ?? null;
  const fb = { get: feedback.getFeedback, submit: feedback.submitFeedback };

  if (acct.loading) return <EditorialLoading />;
  if (acct.error || !detail) return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;

  const handleMetadataChange = (key: string, value: string) => {
    setMetadataValues((prev) => { const updated = { ...prev, [key]: value }; void saveMetadata(updated); return updated; });
  };

  return (
    <>
      <AccountBreadcrumbs ancestors={ancestors} currentName={detail.name ?? ""} />

      <section id="headline" className={shared.chapterSection}>
        <AccountHero detail={detail} intelligence={intelligence}
          editName={acct.editName} setEditName={(v) => { acct.setEditName(v); acct.setDirty(true); }}
          editHealth={acct.editHealth} setEditHealth={(v) => { acct.setEditHealth(v); acct.setDirty(true); }}
          editLifecycle={acct.editLifecycle} setEditLifecycle={(v) => { acct.setEditLifecycle(v); acct.setDirty(true); }}
          onSave={acct.handleSave} onSaveField={saveAccountField}
          vitalsSlot={detail.accountType !== "internal" ? (preset
            ? <EditableVitalsStrip fields={preset.vitals.account} entityData={detail} metadata={metadataValues}
                onFieldChange={(key, col, source, value) => {
                  if (source === "metadata") handleMetadataChange(key, value);
                  else if (source === "column") void saveAccountField(col ?? key, value);
                }} conflicts={conflictsForStrip} sourceRefs={detail.sourceRefs} />
            : <VitalsStrip vitals={buildAccountVitals(detail)} sourceRefs={detail.sourceRefs} />
          ) : undefined}
          provenanceSlot={undefined} />
        {preset && preset.metadata.account.length > 0 && (
          <div className={`editorial-reveal ${shared.presetFieldsReveal}`}>
            <PresetFieldsEditor fields={preset.metadata.account} values={metadataValues} onChange={handleMetadataChange} />
          </div>
        )}
        {detail.renewalDate && !rolloverDismissed && (
          <AccountRolloverPrompt renewalDate={detail.renewalDate}
            onRenewed={() => { acct.setNewEventType("renewal"); acct.setNewEventDate(detail.renewalDate!); acct.handleRecordEvent(); setRolloverDismissed(true); }}
            onChurned={() => { acct.setNewEventType("churn"); acct.setNewEventDate(detail.renewalDate!); acct.handleRecordEvent(); setRolloverDismissed(true); }}
            onDismiss={() => setRolloverDismissed(true)} />
        )}
      </section>

      {intelligence && (intelligence.renewalOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
        <MarginSection id="outlook" label="Outlook">
          <ChapterHeading title="Outlook" />
          <AccountOutlook intelligence={intelligence} onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

      <AccountProductsSection accountId={detail.id} products={detail.products ?? []}
        getFeedback={fb.get} onFeedback={fb.submit} onRefresh={acct.load} silentRefresh={acct.silentRefresh} />

      {detail.isParent && detail.children.length > 0 && (
        <AccountPortfolioSection children={detail.children} intelligence={intelligence} />
      )}

      <MarginSection id="state-of-play" label={<>State of<br/>Play</>}>
        <StateOfPlay intelligence={intelligence} sectionId="" onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        {detail.technicalFootprint && <AccountTechnicalFootprint footprint={detail.technicalFootprint} />}
      </MarginSection>

      {intelligence && <AccountPullQuote intelligence={intelligence} />}
      {intelligence?.health && <AccountHealthSection health={intelligence.health} />}

      <MarginSection id="the-room" label={<>The<br/>Room</>}>
        <StakeholderGallery intelligence={intelligence} linkedPeople={detail.linkedPeople}
          accountTeam={detail.accountTeam} stakeholdersFull={detail.stakeholdersFull} sectionId=""
          entityId={accountId} entityType="account" onIntelligenceUpdated={acct.silentRefresh}
          onRemoveTeamMember={acct.handleRemoveTeamMember} onChangeTeamRole={acct.changeTeamMemberRole}
          onAddTeamMember={acct.addTeamMemberDirect} onCreateTeamMember={acct.createTeamMemberDirect}
          teamSearchQuery={acct.teamSearchQuery} onTeamSearchQueryChange={acct.setTeamSearchQuery}
          teamSearchResults={acct.teamSearchResults} suggestions={acct.suggestions}
          onAcceptSuggestion={acct.acceptSuggestion} onDismissSuggestion={acct.dismissSuggestion}
          onUpdateEngagement={acct.updateStakeholderEngagement} onUpdateAssessment={acct.updateStakeholderAssessment}
          onAddRole={acct.addStakeholderRole} onRemoveRole={acct.removeStakeholderRole} />
      </MarginSection>

      <MarginSection id="watch-list" label={<>Watch<br/>List</>}>
        <WatchList intelligence={intelligence} sectionId="" onUpdateField={handleUpdateIntelField}
          getItemFeedback={fb.get} onItemFeedback={fb.submit}
          bottomSection={<WatchListPrograms programs={acct.programs} onProgramUpdate={acct.handleProgramUpdate}
            onProgramDelete={acct.handleProgramDelete} onAddProgram={acct.handleAddProgram} />} />
      </MarginSection>

      {intelligence && (intelligence.valueDelivered?.length || intelligence.successMetrics?.length || intelligence.openCommitments?.length) ? (
        <MarginSection id="value-commitments" label={<>Value &amp;<br/>Commitments</>}>
          <ChapterHeading title="Value & Commitments" />
          <ValueCommitments intelligence={intelligence} onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

      {intelligence && (intelligence.strategicPriorities?.length || intelligence.competitiveContext?.length || intelligence.organizationalChanges?.length || intelligence.blockers?.length) ? (
        <MarginSection id="strategic-landscape" label={<>Competitive &amp;<br/>Strategic</>}>
          <ChapterHeading title="Competitive & Strategic" />
          <StrategicLandscape intelligence={intelligence} onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

      <MarginSection id="the-record" label={<>The<br/>Record</>}>
        <UnifiedTimeline data={{ ...detail, accountEvents: acct.events, lifecycleChanges: detail.lifecycleChanges,
          autoCompletedMilestones: detail.autoCompletedMilestones, contextEntries: entityCtx.entries }} sectionId=""
          actionSlot={<AddToRecord onAdd={(title, content) => entityCtx.createEntry(title, content)} />} />
      </MarginSection>

      <MarginSection id="the-work" label={<>The<br/>Work</>}>
        <TheWork data={{ ...detail, accountId: detail.id }} sectionId="" addingAction={acct.addingAction}
          setAddingAction={acct.setAddingAction} newActionTitle={acct.newActionTitle}
          setNewActionTitle={acct.setNewActionTitle} creatingAction={acct.creatingAction}
          onCreateAction={acct.handleCreateAction} onRefresh={acct.silentRefresh} />
      </MarginSection>

      <AccountReportsSection accountId={accountId!} presetId={preset?.id} />

      {acct.files.length > 0 && <MarginSection label="Files" reveal={false}><FileListSection files={acct.files} /></MarginSection>}

      <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>

      <AccountDialogs accountId={accountId!} accountName={detail.name} accountType={detail.accountType}
        archiveDialogOpen={archiveDialogOpen} onArchiveDialogChange={setArchiveDialogOpen} onArchive={acct.handleArchive}
        createChildOpen={acct.createChildOpen} onCreateChildOpenChange={acct.setCreateChildOpen}
        childName={acct.childName} onChildNameChange={acct.setChildName}
        childDescription={acct.childDescription} onChildDescriptionChange={acct.setChildDescription}
        creatingChild={acct.creatingChild} onCreateChild={acct.handleCreateChild}
        mergeDialogOpen={mergeDialogOpen} onMergeDialogChange={setMergeDialogOpen}
        onMerged={() => navigate({ to: "/accounts" })} />
    </>
  );
}
