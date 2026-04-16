/**
 * useAccountDetailPage — Orchestrates all hooks and state for AccountDetailPage.
 *
 * Extracts logic from the page component so the TSX focuses on rendering.
 * Returns a flat object with everything the page needs.
 */
import { useState, useEffect, useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { useEntityContextEntries } from "@/hooks/useEntityContextEntries";
import { useAccountFieldSave } from "@/hooks/useAccountFieldSave";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { FolioReportsDropdown } from "@/components/folio/FolioReportsDropdown";
import { FolioToolsDropdown } from "@/components/folio/FolioToolsDropdown";
import { buildChapters } from "@/components/account/account-detail-utils";

import shared from "@/styles/entity-detail.module.css";

export function useAccountDetailPage(accountId: string | undefined) {
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  const preset = useActivePreset();
  useRevealObserver(!acct.loading && !!acct.detail);

  // Intelligence field mutations
  const { updateField: handleUpdateIntelField, saveStatus, setSaveStatus: setFolioSaveStatus,
  } = useIntelligenceFieldUpdate("account", accountId, acct.silentRefresh);

  // Account field saves
  const { saveMetadata, saveAccountField, conflictsForStrip } = useAccountFieldSave({
    accountId, detail: acct.detail, load: acct.load, silentRefresh: acct.silentRefresh, setFolioSaveStatus,
  });

  // Feedback + entity context (used by view sections)
  const feedback = useIntelligenceFeedback(accountId, "account");
  const entityCtx = useEntityContextEntries("account", accountId ?? null);

  // Chapter navigation (all chapters for now; will switch to per-view in Step 4)
  const chapters = useMemo(
    () => buildChapters(acct.detail?.isParent ?? false, !!acct.intelligence?.health),
    [acct.detail?.isParent, acct.intelligence?.health],
  );

  // Magazine shell registration
  const shellConfig = useMemo(() => ({
    folioLabel: acct.detail?.accountType === "internal" ? "Internal" : acct.detail?.accountType === "partner" ? "Partner" : "Account",
    atmosphereColor: acct.detail?.accountType === "internal" ? "larkspur" as const : "turmeric" as const,
    activePage: "accounts" as const,
    backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/accounts" }) },
    chapters,
  }), [navigate, acct.detail?.accountType, chapters]);
  useRegisterMagazineShell(shellConfig);

  // Dialog state
  const [mergeDialogOpen, setMergeDialogOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  // Folio volatile state (actions, save status)
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

  // Rollover prompt
  const [rolloverDismissed, setRolloverDismissed] = useState(false);

  // Entity metadata
  const [metadataValues, setMetadataValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!accountId) return;
    invoke<string>("get_entity_metadata", { entityType: "account", entityId: accountId })
      .then((json) => { try { setMetadataValues(JSON.parse(json) ?? {}); } catch { setMetadataValues({}); } })
      .catch(() => setMetadataValues({}));
  }, [accountId]);

  // Ancestor breadcrumbs
  const [ancestors, setAncestors] = useState<{ id: string; name: string }[]>([]);
  useEffect(() => {
    if (!accountId) return;
    invoke<{ id: string; name: string }[]>("get_account_ancestors", { accountId })
      .then(setAncestors).catch(() => setAncestors([]));
  }, [accountId]);

  // Metadata change handler
  const handleMetadataChange = (key: string, value: string) => {
    setMetadataValues((prev) => { const updated = { ...prev, [key]: value }; void saveMetadata(updated); return updated; });
  };

  return {
    // Core data
    acct,
    preset,
    accountId: accountId!,
    navigate,

    // Derived
    detail: acct.detail,
    intelligence: acct.detail?.intelligence ?? null,
    loading: acct.loading,
    error: acct.error,

    // Field operations
    handleUpdateIntelField,
    saveAccountField,
    saveMetadata,
    conflictsForStrip,
    metadataValues,
    handleMetadataChange,

    // Feedback
    feedback: { get: feedback.getFeedback, submit: feedback.submitFeedback },

    // Entity context (timeline entries)
    entityCtx,

    // Ancestors
    ancestors,

    // Rollover
    rolloverDismissed,
    setRolloverDismissed,

    // Dialogs
    mergeDialogOpen,
    setMergeDialogOpen,
    archiveDialogOpen,
    setArchiveDialogOpen,
  };
}
