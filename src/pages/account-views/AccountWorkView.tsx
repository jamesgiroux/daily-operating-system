/**
 * AccountWorkView — Work JTBD view for the account detail page.
 *
 * DOS-112: Sections: Recommended Actions + The Work, Watch List + Programs,
 * Reports, Finis.
 * Components are moved AS-IS from AccountDetailLegacy — no rewrites.
 */
import { useAccountDetailCtx } from "@/contexts/AccountDetailContext";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { WatchList } from "@/components/entity/WatchList";
import { TheWork } from "@/components/entity/TheWork";
import { RecommendedActions } from "@/components/entity/RecommendedActions";
import { WatchListPrograms } from "@/components/account/WatchListPrograms";
import { AccountReportsSection } from "@/components/account/AccountReportsSection";

export default function AccountWorkView() {
  const {
    accountId,
    acct,
    preset,
    feedback,
    handleUpdateIntelField,
  } = useAccountDetailCtx();

  const detail = acct.detail!;
  const intelligence = detail.intelligence ?? null;
  const fb = { get: feedback.getFeedback, submit: feedback.submitFeedback };

  return (
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
        <WatchList intelligence={intelligence} sectionId="" onUpdateField={handleUpdateIntelField}
          getItemFeedback={fb.get} onItemFeedback={fb.submit}
          bottomSection={<WatchListPrograms programs={acct.programs} onProgramUpdate={acct.handleProgramUpdate}
            onProgramDelete={acct.handleProgramDelete} onAddProgram={acct.handleAddProgram} />} />
      </MarginSection>

      <AccountReportsSection accountId={accountId} presetId={preset?.id} />

      <div className="editorial-reveal"><FinisMarker enrichedAt={intelligence?.enrichedAt} /></div>
    </>
  );
}
