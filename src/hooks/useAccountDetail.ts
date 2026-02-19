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
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "@tanstack/react-router";
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
  StrategicProgram,
} from "@/types";
import { useAccountFields } from "./useAccountFields";
import { useTeamManagement } from "./useTeamManagement";

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

  const fields = useAccountFields(detail, load, setError);
  const team = useTeamManagement(accountId, load);

  // ─── Event listeners ──────────────────────────────────────────────────

  useEffect(() => {
    const unlisten = listen<{ entityId: string }>(
      "intelligence-updated",
      (event) => {
        if (accountId && event.payload.entityId === accountId) {
          load();
        }
      },
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [accountId, load]);

  useEffect(() => {
    const unlisten = listen<{ entityIds: string[]; count: number }>(
      "content-changed",
      (event) => {
        if (accountId && event.payload.entityIds.includes(accountId)) {
          setNewFileCount(event.payload.count);
          setBannerDismissed(false);
        }
      },
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [accountId]);

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
    try {
      await invoke("enrich_account", { accountId: detail.id });
      await load();
    } catch (e) {
      setError(String(e));
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

  async function handleCreateAction() {
    if (!detail || !newActionTitle.trim()) return;
    setCreatingAction(true);
    try {
      await invoke("create_action", {
        request: { title: newActionTitle.trim(), accountId: detail.id },
      });
      setNewActionTitle("");
      setAddingAction(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingAction(false);
    }
  }

  // ─── Flat public API ──────────────────────────────────────────────────

  return {
    // Core data
    detail,
    loading,
    error,
    intelligence,
    files,
    events,
    programs,

    // Field editing (from useAccountFields)
    ...fields,

    // Enrichment
    enriching,
    enrichSeconds,

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
  };
}
