/**
 * AccountContextView — Context JTBD view for the account detail page.
 *
 * DOS-112: Sections: PullQuote, State of Play + Tech Footprint,
 * Strategic Landscape, Stakeholder Gallery, Value Commitments,
 * Unified Timeline + Add to Record, Files, Finis.
 * Components are moved AS-IS from AccountDetailLegacy — no rewrites.
 */
import { useAccountDetailCtx } from "@/contexts/AccountDetailContext";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
import { AccountPullQuote } from "@/components/account/AccountPullQuote";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";

export default function AccountContextView() {
  const {
    accountId,
    acct,
    feedback,
    entityCtx,
    handleUpdateIntelField,
  } = useAccountDetailCtx();

  const detail = acct.detail!;
  const intelligence = detail.intelligence ?? null;
  const fb = { get: feedback.getFeedback, submit: feedback.submitFeedback };

  return (
    <>
      {intelligence && <AccountPullQuote intelligence={intelligence} />}

      <MarginSection id="state-of-play" label={<>State of<br/>Play</>}>
        <StateOfPlay intelligence={intelligence} sectionId="" onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        {detail.technicalFootprint && <AccountTechnicalFootprint footprint={detail.technicalFootprint} />}
      </MarginSection>

      {intelligence && (intelligence.strategicPriorities?.length || intelligence.competitiveContext?.length || intelligence.organizationalChanges?.length || intelligence.blockers?.length) ? (
        <MarginSection id="strategic-landscape" label={<>Competitive &amp;<br/>Strategic</>}>
          <ChapterHeading title="Competitive & Strategic" />
          <StrategicLandscape intelligence={intelligence} onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

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

      {intelligence && (intelligence.valueDelivered?.length || intelligence.successMetrics?.length || intelligence.openCommitments?.length) ? (
        <MarginSection id="value-commitments" label={<>Value &amp;<br/>Commitments</>}>
          <ChapterHeading title="Value & Commitments" />
          <ValueCommitments intelligence={intelligence} onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
        </MarginSection>
      ) : null}

      <MarginSection id="the-record" label={<>The<br/>Record</>}>
        <UnifiedTimeline data={{ ...detail, accountEvents: acct.events, lifecycleChanges: detail.lifecycleChanges,
          autoCompletedMilestones: detail.autoCompletedMilestones, contextEntries: entityCtx.entries }} sectionId=""
          actionSlot={<AddToRecord onAdd={(title, content) => entityCtx.createEntry(title, content)} />} />
      </MarginSection>

      {acct.files.length > 0 && <MarginSection label="Files" reveal={false}><FileListSection files={acct.files} /></MarginSection>}

      <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
    </>
  );
}
