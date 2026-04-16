/**
 * AccountDetailPage — Clean rebuild of the account detail page.
 *
 * Single flat route, state-based view switching, no child routes.
 * Built step by step per plan at ~/.claude/plans/deep-wiggling-hearth.md.
 *
 * Step 3: Hook extracted to useAccountDetailPage. Page focuses on rendering.
 */
import { useParams } from "@tanstack/react-router";
import { useAccountDetailPage } from "@/hooks/useAccountDetailPage";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import { AccountBreadcrumbs } from "@/components/account/AccountBreadcrumbs";
import { AccountRolloverPrompt } from "@/components/account/AccountRolloverPrompt";
import { AccountDialogs } from "@/components/account/AccountDialogs";
import { buildAccountVitals } from "@/components/account/account-detail-utils";

import shared from "@/styles/entity-detail.module.css";

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const page = useAccountDetailPage(accountId);

  if (page.loading) return <EditorialLoading />;
  if (page.error || !page.detail) return <EditorialError message={page.error ?? "Account not found"} onRetry={page.acct.load} />;

  const { detail, intelligence, acct, preset } = page;

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

      {/* Views will be added in Steps 4-5 */}

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
