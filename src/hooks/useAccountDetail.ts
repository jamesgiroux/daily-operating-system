/**
 * useAccountDetail — Orchestrator hook for the account detail page.
 *
 * Composes focused sub-hooks internally:
 *   - useAccountFields (field editing, save/cancel)
 *   - useTeamManagement (search, add, remove, inline create)
 *
 * The public return type is unchanged — page components destructure
 * one flat object, sub-hooks are an internal concern.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
  HealthSparklinePoint,
  SentimentJournalEntry,
  SentimentValue,
  StrategicProgram,
} from "@/types";
import type { RolePreset } from "@/types/preset";
import { useAccountFields } from "./useAccountFields";
import { useAccountWorkData } from "./useAccountWorkData";
import { useEnrichmentProgress } from "./useEnrichmentProgress";
import { useTeamManagement } from "./useTeamManagement";
import { useTauriEvent } from "./useTauriEvent";

interface BackgroundWorkStatusEvent {
  phase: "started" | "completed" | "failed";
  message: string;
  entityId?: string;
  entityType?: string;
  stage?: string;
  error?: string;
}

/** Band steps used to score divergence magnitude. */
const SENTIMENT_RANK: Record<SentimentValue, number> = {
  strong: 4,
  on_track: 3,
  concerning: 2,
  at_risk: 1,
  critical: 0,
};

const COMPUTED_BAND_RANK: Record<string, number> = {
  green: 4,
  yellow: 2,
  red: 0,
};

const MILLIS_PER_DAY = 24 * 60 * 60 * 1000;
const SENTIMENT_STALE_DAYS = 30;

/** CS preset — default labels. Other presets would override via hook composition later. */
export const DEFAULT_SENTIMENT_LABELS: Record<SentimentValue, string> = {
  strong: "Strong",
  on_track: "On Track",
  concerning: "Concerning",
  at_risk: "At Risk",
  critical: "Critical",
};

export function buildPresetSentimentLabels(
  _preset: RolePreset | null | undefined,
): Record<SentimentValue, string> {
  return DEFAULT_SENTIMENT_LABELS;
}

export interface SentimentDivergence {
  severity: "minor" | "major";
  computedBand: string;
  delta: number;
}

export interface SentimentView {
  current: SentimentValue | null;
  note: string | null;
  setAt: string | null;
  history: SentimentJournalEntry[];
  sparkline: HealthSparklinePoint[];
  divergence: SentimentDivergence | null;
  isStale: boolean;
  presetLabels: Record<SentimentValue, string>;
}

function buildSentimentView(detail: AccountDetail | null): SentimentView {
  const current = (detail?.userHealthSentiment ?? null) as SentimentValue | null;
  const setAt = detail?.sentimentSetAt ?? null;
  const note = detail?.sentimentNote ?? null;
  const history = detail?.sentimentHistory ?? [];
  const sparkline = detail?.healthSparkline ?? [];

  // Divergence: compare sentiment rank against current computed band rank.
  let divergence: SentimentDivergence | null = null;
  const computedBand = (detail?.health ?? "").toLowerCase();
  if (current && COMPUTED_BAND_RANK[computedBand] !== undefined) {
    const sRank = SENTIMENT_RANK[current];
    const cRank = COMPUTED_BAND_RANK[computedBand];
    const delta = Math.abs(sRank - cRank);
    if (delta >= 2) {
      divergence = {
        severity: delta >= 3 ? "major" : "minor",
        computedBand,
        delta,
      };
    }
  }

  let isStale = false;
  if (setAt) {
    const ageDays = (Date.now() - new Date(setAt).getTime()) / MILLIS_PER_DAY;
    isStale = ageDays >= SENTIMENT_STALE_DAYS;
  }

  return {
    current,
    note,
    setAt,
    history,
    sparkline,
    divergence,
    isStale,
    presetLabels: DEFAULT_SENTIMENT_LABELS,
  };
}

