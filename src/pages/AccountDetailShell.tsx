/**
 * AccountDetailShell — Parent route for account detail.
 *
 * DOS-111: Strangler fig step 2. This shell:
 *   1. Calls all hooks previously in AccountDetailEditorial
 *   2. Renders the headline section (hero, vitals, preset, rollover)
 *   3. Passes data to children via AccountDetailContext
 *   4. Renders AccountDialogs below the Outlet
 *
 * The child index route (AccountDetailLegacy) renders everything
 * from the outlook section through the finis marker.
 */
import { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate, useRouterState, Outlet } from "@tanstack/react-router";
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
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import { AccountBreadcrumbs } from "@/components/account/AccountBreadcrumbs";
import { AccountRolloverPrompt } from "@/components/account/AccountRolloverPrompt";
import { AccountDialogs } from "@/components/account/AccountDialogs";
import { AccountDetailProvider } from "@/contexts/AccountDetailContext";
import type { AccountDetailContextValue } from "@/contexts/AccountDetailContext";
import { buildAccountVitals, buildHealthChapters, buildContextChapters, buildWorkChapters } from "@/components/account/account-detail-utils";

import shared from "@/styles/entity-detail.module.css";

export default function AccountDetailShell() {
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

  // DOS-112: Per-view chapter navigation
  const routerState = useRouterState();
  const deepestPath = routerState.matches[routerState.matches.length - 1]?.routeId ?? "";
  const activeView = deepestPath.includes("/health") ? "health"
    : deepestPath.includes("/context") ? "context"
    : deepestPath.includes("/work") ? "work"
    : "health";

  const chapters = useMemo(() => {
    if (activeView === "health") return buildHealthChapters(acct.detail?.isParent ?? false, !!acct.intelligence?.health);
    if (activeView === "context") return buildContextChapters();
    return buildWorkChapters();
  }, [activeView, acct.detail?.isParent, acct.intelligence?.health]);

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

  // Loading / error gates — render before context is available
  if (acct.loading) return <EditorialLoading />;
  if (acct.error || !acct.detail) return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;

  const detail = acct.detail;
  const intelligence = detail.intelligence ?? null;

  const handleMetadataChange = (key: string, value: string) => {
    setMetadataValues((prev) => { const updated = { ...prev, [key]: value }; void saveMetadata(updated); return updated; });
  };

  const ctxValue: AccountDetailContextValue = {
    accountId: accountId!,
    acct,
    preset,
    feedback: { getFeedback: feedback.getFeedback, submitFeedback: feedback.submitFeedback },
    entityCtx,
    handleUpdateIntelField,
    saveStatus,
    setFolioSaveStatus,
    saveMetadata,
    saveAccountField,
    conflictsForStrip,
    metadataValues,
    handleMetadataChange,
    ancestors,
    rolloverDismissed,
    setRolloverDismissed,
    mergeDialogOpen,
    setMergeDialogOpen,
    archiveDialogOpen,
    setArchiveDialogOpen,
  };

  return (
    <AccountDetailProvider value={ctxValue}>
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

      <Outlet />

      <AccountDialogs accountId={accountId!} accountName={detail.name} accountType={detail.accountType}
        archiveDialogOpen={archiveDialogOpen} onArchiveDialogChange={setArchiveDialogOpen} onArchive={acct.handleArchive}
        createChildOpen={acct.createChildOpen} onCreateChildOpenChange={acct.setCreateChildOpen}
        childName={acct.childName} onChildNameChange={acct.setChildName}
        childDescription={acct.childDescription} onChildDescriptionChange={acct.setChildDescription}
        creatingChild={acct.creatingChild} onCreateChild={acct.handleCreateChild}
        mergeDialogOpen={mergeDialogOpen} onMergeDialogChange={setMergeDialogOpen}
        onMerged={() => navigate({ to: "/accounts" })} />
    </AccountDetailProvider>
  );
}
