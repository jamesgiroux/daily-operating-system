/**
 * useAccountDetailPage — Orchestrates all hooks and state for AccountDetailPage.
 *
 * Extracts logic from the page component so the TSX focuses on rendering.
 * Returns a flat object with everything the page needs.
 */
import { useState, useEffect, useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { buildPresetSentimentLabels, useAccountDetail } from "@/hooks/useAccountDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { useEntityContextEntries } from "@/hooks/useEntityContextEntries";
import { useAccountFieldSave } from "@/hooks/useAccountFieldSave";
import { toast } from "sonner";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { FolioReportsDropdown } from "@/components/folio/FolioReportsDropdown";
import { FolioToolsDropdown } from "@/components/folio/FolioToolsDropdown";
import { hasTriageContent } from "@/components/health/TriageSection";
import { hasDivergenceContent } from "@/components/health/DivergenceSection";
import {
  buildHealthChapters,
  buildContextChapters,
  buildWorkChapters,
} from "@/components/account/account-detail-utils";
import type { AccountView } from "@/components/account/AccountViewSwitcher";

import shared from "@/styles/entity-detail.module.css";

const VALID_VIEWS: AccountView[] = ["health", "context", "work"];

function readViewFromUrl(): AccountView {
  if (typeof window === "undefined") return "health";
  const params = new URLSearchParams(window.location.search);
  const v = params.get("view");
  return v && (VALID_VIEWS as string[]).includes(v) ? (v as AccountView) : "health";
}

export function useAccountDetailPage(accountId: string | undefined) {
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  const preset = useActivePreset();
  const sentiment = useMemo(
    () => ({
      ...acct.sentiment,
      presetLabels: buildPresetSentimentLabels(preset),
    }),
    [acct.sentiment, preset],
  );

  // View state — driven by ?view= URL param, synced back on change.
  // Declared first so useRevealObserver below can take it as a revision
  // key and re-observe reveals when the active tab changes.
  const [activeView, setActiveView] = useState<AccountView>(() => readViewFromUrl());

  // Re-observe on view switch. The page renders all three tab contents
  // into the DOM and toggles `display: none` — IntersectionObserver can
  // not fire on non-laid-out subtrees, so reveals on the hidden tabs
  // stay at opacity: 0 until we tear down + re-query. The activeView
  // revision does exactly that.
  useRevealObserver(!acct.loading && !!acct.detail, activeView);

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

  // Sync view → URL (replaceState, no navigation, no re-render)
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    if (params.get("view") === activeView) return;
    params.set("view", activeView);
    const newUrl = `${window.location.pathname}?${params.toString()}`;
    window.history.replaceState(null, "", newUrl);
  }, [activeView]);

  // Scroll to top on view change (each view has its own section layout)
  useEffect(() => {
    window.scrollTo({ top: 0, behavior: "auto" });
  }, [activeView]);

  // Per-view chapter arrays.
  // Work tab: the "shared" pill only appears when a commitment carries real
  // tracker provenance (DOS-75, v1.2.2). Until then the pill is suppressed
  // to avoid a dead-link nav anchor (Wave 0g Finding 2).
  const hasSharedData = useMemo(
    () => (acct.intelligence?.openCommitments ?? []).some(
      (c) => !!(c as { trackerLink?: { href?: string } }).trackerLink?.href,
    ),
    [acct.intelligence?.openCommitments],
  );
  const chapters = useMemo(() => {
    const intel = acct.intelligence;
    if (activeView === "health") {
      // Mirror renderHealthView's conditional chapter logic so the nav reflects
      // only chapters the page actually renders. Use the same guard helpers
      // the page uses so the two stay in lockstep.
      const findings = intel?.consistencyFindings ?? [];
      const glean = acct.gleanSignals;
      const showTriage = hasTriageContent(intel, glean, sentiment.current);
      const showDivergence = hasDivergenceContent(findings, glean);
      const fineState = !!intel && !showTriage && !showDivergence;
      const hasOutlook = !!(
        intel?.renewalOutlook ||
        intel?.expansionSignals?.length ||
        intel?.contractContext
      );
      return buildHealthChapters(
        acct.detail?.isParent ?? false,
        !!intel?.health,
        { fineState, hasOutlook },
      );
    }
    if (activeView === "context") {
      const priorityCount = intel?.strategicPriorities?.length ?? 0;
      const competitorCount = intel?.competitiveContext?.length ?? 0;
      const hasWhatMatters = !!(
        priorityCount ||
        competitorCount ||
        intel?.organizationalChanges?.length ||
        intel?.blockers?.length
      );
      const hasBuilt = !!(
        intel?.valueDelivered?.length ||
        intel?.successMetrics ||
        intel?.openCommitments?.length
      );
      const hasTechnical = !!acct.detail?.technicalFootprint;
      return buildContextChapters({ hasWhatMatters, hasBuilt, hasTechnical });
    }
    const hasFiles = (acct.files?.length ?? 0) > 0;
    return buildWorkChapters(hasSharedData, hasFiles);
  }, [
    activeView,
    acct.detail?.isParent,
    acct.detail?.technicalFootprint,
    acct.detail?.products,
    acct.intelligence,
    acct.gleanSignals,
    sentiment.current,
    acct.files,
    hasSharedData,
  ]);

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

  // DOS-231 Codex fix: persist a single gap-row field on
  // `account_technical_footprint` and refresh the account. Prompts the user
  // for the value inline so v1.2.1 doesn't need to ship a full structured
  // editor — the full editor lands with DOS-207.
  const captureTechnicalFootprintField = async (field: string) => {
    if (!accountId) return;
    const labelMap: Record<string, string> = {
      usage_tier: "Usage tier (e.g. enterprise, professional, starter)",
      services_stage: "Services stage (e.g. onboarding, implementation, optimization, steady-state)",
      support_tier: "Support tier (e.g. premium, standard, basic)",
      active_users: "Active users (integer)",
      open_tickets: "Open tickets (integer)",
      csat_score: "CSAT score (0 - 5)",
      adoption_score: "Adoption score (0.0 - 1.0)",
    };
    const prompt = labelMap[field] ?? `New value for ${field}`;
    const value = typeof window !== "undefined" ? window.prompt(prompt) : null;
    if (value == null) return;
    const trimmed = value.trim();
    if (!trimmed) return;
    try {
      await invoke("update_technical_footprint_field", {
        accountId, field, value: trimmed,
      });
      await acct.load();
      toast.success(`${field.replace(/_/g, " ")} saved`);
    } catch (err) {
      console.error("update_technical_footprint_field failed:", err);
      toast.error(`Failed to save ${field.replace(/_/g, " ")}`);
    }
  };

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
    sentiment,
    loading: acct.loading,
    error: acct.error,

    // View switching
    activeView,
    setActiveView,

    // Field operations
    handleUpdateIntelField,
    saveAccountField,
    captureTechnicalFootprintField,
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
