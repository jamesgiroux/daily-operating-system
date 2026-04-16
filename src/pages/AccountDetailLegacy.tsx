/**
 * AccountDetailLegacy — Temporary child route for the account detail page.
 *
 * DOS-111: Renders the body sections (outlook through finis) that were
 * previously in AccountDetailEditorial. Consumes data from the parent
 * shell via AccountDetailContext. Will be replaced by tab routes in
 * future strangler fig steps.
 */
import { useAccountDetailCtx } from "@/contexts/AccountDetailContext";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { AccountOutlook } from "@/components/entity/AccountOutlook";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
import { WatchListPrograms } from "@/components/account/WatchListPrograms";
import { AccountProductsSection } from "@/components/account/AccountProductsSection";
import { AccountPortfolioSection } from "@/components/account/AccountPortfolioSection";
import { AccountHealthSection } from "@/components/account/AccountHealthSection";
import { AccountPullQuote } from "@/components/account/AccountPullQuote";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";
import { AccountReportsSection } from "@/components/account/AccountReportsSection";
import { RecommendedActions } from "@/components/entity/RecommendedActions";

export default function AccountDetailLegacy() {
  const {
    accountId,
    acct,
    preset,
    feedback,
    entityCtx,
    handleUpdateIntelField,
  } = useAccountDetailCtx();

  const detail = acct.detail!;
  const intelligence = detail.intelligence ?? null;
  const fb = { get: feedback.getFeedback, submit: feedback.submitFeedback };

  return (
    <>
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
      {intelligence?.health && <AccountHealthSection health={intelligence.health} consistencyFindings={intelligence.consistencyFindings} />}

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
        {intelligence?.recommendedActions && intelligence.recommendedActions.length > 0 && (
          <RecommendedActions entityId={detail.id} entityType="account"
            actions={intelligence.recommendedActions} onRefresh={acct.silentRefresh} />
        )}
        <TheWork data={{ ...detail, accountId: detail.id }} sectionId="" addingAction={acct.addingAction}
          setAddingAction={acct.setAddingAction} newActionTitle={acct.newActionTitle}
          setNewActionTitle={acct.setNewActionTitle} creatingAction={acct.creatingAction}
          onCreateAction={acct.handleCreateAction} onRefresh={acct.silentRefresh} />
      </MarginSection>

      <AccountReportsSection accountId={accountId} presetId={preset?.id} />

      {acct.files.length > 0 && <MarginSection label="Files" reveal={false}><FileListSection files={acct.files} /></MarginSection>}

      <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
    </>
  );
}
