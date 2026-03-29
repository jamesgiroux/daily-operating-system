import React, { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { formatArr, formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import type { AccountProduct } from "@/types";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { getAccountReports } from "@/lib/report-config";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { FolioReportsDropdown } from "@/components/folio/FolioReportsDropdown";
import { FolioToolsDropdown } from "@/components/folio/FolioToolsDropdown";
import {
  AlignLeft,
  BarChart3,
  Briefcase,
  Clock,
  Users,
  Eye,
  Activity,
  CheckSquare2,
  FileText,
  TrendingUp,
  TrendingDown,
  Minus,
  Award,
  Compass,
  Telescope,
  ChevronDown,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip, type VitalConflict } from "@/components/entity/EditableVitalsStrip";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { AccountOutlook } from "@/components/entity/AccountOutlook";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { FileListSection } from "@/components/entity/FileListSection";
import { AccountMergeDialog } from "@/components/account/AccountMergeDialog";
import { WatchListPrograms } from "@/components/account/WatchListPrograms";
import { DimensionBar } from "@/components/shared/DimensionBar";
import { useEntityContextEntries } from "@/hooks/useEntityContextEntries";
import {
  formatProvenanceSource,
} from "@/components/ui/ProvenanceLabel";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";

import shared from "@/styles/entity-detail.module.css";
import styles from "./AccountDetailEditorial.module.css";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";

/* ── Vitals assembly (moved from old account/VitalsStrip) ── */

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round(
      (renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24),
    );
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    return `Renewal in ${diffDays}d`;
  } catch {
    return dateStr;
  }
}

const healthColorMap: Record<string, "saffron" | undefined> = {
  yellow: "saffron",
};

