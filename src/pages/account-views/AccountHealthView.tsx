/**
 * AccountHealthView — Health JTBD view for the account detail page.
 *
 * DOS-112: Sections: Health, Outlook, Portfolio (conditional), Products, Finis.
 * Components are moved AS-IS from AccountDetailLegacy — no rewrites.
 */
import { useAccountDetailCtx } from "@/contexts/AccountDetailContext";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MarginSection } from "@/components/editorial/MarginSection";
import { AccountOutlook } from "@/components/entity/AccountOutlook";
import { AccountProductsSection } from "@/components/account/AccountProductsSection";
import { AccountPortfolioSection } from "@/components/account/AccountPortfolioSection";
import { AccountHealthSection } from "@/components/account/AccountHealthSection";

export default function AccountHealthView() {
  const {
    acct,
    feedback,
    handleUpdateIntelField,
  } = useAccountDetailCtx();

  const detail = acct.detail!;
  const intelligence = detail.intelligence ?? null;
  const fb = { get: feedback.getFeedback, submit: feedback.submitFeedback };

  return (
    <>
      {intelligence?.health && <AccountHealthSection health={intelligence.health} consistencyFindings={intelligence.consistencyFindings} />}

      {intelligence && (intelligence.renewalOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
        <MarginSection id="outlook" label="Outlook">
          <ChapterHeading title="Outlook" />
          <AccountOutlook intelligence={intelligence} onUpdateField={handleUpdateIntelField} getItemFeedback={fb.get} onItemFeedback={fb.submit} />
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
}