export function useAccountDetail(accountId: string | undefined) {
  const navigate = useNavigate();
  const [detail, setDetail] = useState<AccountDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Enrichment
  const [enriching, setEnriching] = useState(false);
  const [enrichSeconds, setEnrichSeconds] = useState(0);

  // Inline action creation
  const [addingAction, setAddingAction] = useState(false);
  const [newActionTitle, setNewActionTitle] = useState("");
  const [creatingAction, setCreatingAction] = useState(false);

  // Child account creation
  const [createChildOpen, setCreateChildOpen] = useState(false);
  const [childName, setChildName] = useState("");
  const [childDescription, setChildDescription] = useState("");
  const [childOwnerId, setChildOwnerId] = useState("");
  const [creatingChild, setCreatingChild] = useState(false);

  // Content index state
  const [files, setFiles] = useState<ContentFile[]>([]);
  const [indexing, setIndexing] = useState(false);
  const [newFileCount, setNewFileCount] = useState(0);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [indexFeedback, setIndexFeedback] = useState<string | null>(null);

  // Strategic programs
  const [programs, setPrograms] = useState<StrategicProgram[]>([]);
  const programsSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Lifecycle events
  const [events, setEvents] = useState<AccountEvent[]>([]);
  const [showEventForm, setShowEventForm] = useState(false);
  const [newEventType, setNewEventType] = useState("renewal");
  const [newEventDate, setNewEventDate] = useState("");
  const [newArrImpact, setNewArrImpact] = useState("");
  const [newEventNotes, setNewEventNotes] = useState("");

  // Evidence collapse
  const [recentMeetingsExpanded, setRecentMeetingsExpanded] = useState(false);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (programsSaveTimer.current) clearTimeout(programsSaveTimer.current);
    };
  }, []);

  const intelligence = detail?.intelligence ?? null;

  // ─── Core data loading ────────────────────────────────────────────────

  const fetchDetail = useCallback(async (showLoading: boolean) => {
    if (!accountId) return;
    try {
      if (showLoading) setLoading(true);
      setError(null);
      const result = await invoke<AccountDetail>("get_account_detail", {
        accountId,
      });
      setDetail(result);
      setPrograms(result.strategicPrograms);
      // Load content files
      try {
        const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
          entityId: accountId,
        });
        setFiles(contentFiles);
      } catch {
        // Non-critical
      }
      // Load lifecycle events
      try {
        const accountEvents = await invoke<AccountEvent[]>("get_account_events", {
          accountId,
        });
        setEvents(accountEvents);
      } catch {
        // Non-critical
      }
    } catch (e) {
      setError(String(e));
    } finally {
      if (showLoading) setLoading(false);
    }
  }, [accountId]);

  /** Full load with loading spinner (initial + navigation). */
  const load = useCallback(() => fetchDetail(true), [fetchDetail]);

  /** Silent refresh — updates data without flipping loading state or resetting scroll. */
  const silentRefresh = useCallback(() => fetchDetail(false), [fetchDetail]);

  useEffect(() => {
    load();
  }, [load]);

  // ─── Composed sub-hooks ───────────────────────────────────────────────

  // expose setDetail to sub-hooks so commands that
  // return a fresh AccountDetail can apply it directly (no follow-up
  // silentRefresh needed, avoiding SQLite WAL reader-snapshot lag).
  const applyDetail = useCallback((d: AccountDetail) => {
    setDetail(d);
    setPrograms(d.strategicPrograms);
  }, []);

  const fields = useAccountFields(detail, load, setError, applyDetail);
  const team = useTeamManagement(accountId, silentRefresh, applyDetail);

  // Progressive enrichment — refresh data as each dimension completes
  const enrichmentProgress = useEnrichmentProgress(accountId, silentRefresh);
  const enrichmentPercentage = enrichmentProgress
    ? Math.round((enrichmentProgress.completed / enrichmentProgress.total) * 100)
    : null;

  // ─── Event listeners ──────────────────────────────────────────────────

  const handleIntelligenceUpdated = useCallback(
    (payload: { entityId: string }) => {
      if (accountId && payload.entityId === accountId) {
        load();
      }
    },
    [accountId, load],
  );
  useTauriEvent("intelligence-updated", handleIntelligenceUpdated);

  const handleBackgroundWorkStatus = useCallback(
    (payload: BackgroundWorkStatusEvent) => {
      if (payload.phase !== "failed") return;
      if (!accountId || payload.entityId !== accountId) return;
      setEnriching(false);
      setError(payload.error ?? payload.message);
    },
    [accountId],
  );
  useTauriEvent("background-work-status", handleBackgroundWorkStatus);

  const handleContentChanged = useCallback(
    (payload: { entityIds: string[]; count: number }) => {
      if (accountId && payload.entityIds.includes(accountId)) {
        setNewFileCount(payload.count);
        setBannerDismissed(false);
      }
    },
    [accountId],
  );
  useTauriEvent("content-changed", handleContentChanged);

  // ─── Enrichment timer ─────────────────────────────────────────────────

  useEffect(() => {
    if (!enriching) {
      setEnrichSeconds(0);
      return;
    }
    const interval = setInterval(() => setEnrichSeconds((s) => s + 1), 1000);
    return () => clearInterval(interval);
  }, [enriching]);

  // ─── Handlers ─────────────────────────────────────────────────────────

  async function handleIndexFiles() {
    if (!detail) return;
    setIndexing(true);
    setIndexFeedback(null);
    try {
      const updated = await invoke<ContentFile[]>("index_entity_files", {
        entityType: "account",
        entityId: detail.id,
      });
      const diff = updated.length - files.length;
      setFiles(updated);
      setNewFileCount(0);
      setBannerDismissed(true);
      if (diff > 0) {
        setIndexFeedback(`${diff} new file${diff !== 1 ? "s" : ""} found`);
      } else {
        setIndexFeedback("Up to date");
      }
      setTimeout(() => setIndexFeedback(null), 3000);
    } catch (e) {
      setError(String(e));
    } finally {
      setIndexing(false);
    }
  }

  async function handleEnrich() {
    if (!detail) return;
    setEnriching(true);
    const refreshPromise = invoke("enrich_account", { accountId: detail.id });
    try {
      const completed = await Promise.race([
        refreshPromise.then(() => true),
        new Promise<boolean>((resolve) => {
          setTimeout(() => resolve(false), 90_000);
        }),
      ]);

      if (completed) {
        await load();
      } else {
        toast("Refresh is still running in the background. This page will update when it finishes.", {
          duration: 8000,
          id: "account-refresh-background",
        });
        void refreshPromise
          .then(() => load())
          .catch((e) => {
            const message = String(e);
            setError(message);
            toast.error(message, { id: "account-refresh-error", duration: 8000 });
          });
      }
    } catch (e) {
      const message = String(e);
      setError(message);
      toast.error(message, { id: "account-refresh-error", duration: 8000 });
    } finally {
      setEnriching(false);
    }
  }

  async function handleCreateChild() {
    if (!detail || !childName.trim()) return;
    setCreatingChild(true);
    try {
      const result = await invoke<{ id: string }>("create_child_account", {
        parentId: detail.id,
        name: childName.trim(),
        description: childDescription.trim() || null,
        ownerPersonId: childOwnerId || null,
      });
      setCreateChildOpen(false);
      setChildName("");
      setChildDescription("");
      setChildOwnerId("");
      await load();
      navigate({ to: "/accounts/$accountId", params: { accountId: result.id } });
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingChild(false);
    }
  }

  // Debounced save for strategic programs
  const savePrograms = useCallback(
    async (updated: StrategicProgram[]) => {
      if (!detail) return;
      if (programsSaveTimer.current) clearTimeout(programsSaveTimer.current);
      programsSaveTimer.current = setTimeout(async () => {
        try {
          await invoke("update_account_programs", {
            accountId: detail.id,
            programsJson: JSON.stringify(updated),
          });
        } catch (e) {
          console.error("Failed to save programs:", e);
          toast.error("Failed to save programs");
        }
      }, 400);
    },
    [detail],
  );

  function handleProgramUpdate(index: number, updated: StrategicProgram) {
    const next = [...programs];
    next[index] = updated;
    setPrograms(next);
    savePrograms(next);
  }

  function handleProgramDelete(index: number) {
    const next = programs.filter((_, i) => i !== index);
    setPrograms(next);
    savePrograms(next);
  }

  function handleAddProgram() {
    const next = [...programs, { name: "", status: "Active", notes: "" }];
    setPrograms(next);
  }

  async function handleRecordEvent() {
    if (!detail || !newEventDate) return;
    try {
      await invoke("record_account_event", {
        accountId: detail.id,
        eventType: newEventType,
        eventDate: newEventDate,
        arrImpact: newArrImpact ? parseFloat(newArrImpact) : null,
        notes: newEventNotes || null,
      });
      setShowEventForm(false);
      setNewEventType("renewal");
      setNewEventDate("");
      setNewArrImpact("");
      setNewEventNotes("");
      // Reload full detail so archived state updates (e.g. churn auto-archives)
      await load();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleArchive() {
    if (!detail) return;
    try {
      await invoke("archive_account", { id: detail.id, archived: true });
      navigate({ to: "/accounts" });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnarchive() {
    if (!detail) return;
    try {
      await invoke("archive_account", { id: detail.id, archived: false });
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  // ─── DOS Work-tab Phase 3: Commitments / Suggestions / Recently landed ──
  // Action-table-backed surfaces, id-based dispatch. Replaces the previous
  // index-based IntelligenceJson handlers (Phase 2 made Commitments and
  // Suggestions read from actions directly; the old `mark_commitment_done`
  // / `dismiss_intelligence_item` / `track_recommendation` /
  // `dismiss_recommendation` commands are no longer called from the Work
  // tab UI).
  const work = useAccountWorkData(accountId);

  async function handleCreateAction() {
    if (!detail || !newActionTitle.trim()) return;
    setCreatingAction(true);
    try {
      await invoke("create_action", {
        request: { title: newActionTitle.trim(), accountId: detail.id },
      });
      setNewActionTitle("");
      setAddingAction(false);
      await silentRefresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingAction(false);
    }
  }

  // ─── Sentiment journal + divergence ───────────────────────────

  const sentiment = buildSentimentView(detail);

  async function setUserHealthSentiment(value: SentimentValue, note?: string) {
    if (!accountId) return;
    // Command returns the updated AccountDetail assembled on the
    // writer connection — apply it directly. Avoids the SQLite WAL reader
    // snapshot lag that made a follow-up silentRefresh() show stale data
    // until a manual reload.
    const result = await invoke<AccountDetail>("set_user_health_sentiment", {
      accountId,
      sentiment: value,
      note: note?.trim() ? note.trim() : null,
    });
    setDetail(result);
    setPrograms(result.strategicPrograms);
  }

  /**
   * "Add more detail" — update the note on the existing journal
   * entry rather than creating a new one. Backs the SentimentHero
   * `onUpdateNote` prop. Falls back to a fresh insert when no journal row
   * exists for the current sentiment value yet.
   */
  async function updateSentimentNote(note: string) {
    if (!accountId) return;
    const cleaned = note.trim().length > 0 ? note.trim() : null;
    const result = await invoke<AccountDetail>("update_latest_sentiment_note", {
      accountId,
      note: cleaned,
    });
    setDetail(result);
    setPrograms(result.strategicPrograms);
  }

  /** Re-stamp sentiment_set_at so the "Still accurate?" prompt resets for 30 days. */
  async function acknowledgeSentimentStale() {
    if (!accountId || !detail?.userHealthSentiment) return;
    const result = await invoke<AccountDetail>("set_user_health_sentiment", {
      accountId,
      sentiment: detail.userHealthSentiment,
      note: null,
    });
    setDetail(result);
    setPrograms(result.strategicPrograms);
  }

  // ─── risk briefing status + retry ──────────────

  const riskBriefingJob = detail?.riskBriefingJob ?? null;

  /**
   * Kick off a new risk-briefing generation attempt. Safe to call from a
   * UI button — backend coalesces into an already-running job and the
   * status transitions become visible on the next `get_account_detail`
   * (we optimistically refresh after a short delay to pick up the
   * enqueued → running transition).
   */
  async function retryRiskBriefing() {
    if (!accountId) return;
    try {
      await invoke("retry_risk_briefing", { accountId });
      // Pick up the fresh 'enqueued' row. We can't use the return-value refresh pattern
      // here because retry_risk_briefing returns void — the status is
      // persisted on a separate writer call and we already wait for it.
      await silentRefresh();
    } catch (e) {
      setError(String(e));
    }
  }

  // ─── Flat public API ──────────────────────────────────────────────────

  // Glean leading-signal enrichment bundle for Health & Outlook tab.
  // Namespaced under `gleanSignals` per v121-foundation peer coordination
  // (does not collide with the health sentiment `sentiment.*` namespace).
  const gleanSignals = detail?.gleanSignals ?? null;

  return {
    // Core data
    detail,
    loading,
    error,
    intelligence,
    gleanSignals,
    files,
    events,
    programs,

    // Field editing (from useAccountFields)
    ...fields,

    // Enrichment
    enriching,
    enrichSeconds,
    enrichmentProgress,
    enrichmentPercentage,

    // Action creation
    addingAction, setAddingAction,
    newActionTitle, setNewActionTitle,
    creatingAction,

    // Child account creation
    createChildOpen, setCreateChildOpen,
    childName, setChildName,
    childDescription, setChildDescription,
    childOwnerId, setChildOwnerId,
    creatingChild,

    // File indexing
    indexing,
    newFileCount,
    bannerDismissed, setBannerDismissed,
    indexFeedback,

    // Lifecycle events
    showEventForm, setShowEventForm,
    newEventType, setNewEventType,
    newEventDate, setNewEventDate,
    newArrImpact, setNewArrImpact,
    newEventNotes, setNewEventNotes,

    // Team management (from useTeamManagement)
    ...team,

    // Evidence collapse
    recentMeetingsExpanded, setRecentMeetingsExpanded,

    // Handlers
    load,
    silentRefresh,
    handleSave: fields.handleSave,
    saveField: fields.saveField,
    handleCancelEdit: fields.handleCancelEdit,
    handleIndexFiles,
    handleEnrich,
    handleCreateChild,
    handleProgramUpdate,
    handleProgramDelete,
    handleAddProgram,
    handleRecordEvent,
    handleArchive,
    handleUnarchive,
    handleAddExistingTeamMember: team.handleAddExistingTeamMember,
    handleRemoveTeamMember: team.handleRemoveTeamMember,
    handleCreateInlineTeamMember: team.handleCreateInlineTeamMember,
    handleImportNoteCreateAndAdd: team.handleImportNoteCreateAndAdd,
    handleCreateAction,

    // DOS Work-tab Phase 3: Action-table-backed Work surfaces
    // `work.commitments`, `work.suggestions`, `work.recentlyLanded`, plus
    // id-based handlers + in-flight Sets. See useAccountWorkData.
    work,

    // sentiment journal
    sentiment,
    setUserHealthSentiment,
    acknowledgeSentimentStale,
    updateSentimentNote,

    // risk briefing status + retry
    riskBriefingJob,
    retryRiskBriefing,
  };
}
