/**
 * AccountDetailPage — Clean rebuild of the account detail page.
 *
 * Single flat route, state-based view switching, no child routes.
 * Built step by step per plan at ~/.claude/plans/deep-wiggling-hearth.md.
 *
 * Step 2: Shell chrome + shared header (hero, vitals, preset fields, rollover).
 */
import { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useAccountFieldSave } from "@/hooks/useAccountFieldSave";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { FolioReportsDropdown } from "@/components/folio/FolioReportsDropdown";
import { FolioToolsDropdown } from "@/components/folio/FolioToolsDropdown";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import { AccountBreadcrumbs } from "@/components/account/AccountBreadcrumbs";
import { AccountRolloverPrompt } from "@/components/account/AccountRolloverPrompt";
import { AccountDialogs } from "@/components/account/AccountDialogs";
import { buildAccountVitals, buildChapters } from "@/components/account/account-detail-utils";

import shared from "@/styles/entity-detail.module.css";

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  const preset = useActivePreset();
  useRevealObserver(!acct.loading && !!acct.detail);

  const { saveStatus, setSaveStatus: setFolioSaveStatus,
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

  if (acct.loading) return <EditorialLoading />;
  if (acct.error || !acct.detail) return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;

  const detail = acct.detail;
  const intelligence = detail.intelligence ?? null;

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

      {/* Views will be added in Steps 4-5 */}

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