function buildAccountVitals(detail: {
  arr?: number | null;
  health?: string;
  lifecycle?: string;
  renewalDate?: string;
  renewalStage?: string | null;
  commercialStage?: string | null;
  nps?: number | null;
  signals?: { meetingFrequency30d?: number };
  contractStart?: string;
}): VitalDisplay[] {
  const vitals: VitalDisplay[] = [];
  if (detail.arr != null) {
    vitals.push({ text: `$${formatArr(detail.arr)} ARR`, highlight: "turmeric" });
  }
  if (detail.health) {
    vitals.push({
      text: `${detail.health.charAt(0).toUpperCase() + detail.health.slice(1)} Health`,
      highlight: healthColorMap[detail.health],
    });
  }
  if (detail.lifecycle) vitals.push({ text: detail.lifecycle });
  if (detail.renewalStage) {
    vitals.push({
      text: `Stage: ${detail.renewalStage.replace(/_/g, " ")}`,
      highlight: "olive",
    });
  }
  if (detail.commercialStage) {
    vitals.push({
      text: `Opp: ${detail.commercialStage.replace(/_/g, " ")}`,
      highlight: "larkspur",
    });
  }
  if (detail.renewalDate) {
    const renewal = new Date(detail.renewalDate);
    const now = new Date();
    const diffDays = Math.round((renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
    vitals.push({
      text: formatRenewalCountdown(detail.renewalDate),
      highlight: diffDays <= 60 ? "saffron" : undefined,
    });
  }
  if (detail.nps != null) vitals.push({ text: `NPS ${detail.nps}` });
  if (detail.signals?.meetingFrequency30d != null) {
    vitals.push({ text: `${detail.signals.meetingFrequency30d} meetings / 30d` });
  }
  if (detail.contractStart) {
    vitals.push({ text: `Contract: ${formatShortDate(detail.contractStart)}` });
  }
  return vitals;
}

// Chapter definitions for the editorial layout — icons match the v3 mockup nav island
// I393: Portfolio chapter conditionally included for parent accounts
const BASE_CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Headline", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "outlook", label: "Outlook", icon: <Telescope size={18} strokeWidth={1.5} /> },
  { id: "state-of-play", label: "State of Play", icon: <Clock size={18} strokeWidth={1.5} /> },
  { id: "the-room", label: "The Room", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "watch-list", label: "Watch List", icon: <Eye size={18} strokeWidth={1.5} /> },
  { id: "value-commitments", label: "Value & Commitments", icon: <Award size={18} strokeWidth={1.5} /> },
  { id: "strategic-landscape", label: "Competitive & Strategic", icon: <Compass size={18} strokeWidth={1.5} /> },
  { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
  { id: "the-work", label: "The Work", icon: <CheckSquare2 size={18} strokeWidth={1.5} /> },
  { id: "reports", label: "Reports", icon: <FileText size={18} strokeWidth={1.5} /> },
];

const PORTFOLIO_CHAPTER = {
  id: "portfolio",
  label: "Portfolio",
  icon: <Briefcase size={18} strokeWidth={1.5} />,
};

const HEALTH_CHAPTER = {
  id: "relationship-health",
  label: "Health",
  icon: <BarChart3 size={18} strokeWidth={1.5} />,
};

function buildChapters(isParent: boolean, hasHealth: boolean) {
  // BASE_CHAPTERS: [headline, outlook, state-of-play, the-room, watch-list, ...]
  let chapters = [...BASE_CHAPTERS];
  // Portfolio inserts after outlook (index 2), before state-of-play
  if (isParent) {
    chapters.splice(2, 0, PORTFOLIO_CHAPTER);
  }
  // Health inserts after state-of-play, before the-room
  const sopIndex = chapters.findIndex((c) => c.id === "state-of-play");
  if (hasHealth && sopIndex >= 0) {
    chapters.splice(sopIndex + 1, 0, HEALTH_CHAPTER);
  }
  return chapters;
}

function getHealthColorClass(health: string): string {
  if (health === "green") return styles.healthGreen;
  if (health === "red") return styles.healthRed;
  return styles.healthYellow;
}

function getHealthDotClass(health: string): string {
  if (health === "green") return styles.healthDotGreen;
  if (health === "red") return styles.healthDotRed;
  return styles.healthDotYellow;
}

function formatTrackedFieldLabel(field: string): string {
  const labels: Record<string, string> = {
    arr: "ARR",
    lifecycle: "Lifecycle",
    contract_end: "Renewal Date",
    nps: "NPS",
  };
  return labels[field] ?? field.replace(/_/g, " ");
}

function formatLifecycleDisplay(value?: string | null): string {
  if (!value) return "Unknown";
  return value
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function formatSuggestedValue(field: string, value: string): string {
  if (field === "arr") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? `$${formatArr(parsed)} ARR` : value;
  }
  if (field === "contract_end") {
    return `Renews ${formatShortDate(value)}`;
  }
  if (field === "lifecycle") {
    return formatLifecycleDisplay(value);
  }
  if (field === "nps") {
    return `NPS ${value}`;
  }
  return value;
}

export default function AccountDetailEditorial() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  const preset = useActivePreset();
  useRevealObserver(!acct.loading && !!acct.detail);

  // Dropdown open/close state moved to FolioReportsDropdown / FolioToolsDropdown

  // I352: Shared intelligence field update hook (must be before shellConfig useMemo)
  const {
    updateField: handleUpdateIntelField,
    saveStatus,
    setSaveStatus: setFolioSaveStatus,
  } = useIntelligenceFieldUpdate("account", accountId, acct.silentRefresh);

  const finishFolioSave = () => {
    setFolioSaveStatus("saved");
    window.setTimeout(() => setFolioSaveStatus("idle"), 2000);
  };

  const saveMetadata = async (updated: Record<string, string>) => {
    if (!accountId) return;
    setFolioSaveStatus("saving");
    try {
      await invoke("update_entity_metadata", {
        entityId: accountId,
        entityType: "account",
        metadata: JSON.stringify(updated),
      });
      finishFolioSave();
    } catch (err) {
      console.error("update_entity_metadata failed:", err);
      toast.error("Failed to save metadata");
      setFolioSaveStatus("idle");
      throw err;
    }
  };

  const saveAccountField = async (field: string, value: string) => {
    if (!acct.detail) return;
    setFolioSaveStatus("saving");
    try {
      await invoke("update_account_field", { accountId: acct.detail.id, field, value });
      await acct.load();
      finishFolioSave();
    } catch (err) {
      console.error("update_account_field failed:", err);
      toast.error("Failed to save field");
      setFolioSaveStatus("idle");
    }
  };

  // Register magazine shell configuration — MagazinePageLayout consumes this
  // Memoize chapters separately — they only change with isParent/hasHealth,
  // not with frequently-changing folio state (saveStatus, enrichSeconds, etc.)
  const chapters = useMemo(
    () => buildChapters(acct.detail?.isParent ?? false, !!acct.intelligence?.health),
    [acct.detail?.isParent, acct.intelligence?.health],
  );

  // I563: Split stable shell config from volatile folio state to prevent render loops.
  // Stable config only changes on page/entity identity changes → re-registers rarely.
  const shellConfig = useMemo(
    () => ({
      folioLabel: acct.detail?.accountType === "internal" ? "Internal" : acct.detail?.accountType === "partner" ? "Partner" : "Account",
      atmosphereColor: acct.detail?.accountType === "internal" ? "larkspur" as const : "turmeric" as const,
      activePage: "accounts" as const,
      backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/accounts" }) },
      chapters,
    }),
    [navigate, acct.detail?.accountType, chapters],
  );
  useRegisterMagazineShell(shellConfig);

  // Dialog open state — must be before useUpdateFolioVolatile (used by FolioToolsDropdown callbacks)
  const [mergeDialogOpen, setMergeDialogOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  // I563: Volatile folio state — updates via ref, no re-registration.
  // Pass accountId as repaintKey so MagazinePageLayout re-reads when navigating
  // between accounts (same route, different params → component doesn't remount).
  // Dropdown components own their own state — no closures leak into the volatile ref.
  useUpdateFolioVolatile({
    folioStatusText: saveStatus === "saving" ? "Saving\u2026" : saveStatus === "saved" ? "\u2713 Saved" : undefined,
    folioActions: (
      <div className={shared.folioActions}>
        {acct.detail && !acct.detail.archived && (
          <FolioRefreshButton
            onClick={acct.handleEnrich}
            loading={!!acct.enriching}
            loadingProgress={
              acct.enriching
                ? acct.enrichmentPercentage != null
                  ? `${acct.enrichmentPercentage}%`
                  : `${acct.enrichSeconds ?? 0}s`
                : undefined
            }
          />
        )}
        <FolioReportsDropdown accountId={accountId!} />
        <FolioToolsDropdown
          onCreateChild={() => acct.setCreateChildOpen(true)}
          onMerge={() => setMergeDialogOpen(true)}
          onArchive={() => setArchiveDialogOpen(true)}
          onUnarchive={acct.handleUnarchive}
          onIndexFiles={acct.handleIndexFiles}
          isArchived={!!acct.detail?.archived}
          isIndexing={acct.indexing}
          hasDetail={!!acct.detail}
        />
      </div>
    ),
  }, accountId);
  const [rolloverDismissed, setRolloverDismissed] = useState(false);
  const [pendingConflictField, setPendingConflictField] = useState<string | null>(null);
  const [editingProductId, setEditingProductId] = useState<number | null>(null);
  const [editProductName, setEditProductName] = useState("");
  const [editProductStatus, setEditProductStatus] = useState("");
  const [statusDropdownProductId, setStatusDropdownProductId] = useState<number | null>(null);

  // Close product status dropdown on outside click
  useEffect(() => {
    if (statusDropdownProductId === null) return;
    const handler = () => setStatusDropdownProductId(null);
    // Delay to avoid closing on the same click that opened it
    const id = setTimeout(() => document.addEventListener("click", handler), 0);
    return () => { clearTimeout(id); document.removeEventListener("click", handler); };
  }, [statusDropdownProductId]);

  // I312: Preset metadata state
  const [metadataValues, setMetadataValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!accountId) return;
    invoke<string>("get_entity_metadata", { entityType: "account", entityId: accountId })
      .then((json) => {
        try { setMetadataValues(JSON.parse(json) ?? {}); } catch { setMetadataValues({}); }
      })
      .catch((err) => {
        console.error("get_entity_metadata (account) failed:", err); // Expected: background data fetch on mount
        setMetadataValues({});
      });
  }, [accountId]);

  // I316: Fetch ancestor accounts for breadcrumb navigation
  const [ancestors, setAncestors] = useState<{ id: string; name: string }[]>([]);
  useEffect(() => {
    if (!accountId) return;
    invoke<{ id: string; name: string }[]>("get_account_ancestors", { accountId })
      .then(setAncestors)
      .catch((err) => {
        console.error("get_account_ancestors failed:", err); // Expected: background data fetch on mount
        setAncestors([]);
      });
  }, [accountId]);

  // I529: Intelligence quality feedback
  const feedback = useIntelligenceFeedback(accountId, "account");

  // Context entries — must be before early returns (React hooks rule)
  const entityCtx = useEntityContextEntries("account", accountId ?? null);
  const detail = acct.detail;
  const intelligence = detail?.intelligence ?? null;
  const events = acct.events;
  const files = acct.files;
  const fieldConflictMap = useMemo(
    () => new Map((detail?.fieldConflicts ?? []).map((item) => [item.field, item])),
    [detail?.fieldConflicts],
  );

  if (acct.loading) return <EditorialLoading />;

  if (acct.error || !detail) {
    return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;
  }

  const handleAcceptConflict = async (field: string) => {
    const conflict = fieldConflictMap.get(field);
    if (!conflict) return;
    setPendingConflictField(field);
    try {
      await invoke("accept_account_field_conflict", {
        accountId: detail.id,
        field: conflict.field,
        suggestedValue: conflict.suggestedValue,
        source: conflict.source,
        signalId: conflict.signalId || null,
      });
      await acct.load();
      toast.success(`${formatTrackedFieldLabel(field)} updated`);
    } catch (err) {
      console.error("accept_account_field_conflict failed:", err);
      toast.error(`Failed to update ${formatTrackedFieldLabel(field)}`);
    } finally {
      setPendingConflictField(null);
    }
  };

  const handleDismissConflict = async (field: string) => {
    const conflict = fieldConflictMap.get(field);
    if (!conflict) return;
    setPendingConflictField(field);
    try {
      await invoke("dismiss_account_field_conflict", {
        accountId: detail.id,
        field: conflict.field,
        signalId: conflict.signalId,
        source: conflict.source,
        suggestedValue: conflict.suggestedValue,
      });
      await acct.silentRefresh();
      toast.success(`${formatTrackedFieldLabel(field)} suggestion dismissed`);
    } catch (err) {
      console.error("dismiss_account_field_conflict failed:", err);
      toast.error(`Failed to dismiss ${formatTrackedFieldLabel(field)} suggestion`);
    } finally {
      setPendingConflictField(null);
    }
  };

  const handleCorrectProduct = async (product: AccountProduct) => {
    try {
      await invoke("correct_account_product", {
        accountId: detail.id,
        productId: product.id,
        name: editProductName,
        status: editProductStatus || null,
        sourceToPenalize: product.source,
      });
      setEditingProductId(null);
      await acct.load();
      toast.success(`Product "${editProductName}" updated`);
    } catch (err) {
      console.error("correct_account_product failed:", err);
      toast.error("Failed to update product");
    }
  };

  const conflictsForStrip = new Map(
    ["lifecycle", "arr", "contract_end", "nps"]
      .flatMap((field) => {
        const c = fieldConflictMap.get(field);
        if (!c) return [];
        const conflict: VitalConflict = {
          source: c.source,
          suggestedValue: formatSuggestedValue(field, c.suggestedValue),
          detectedAt: c.detectedAt,
          pending: pendingConflictField === field,
          onAccept: () => void handleAcceptConflict(field),
          onDismiss: () => void handleDismissConflict(field),
        };
        return [[field, conflict]];
      })
  );

  return (
    <>
      {/* I316: Ancestor breadcrumbs for nested accounts */}
      {ancestors.length > 0 && (
        <nav className={shared.breadcrumbNav}>
          <button
            onClick={() => navigate({ to: "/accounts" })}
            className={shared.breadcrumbButton}
          >
            Accounts
          </button>
          {ancestors.map((anc) => (
            <React.Fragment key={anc.id}>
              <span className={shared.breadcrumbSeparator}>/</span>
              <button
                onClick={() =>
                  navigate({
                    to: "/accounts/$accountId",
                    params: { accountId: anc.id },
                  })
                }
                className={styles.breadcrumbAncestorLink}
              >
                {anc.name}
              </button>
            </React.Fragment>
          ))}
          <span className={shared.breadcrumbSeparator}>/</span>
          <span className={shared.breadcrumbCurrent}>{detail?.name ?? ""}</span>
        </nav>
      )}

      {/* Chapter 1: The Headline (Hero) — no reveal, visible immediately */}
      <section id="headline" className={shared.chapterSection}>
        <AccountHero
          detail={detail}
          intelligence={intelligence}
          editName={acct.editName}
          setEditName={(v) => { acct.setEditName(v); acct.setDirty(true); }}
          editHealth={acct.editHealth}
          setEditHealth={(v) => { acct.setEditHealth(v); acct.setDirty(true); }}
          editLifecycle={acct.editLifecycle}
          setEditLifecycle={(v) => { acct.setEditLifecycle(v); acct.setDirty(true); }}
          onSave={acct.handleSave}
          onSaveField={saveAccountField}
          vitalsSlot={
            detail.accountType !== "internal" ? (
              preset ? (
                <EditableVitalsStrip
                  fields={preset.vitals.account}
                  entityData={detail}
                  metadata={metadataValues}
                  onFieldChange={(key, columnMapping, source, value) => {
                    if (source === "metadata") {
                  setMetadataValues((prev) => {
                    const updated = { ...prev, [key]: value };
                    void saveMetadata(updated);
                    return updated;
                  });
                } else if (source === "column") {
                  const field = columnMapping ?? key;
                  void saveAccountField(field, value);
                }
              }}
                  conflicts={conflictsForStrip}
                />
              ) : (
                <VitalsStrip vitals={buildAccountVitals(detail)} />
              )
            ) : undefined
          }
          provenanceSlot={undefined}
        />
        {/* I312: Preset metadata fields */}
        {preset && preset.metadata.account.length > 0 && (
          <div className={`editorial-reveal ${shared.presetFieldsReveal}`}>
            <PresetFieldsEditor
              fields={preset.metadata.account}
              values={metadataValues}
              onChange={(key, value) => {
                setMetadataValues((prev) => {
                  const updated = { ...prev, [key]: value };
                  void saveMetadata(updated);
                  return updated;
                });
              }}
            />
          </div>
        )}
        {/* Auto-rollover prompt for past renewal dates */}
        {detail.renewalDate && new Date(detail.renewalDate) < new Date() && !rolloverDismissed && (
          <div className={styles.rolloverPrompt}>
            <span>Renewal date has passed — what happened?</span>
            <div className={styles.rolloverActions}>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  acct.setNewEventType("renewal");
                  acct.setNewEventDate(detail.renewalDate!);
                  acct.handleRecordEvent();
                  setRolloverDismissed(true);
                }}
                className={styles.rolloverButton}
              >
                Renewed
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  acct.setNewEventType("churn");
                  acct.setNewEventDate(detail.renewalDate!);
                  acct.handleRecordEvent();
                  setRolloverDismissed(true);
                }}
                className={styles.rolloverButton}
              >
                Churned
              </Button>
              <button
                onClick={() => setRolloverDismissed(true)}
                className={styles.rolloverDismiss}
              >
                Dismiss
              </button>
            </div>
          </div>
        )}
      </section>

      {/* Chapter 2: Outlook — commercial picture immediately after hero */}
      {intelligence && (intelligence.renewalOutlook || intelligence.expansionSignals?.length || intelligence.contractContext) ? (
        <div id="outlook" className={`editorial-reveal ${shared.marginLabelSection}`}>
          <div className={shared.marginLabel}>Outlook</div>
          <div className={shared.marginContent}>
            <ChapterHeading title="Outlook" />
            <AccountOutlook
              intelligence={intelligence}
              onUpdateField={handleUpdateIntelField}
              getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
              onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
            />
          </div>
        </div>
      ) : null}

      <div className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>Products</div>
        <div className={shared.marginContent}>
          <ChapterHeading title="Products & Entitlements" />
          {detail.products && detail.products.length > 0 ? (
            <div className={styles.productList}>
              {detail.products.map((product) => {
                const confidencePct = Math.round(product.confidence * 100);
                const sourceLabel = formatProvenanceSource(product.source);
                const tooltipText = `${sourceLabel ?? "Unknown source"} \u00b7 ${confidencePct}% confidence`;

                const setProductStatus = (nextStatus: string) => {
                  setStatusDropdownProductId(null);
                  void invoke("correct_account_product", {
                    accountId: detail.id,
                    productId: product.id,
                    name: product.name,
                    status: nextStatus,
                    sourceToPenalize: product.source,
                  }).then(() => {
                    void acct.silentRefresh();
                  }).catch((err: unknown) => {
                    console.error("correct_account_product failed:", err);
                    toast.error("Failed to update product status");
                  });
                };

                return (
                  <div key={`${product.id}-${product.name}`} className={styles.productRow}>
                    {/* Left: name (inline editable) + source provenance */}
                    <div
                      className={styles.productInfo}
                      onClick={() => {
                        if (editingProductId !== product.id) {
                          setEditingProductId(product.id);
                          setEditProductName(product.name);
                          setEditProductStatus(product.status);
                        }
                      }}
                    >
                      {editingProductId === product.id ? (
                        <input
                          className={styles.productEditInput}
                          value={editProductName}
                          onChange={(e) => setEditProductName(e.target.value)}
                          onBlur={() => void handleCorrectProduct(product)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") void handleCorrectProduct(product);
                            if (e.key === "Escape") setEditingProductId(null);
                          }}
                          autoFocus
                        />
                      ) : (
                        <div className={styles.productName}>{product.name}</div>
                      )}
                      </div>

                    {/* Center: status badge with dropdown */}
                    {product.status && (
                      <div className={styles.productStatusWrapper}>
                        <button
                          type="button"
                          className={styles.productStatusBadge}
                          data-status={product.status}
                          onClick={() => setStatusDropdownProductId(
                            statusDropdownProductId === product.id ? null : product.id
                          )}
                        >
                          {product.status}
                          <ChevronDown size={8} strokeWidth={2} />
                        </button>
                        {statusDropdownProductId === product.id && (
                          <div className={styles.productStatusDropdown}>
                            {["active", "trial", "churned"].map((s) => (
                              <button
                                key={s}
                                type="button"
                                className={`${styles.productStatusOption} ${s === product.status ? styles.productStatusOptionActive : ""}`}
                                data-status={s}
                                onClick={() => setProductStatus(s)}
                              >
                                {s}
                              </button>
                            ))}
                          </div>
                        )}
                      </div>
                    )}

                    {/* Right: feedback + dismiss */}
                    <div className={styles.productRight}>
                      <span className={styles.productFeedback}>
                        <IntelligenceFeedback
                          value={feedback.getFeedback(`products[${product.id}]`)}
                          onFeedback={(type) => {
                            feedback.submitFeedback(`products[${product.id}]`, type);
                          }}
                        />
                      </span>
                      <button
                        type="button"
                        className={styles.productDismiss}
                        onClick={(e) => {
                          e.stopPropagation();
                          feedback.submitFeedback(`products[${product.id}]`, "negative");
                          // Remove from display by correcting to churned
                          void handleCorrectProduct({
                            ...product,
                            status: "churned",
                          });
                        }}
                        title={`Remove — ${tooltipText}`}
                      >
                        ×
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <p className={styles.sectionEmpty}>No products captured yet.</p>
          )}
        </div>
      </div>

      {/* I393: Portfolio chapter — only for parent accounts */}
      {detail.isParent && detail.children.length > 0 && (
        <section id="portfolio" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
          <ChapterHeading title="Portfolio" />

          {/* Health summary — one-sentence portfolio health statement */}
          {intelligence?.portfolio?.healthSummary && (
            <div className={shared.portfolioHealthSummary}>
              <p className={shared.portfolioHealthSummaryText}>
                {intelligence.portfolio.healthSummary}
              </p>
            </div>
          )}

          {/* Portfolio narrative */}
          {intelligence?.portfolio?.portfolioNarrative && (
            <div className={shared.portfolioNarrative}>
              <p className={shared.portfolioNarrativeText}>
                {intelligence.portfolio.portfolioNarrative}
              </p>
            </div>
          )}

          {/* Hotspots — child accounts needing attention */}
          {intelligence?.portfolio?.hotspots && intelligence.portfolio.hotspots.length > 0 && (
            <div className={shared.portfolioHotspotsSection}>
              <div className={shared.portfolioSectionLabelTerracotta}>
                Needs Attention
              </div>
              {intelligence.portfolio.hotspots.map((hotspot, i) => (
                <div
                  key={hotspot.childId}
                  className={
                    i === intelligence.portfolio!.hotspots.length - 1
                      ? shared.hotspotRow
                      : shared.hotspotRowBorder
                  }
                >
                  <span className={shared.hotspotDot} />
                  <div className={shared.hotspotContent}>
                    <button
                      onClick={() =>
                        navigate({
                          to: "/accounts/$accountId",
                          params: { accountId: hotspot.childId },
                        })
                      }
                      className={styles.hotspotLinkTurmeric}
                    >
                      {hotspot.childName}
                    </button>
                    <p className={shared.hotspotReason}>
                      {hotspot.reason}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Cross-BU patterns — only shown when non-empty */}
          {intelligence?.portfolio?.crossBuPatterns && intelligence.portfolio.crossBuPatterns.length > 0 && (
            <div className={shared.crossPatternsBlock}>
              <div className={shared.portfolioSectionLabelLarkspur}>
                Cross-BU Patterns
              </div>
              {intelligence.portfolio.crossBuPatterns.map((pattern, i) => (
                <p
                  key={i}
                  className={i === 0 ? shared.crossPatternTextFirst : shared.crossPatternTextSubsequent}
                >
                  {pattern}
                </p>
              ))}
            </div>
          )}

          {/* Condensed child list */}
          <div className={shared.childListSection}>
            <div className={shared.portfolioSectionLabelTertiary}>
              Business Units
            </div>
            {detail.children.map((child, i) => (
              <div
                key={child.id}
                className={
                  i === detail.children.length - 1
                    ? shared.childRow
                    : shared.childRowBorder
                }
              >
                <button
                  onClick={() =>
                    navigate({
                      to: "/accounts/$accountId",
                      params: { accountId: child.id },
                    })
                  }
                  className={shared.childNameButton}
                >
                  {child.name}
                  {child.accountType && child.accountType !== "customer" && (
                    <span className={shared.childTypeBadge}>
                      {child.accountType === "partner" ? "Partner" : "Internal"}
                    </span>
                  )}
                </button>
                {/* Health indicator */}
                {child.health && (
                  <span className={`${shared.statusIndicator} ${getHealthColorClass(child.health)}`}>
                    <span className={getHealthDotClass(child.health)} />
                    {child.health === "green"
                      ? "Healthy"
                      : child.health === "red"
                        ? "At Risk"
                        : "Monitor"}
                  </span>
                )}
                {/* ARR if available */}
                {child.arr != null && (
                  <span className={shared.secondaryMetric}>
                    ${formatArr(child.arr)}
                  </span>
                )}
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Chapter 2: State of Play */}
      <div id="state-of-play" className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>State of<br/>Play</div>
        <div className={shared.marginContent}>
          <StateOfPlay
            intelligence={intelligence}
            sectionId=""
            onUpdateField={handleUpdateIntelField}
            getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
            onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
          />
          {detail.technicalFootprint && (() => {
            const tf = detail.technicalFootprint;
            const items: { label: string; value: string }[] = [];
            if (tf.supportTier) items.push({ label: "Support", value: tf.supportTier });
            if (tf.csatScore != null) items.push({ label: "CSAT", value: `${tf.csatScore.toFixed(1)}/5` });
            if (tf.openTickets != null && tf.openTickets > 0) items.push({ label: "Open Tickets", value: String(tf.openTickets) });
            if (tf.usageTier) items.push({ label: "Usage Tier", value: tf.usageTier });
            if (tf.activeUsers != null) items.push({ label: "Active Users", value: tf.activeUsers.toLocaleString() });
            if (tf.adoptionScore != null) items.push({ label: "Adoption", value: `${Math.round(tf.adoptionScore * 100)}%` });
            if (tf.servicesStage) items.push({ label: "Services", value: tf.servicesStage });
            if (items.length === 0) return null;
            return (
              <div className={styles.technicalFootprint}>
                <div className={styles.technicalFootprintLabel}>Technical Footprint</div>
                <div className={styles.technicalFootprintGrid}>
                  {items.map((item) => (
                    <div key={item.label} className={styles.technicalFootprintItem}>
                      <span className={styles.technicalFootprintItemLabel}>{item.label}</span>
                      <span className={styles.technicalFootprintItemValue}>{item.value}</span>
                    </div>
                  ))}
                </div>
                <div className={styles.technicalFootprintSource}>
                  Source: {tf.source} &middot; {formatShortDate(tf.sourcedAt)}
                </div>
              </div>
            );
          })()}
        </div>
      </div>

      {/* Pull quote — dedicated field with fallback to first sentence of executive assessment */}
      {(intelligence?.pullQuote || intelligence?.executiveAssessment) && (() => {
        const quote = intelligence.pullQuote
          || (() => {
            // Fallback: extract first sentence from executiveAssessment
            const text = intelligence.executiveAssessment!;
            const match = text.match(/^(.+?[.!?])(?:\s|\n|$)/);
            return match ? match[1] : text.split("\n\n")[0]?.slice(0, 200);
          })();
        return quote ? (
          <div className={`editorial-reveal-slow ${styles.pullQuote}`}>
            <blockquote className={styles.pullQuoteText}>
              {quote}
            </blockquote>
            <cite className={styles.pullQuoteAttribution}>From the executive assessment</cite>
          </div>
        ) : null;
      })()}

      {/* Chapter 3: Relationship Health */}
      {intelligence?.health && (
        <div id="relationship-health" className={`editorial-reveal ${shared.marginLabelSection}`}>
          <div className={shared.marginLabel}>Relationship<br/>Health</div>
          <div className={shared.marginContent}>
            <ChapterHeading title="Relationship Health" />
            <div className={styles.healthHero}>
              <div className={styles.healthScoreNumber}>
                {Math.round(intelligence.health.score)}
              </div>
              <div className={styles.healthMeta}>
                <div className={
                  intelligence.health.band === "green" ? styles.healthBandGreen
                    : intelligence.health.band === "red" ? styles.healthBandRed
                    : styles.healthBandYellow
                }>
                  {intelligence.health.band === "green" ? "Healthy"
                    : intelligence.health.band === "red" ? "At Risk"
                    : "Monitor"}
                </div>
                {intelligence.health.narrative && (
                  <p className={styles.healthNarrative}>{intelligence.health.narrative}</p>
                )}
                <div className={styles.healthTrendLabel}>
                  {intelligence.health.trend.direction === "improving" && <TrendingUp size={12} strokeWidth={2} />}
                  {intelligence.health.trend.direction === "declining" && <TrendingDown size={12} strokeWidth={2} />}
                  {(intelligence.health.trend.direction === "stable" || intelligence.health.trend.direction === "volatile") && <Minus size={12} strokeWidth={2} />}
                  {intelligence.health.trend.direction}
                  {intelligence.health.trend.timeframe && ` \u00b7 ${intelligence.health.trend.timeframe}`}
                </div>
              </div>
            </div>
            <div className="editorial-reveal-stagger">
              <DimensionBar dimensions={intelligence.health.dimensions} />
            </div>
            {/* I557: Engagement cadence context below dimension bars */}
          </div>
        </div>
      )}

      {/* Chapter 4: The Room */}
      <div id="the-room" className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>The<br/>Room</div>
        <div className={shared.marginContent}>
          <StakeholderGallery
            intelligence={intelligence}
            linkedPeople={detail.linkedPeople}
            accountTeam={detail.accountTeam}
            sectionId=""
            entityId={accountId}
            entityType="account"
            onIntelligenceUpdated={acct.silentRefresh}
            onRemoveTeamMember={acct.handleRemoveTeamMember}
            onChangeTeamRole={acct.changeTeamMemberRole}
            onAddTeamMember={acct.addTeamMemberDirect}
            onCreateTeamMember={acct.createTeamMemberDirect}
            teamSearchQuery={acct.teamSearchQuery}
            onTeamSearchQueryChange={acct.setTeamSearchQuery}
            teamSearchResults={acct.teamSearchResults}
          />
        </div>
      </div>

      {/* Chapter 5: Watch List */}
      <div id="watch-list" className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>Watch<br/>List</div>
        <div className={shared.marginContent}>
          <WatchList
            intelligence={intelligence}
            sectionId=""
            onUpdateField={handleUpdateIntelField}
            getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
            onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
            bottomSection={
              <WatchListPrograms
                programs={acct.programs}
                onProgramUpdate={acct.handleProgramUpdate}
                onProgramDelete={acct.handleProgramDelete}
                onAddProgram={acct.handleAddProgram}
              />
            }
          />
        </div>
      </div>

      {/* Chapter: Value & Commitments (I557) */}
      {intelligence && (intelligence.valueDelivered?.length || intelligence.successMetrics?.length || intelligence.openCommitments?.length) ? (
        <div id="value-commitments" className={`editorial-reveal ${shared.marginLabelSection}`}>
          <div className={shared.marginLabel}>Value &<br/>Commitments</div>
          <div className={shared.marginContent}>
            <ChapterHeading title="Value & Commitments" />
            <ValueCommitments
              intelligence={intelligence}
              onUpdateField={handleUpdateIntelField}
              getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
              onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
            />
          </div>
        </div>
      ) : null}

      {/* Chapter: Competitive & Strategic Landscape (I557) */}
      {intelligence && (intelligence.strategicPriorities?.length || intelligence.competitiveContext?.length || intelligence.organizationalChanges?.length || intelligence.blockers?.length) ? (
        <div id="strategic-landscape" className={`editorial-reveal ${shared.marginLabelSection}`}>
          <div className={shared.marginLabel}>Competitive &<br/>Strategic</div>
          <div className={shared.marginContent}>
            <ChapterHeading title="Competitive & Strategic" />
            <StrategicLandscape
              intelligence={intelligence}
              onUpdateField={handleUpdateIntelField}
              getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
              onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
            />
          </div>
        </div>
      ) : null}

      {/* Chapter 6: The Record */}
      <div id="the-record" className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>The<br/>Record</div>
        <div className={shared.marginContent}>
          <UnifiedTimeline
            data={{
              ...detail,
              accountEvents: events,
              lifecycleChanges: detail.lifecycleChanges,
              autoCompletedMilestones: detail.autoCompletedMilestones,
              contextEntries: entityCtx.entries,
            }}
            sectionId=""
            actionSlot={<AddToRecord onAdd={(title, content) => entityCtx.createEntry(title, content)} />}
          />
        </div>
      </div>

      {/* Chapter 7: The Work */}
      <div id="the-work" className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>The<br/>Work</div>
        <div className={shared.marginContent}>
          <TheWork
            data={{ ...detail, accountId: detail.id }}
            sectionId=""
            addingAction={acct.addingAction}
            setAddingAction={acct.setAddingAction}
            newActionTitle={acct.newActionTitle}
            setNewActionTitle={acct.setNewActionTitle}
            creatingAction={acct.creatingAction}
            onCreateAction={acct.handleCreateAction}
            onRefresh={acct.silentRefresh}
          />
        </div>
      </div>

      {/* Chapter 8: Reports */}
      <div id="reports" className={`editorial-reveal ${shared.marginLabelSection}`}>
        <div className={shared.marginLabel}>Reports</div>
        <div className={shared.marginContent}>
          <ChapterHeading title="Reports" />
          <div className={styles.reportsChapter}>
            {getAccountReports(preset?.id).map((item) => {
              const handleClick = () => {
                if (item.reportType === "risk_briefing") {
                  navigate({ to: "/accounts/$accountId/reports/risk_briefing", params: { accountId: accountId! } } as any);
                } else if (item.reportType === "account_health") {
                  navigate({ to: "/accounts/$accountId/reports/account_health", params: { accountId: accountId! } } as any);
                } else if (item.reportType === "ebr_qbr") {
                  navigate({ to: "/accounts/$accountId/reports/ebr_qbr", params: { accountId: accountId! } } as any);
                } else {
                  navigate({ to: "/accounts/$accountId/reports/$reportType", params: { accountId: accountId!, reportType: item.reportType } });
                }
              };
              return (
                <button
                  key={item.label}
                  onClick={handleClick}
                  className={styles.reportRow}
                >
                  <FileText size={16} strokeWidth={1.5} className={styles.reportIcon} />
                  <span className={styles.reportName}>{item.label}</span>
                  <span className={styles.reportAction}>View</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      {/* Files section — inline when files exist */}
      {files.length > 0 && (
        <div className={shared.marginLabelSection}>
          <div className={shared.marginLabel}>Files</div>
          <div className={shared.marginContent}>
            <FileListSection files={files} />
          </div>
        </div>
      )}

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={intelligence?.enrichedAt} />
      </div>

      {/* ─── Archive Confirmation ─── */}
      <AlertDialog open={archiveDialogOpen} onOpenChange={setArchiveDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Archive Account</AlertDialogTitle>
            <AlertDialogDescription>
              This will hide {detail.name} from active views.
              You can unarchive it later.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={acct.handleArchive}>Archive</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* ─── Child Account Creation ─── */}
      <Dialog open={acct.createChildOpen} onOpenChange={acct.setCreateChildOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {detail.accountType === "internal" ? "Create Team" : "Create Business Unit"}
            </DialogTitle>
            <DialogDescription>
              Create a new {detail.accountType === "internal" ? "team" : "business unit"} under {detail.name}.
            </DialogDescription>
          </DialogHeader>
          <div className={shared.dialogForm}>
            <Input
              value={acct.childName}
              onChange={(e) => acct.setChildName(e.target.value)}
              placeholder="Name"
            />
            <Input
              value={acct.childDescription}
              onChange={(e) => acct.setChildDescription(e.target.value)}
              placeholder="Description (optional)"
            />
            <div className={shared.dialogActions}>
              <Button
                variant="ghost"
                onClick={() => acct.setCreateChildOpen(false)}
                className={shared.dialogButton}
              >
                Cancel
              </Button>
              <Button
                onClick={acct.handleCreateChild}
                disabled={acct.creatingChild || !acct.childName.trim()}
                className={shared.dialogButton}
              >
                {acct.creatingChild ? "Creating…" : "Create"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <AccountMergeDialog
        open={mergeDialogOpen}
        onOpenChange={setMergeDialogOpen}
        sourceAccountId={accountId!}
        sourceAccountName={detail.name}
        onMerged={() => navigate({ to: "/accounts" })}
      />
    </>
  );
}
