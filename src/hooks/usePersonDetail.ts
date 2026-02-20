/**
 * usePersonDetail — Orchestrator hook for the person detail editorial page.
 * Mirrors useAccountDetail/useProjectDetail pattern: load, field editing,
 * enrichment, merge, delete, entity link/unlink.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "@tanstack/react-router";
import type { Person, PersonDetail, DuplicateCandidate, ContentFile } from "@/types";

export function usePersonDetail(personId: string | undefined) {
  const navigate = useNavigate();
  const [detail, setDetail] = useState<PersonDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Enrichment
  const [enriching, setEnriching] = useState(false);
  const [enrichSeconds, setEnrichSeconds] = useState(0);
  const enrichTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Editable fields
  const [editName, setEditName] = useState("");
  const [editRole, setEditRole] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  // Merge flow
  const [mergeDialogOpen, setMergeDialogOpen] = useState(false);
  const [mergeTarget, setMergeTarget] = useState<Person | null>(null);
  const [mergeConfirmOpen, setMergeConfirmOpen] = useState(false);
  const [mergeSearchQuery, setMergeSearchQuery] = useState("");
  const [mergeSearchResults, setMergeSearchResults] = useState<Person[]>([]);
  const [merging, setMerging] = useState(false);

  // Delete
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);

  // Duplicates
  const [duplicateCandidates, setDuplicateCandidates] = useState<DuplicateCandidate[]>([]);

  // Inline action creation
  const [addingAction, setAddingAction] = useState(false);
  const [newActionTitle, setNewActionTitle] = useState("");
  const [creatingAction, setCreatingAction] = useState(false);

  // Files
  const [files, setFiles] = useState<ContentFile[]>([]);
  const [indexing, setIndexing] = useState(false);
  const [indexFeedback, setIndexFeedback] = useState<string | null>(null);

  // ─── Core data loading ────────────────────────────────────────────────

  const fetchDetail = useCallback(async (showLoading: boolean) => {
    if (!personId) return;
    try {
      if (showLoading) setLoading(true);
      setError(null);
      const result = await invoke<PersonDetail>("get_person_detail", { personId });
      setDetail(result);
      if (showLoading) {
        setEditName(result.name);
        setEditRole(result.role ?? "");
        setEditNotes(result.notes ?? "");
        setDirty(false);
      }

      // Load files
      try {
        const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
          entityType: "person",
          entityId: personId,
        });
        setFiles(contentFiles);
      } catch {
        setFiles([]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      if (showLoading) setLoading(false);
    }
  }, [personId]);

  const load = useCallback(() => fetchDetail(true), [fetchDetail]);
  const silentRefresh = useCallback(() => fetchDetail(false), [fetchDetail]);

  useEffect(() => {
    load();
  }, [load]);

  // ─── Duplicate candidates ──────────────────────────────────────────────

  const loadDuplicateCandidates = useCallback(async () => {
    if (!personId) return;
    try {
      const candidates = await invoke<DuplicateCandidate[]>(
        "get_duplicate_people_for_person",
        { personId },
      );
      setDuplicateCandidates(candidates);
    } catch {
      setDuplicateCandidates([]);
    }
  }, [personId]);

  useEffect(() => {
    loadDuplicateCandidates();
  }, [loadDuplicateCandidates]);

  // ─── Event listeners ──────────────────────────────────────────────────

  useEffect(() => {
    if (!personId) return;
    const unlisten = listen("intelligence-updated", (event) => {
      const payload = event.payload as { entity_type?: string; entity_id?: string };
      if (payload.entity_type === "person" && payload.entity_id === personId) {
        load();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [personId, load]);

  // ─── Merge search (debounced) ─────────────────────────────────────────

  useEffect(() => {
    if (!mergeSearchQuery || mergeSearchQuery.length < 2) {
      setMergeSearchResults([]);
      return;
    }
    const timer = setTimeout(async () => {
      try {
        const results = await invoke<Person[]>("search_people", {
          query: mergeSearchQuery,
        });
        setMergeSearchResults(results.filter((p) => p.id !== personId));
      } catch {
        setMergeSearchResults([]);
      }
    }, 200);
    return () => clearTimeout(timer);
  }, [mergeSearchQuery, personId]);

  // ─── Field editing ────────────────────────────────────────────────────

  async function saveField(field: string, value: string) {
    if (!detail) return;
    try {
      setSaving(true);
      await invoke("update_person", {
        personId: detail.id,
        field,
        value,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleSave() {
    if (!detail) return;
    const updates: [string, string][] = [];
    if (editName !== detail.name) updates.push(["name", editName]);
    if (editRole !== (detail.role ?? "")) updates.push(["role", editRole]);
    if (editNotes !== (detail.notes ?? "")) updates.push(["notes", editNotes]);

    for (const [field, value] of updates) {
      await saveField(field, value);
    }
    setDirty(false);
    await load();
  }

  function handleCancelEdit() {
    if (!detail) return;
    setEditName(detail.name);
    setEditRole(detail.role ?? "");
    setEditNotes(detail.notes ?? "");
    setDirty(false);
  }

  // ─── Enrichment ───────────────────────────────────────────────────────

  async function handleEnrich() {
    if (!detail) return;
    setEnriching(true);
    setEnrichSeconds(0);
    enrichTimerRef.current = setInterval(() => {
      setEnrichSeconds((s) => s + 1);
    }, 1000);
    try {
      await invoke("enrich_person", { personId: detail.id });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setEnriching(false);
      if (enrichTimerRef.current) clearInterval(enrichTimerRef.current);
    }
  }

  // ─── Entity linking ───────────────────────────────────────────────────

  async function handleLinkEntity(entityId: string) {
    if (!detail) return;
    try {
      await invoke("link_person_entity", {
        personId: detail.id,
        entityId,
        relationshipType: "associated",
      });
      // No reload — PersonNetwork manages optimistic local state
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnlinkEntity(entityId: string) {
    if (!detail) return;
    try {
      await invoke("unlink_person_entity", {
        personId: detail.id,
        entityId,
      });
      // No reload — PersonNetwork manages optimistic local state
    } catch (e) {
      setError(String(e));
    }
  }

  // ─── Merge ────────────────────────────────────────────────────────────

  function openMergeDialog() {
    setMergeSearchQuery("");
    setMergeSearchResults([]);
    setMergeTarget(null);
    setMergeDialogOpen(true);
  }

  async function handleMerge() {
    if (!detail || !mergeTarget) return;
    try {
      setMerging(true);
      const keepId = await invoke<string>("merge_people", {
        keepId: mergeTarget.id,
        removeId: detail.id,
      });
      setMergeConfirmOpen(false);
      setMergeDialogOpen(false);
      navigate({ to: "/people/$personId", params: { personId: keepId } });
    } catch (e) {
      setError(String(e));
      setMerging(false);
    }
  }

  async function handleOpenSuggestedMerge(candidate: DuplicateCandidate) {
    if (!detail) return;
    const targetId =
      candidate.person1Id === detail.id ? candidate.person2Id : candidate.person1Id;
    try {
      const suggested = await invoke<PersonDetail>("get_person_detail", {
        personId: targetId,
      });
      setMergeTarget(suggested);
      setMergeConfirmOpen(true);
    } catch (e) {
      setError(String(e));
    }
  }

  // ─── Delete ───────────────────────────────────────────────────────────

  async function handleDelete() {
    if (!detail) return;
    try {
      setMerging(true);
      await invoke("delete_person", { personId: detail.id });
      setDeleteConfirmOpen(false);
      navigate({ to: "/people" });
    } catch (e) {
      setError(String(e));
      setMerging(false);
    }
  }

  // ─── Archive ──────────────────────────────────────────────────────────

  async function handleArchive() {
    if (!detail) return;
    try {
      await invoke("archive_person", { id: detail.id, archived: true });
      navigate({ to: "/people" });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnarchive() {
    if (!detail) return;
    try {
      await invoke("archive_person", { id: detail.id, archived: false });
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  // ─── File indexing ───────────────────────────────────────────────────

  async function handleIndexFiles() {
    if (!detail) return;
    try {
      setIndexing(true);
      setIndexFeedback(null);
      const result = await invoke<string>("index_entity_files", {
        entityType: "person",
        entityId: detail.id,
      });
      setIndexFeedback(result);
      // Reload files
      const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
        entityType: "person",
        entityId: detail.id,
      });
      setFiles(contentFiles);
    } catch (e) {
      setIndexFeedback(String(e));
    } finally {
      setIndexing(false);
    }
  }

  // ─── Action creation ────────────────────────────────────────────────

  async function handleCreateAction() {
    if (!detail || !newActionTitle.trim()) return;
    setCreatingAction(true);
    try {
      await invoke("create_action", {
        request: { title: newActionTitle.trim(), personId: detail.id },
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

  // ─── Derived ──────────────────────────────────────────────────────────

  const intelligence = detail?.intelligence ?? null;

  return {
    // Core data
    detail,
    intelligence,
    loading,
    error,
    load,
    silentRefresh,

    // Field editing
    editName, setEditName,
    editRole, setEditRole,
    editNotes, setEditNotes,
    dirty, setDirty,
    saving,
    handleSave,
    handleCancelEdit,

    // Enrichment
    enriching,
    enrichSeconds,
    handleEnrich,

    // Entity linking
    handleLinkEntity,
    handleUnlinkEntity,

    // Merge flow
    mergeDialogOpen, setMergeDialogOpen,
    mergeTarget, setMergeTarget,
    mergeConfirmOpen, setMergeConfirmOpen,
    mergeSearchQuery, setMergeSearchQuery,
    mergeSearchResults,
    merging,
    openMergeDialog,
    handleMerge,
    handleOpenSuggestedMerge,

    // Delete
    deleteConfirmOpen, setDeleteConfirmOpen,
    handleDelete,

    // Duplicates
    duplicateCandidates,

    // Files
    files,
    indexing,
    indexFeedback,
    handleIndexFiles,

    // Archive
    handleArchive,
    handleUnarchive,

    // Action creation
    addingAction, setAddingAction,
    newActionTitle, setNewActionTitle,
    creatingAction,
    handleCreateAction,
  };
}
