/**
 * useProjectDetail — Orchestrator hook for the project detail editorial page.
 * Mirrors useAccountDetail pattern: load, field editing, enrichment, archive.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "@tanstack/react-router";
import type { ProjectDetail, ContentFile } from "@/types";

export function useProjectDetail(projectId: string | undefined) {
  const navigate = useNavigate();
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Enrichment
  const [enriching, setEnriching] = useState(false);
  const [enrichSeconds, setEnrichSeconds] = useState(0);
  const enrichTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Editable structured fields
  const [editName, setEditName] = useState("");
  const [editStatus, setEditStatus] = useState("");
  const [editMilestone, setEditMilestone] = useState("");
  const [editOwner, setEditOwner] = useState("");
  const [editTargetDate, setEditTargetDate] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  // Inline action creation
  const [addingAction, setAddingAction] = useState(false);
  const [newActionTitle, setNewActionTitle] = useState("");
  const [creatingAction, setCreatingAction] = useState(false);

  // Files
  const [files, setFiles] = useState<ContentFile[]>([]);
  const [indexing, setIndexing] = useState(false);
  const [indexFeedback, setIndexFeedback] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!projectId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ProjectDetail>("get_project_detail", { projectId });
      setDetail(result);
      setEditName(result.name);
      setEditStatus(result.status ?? "active");
      setEditMilestone(result.milestone ?? "");
      setEditOwner(result.owner ?? "");
      setEditTargetDate(result.targetDate ?? "");
      setEditNotes(result.notes ?? "");
      setDirty(false);

      // Load files
      try {
        const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
          entityType: "project",
          entityId: projectId,
        });
        setFiles(contentFiles);
      } catch {
        /* non-critical */
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    load();
  }, [load]);

  // Listen for intelligence-updated events
  useEffect(() => {
    if (!projectId) return;
    const unlisten = listen("intelligence-updated", (event) => {
      const payload = event.payload as { entity_type?: string; entity_id?: string };
      if (payload.entity_type === "project" && payload.entity_id === projectId) {
        load();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [projectId, load]);

  async function handleSave() {
    if (!detail) return;
    setSaving(true);
    try {
      const fieldUpdates: [string, string][] = [];
      if (editName !== detail.name) fieldUpdates.push(["name", editName]);
      if (editStatus !== (detail.status ?? "")) fieldUpdates.push(["status", editStatus]);
      if (editMilestone !== (detail.milestone ?? "")) fieldUpdates.push(["milestone", editMilestone]);
      if (editOwner !== (detail.owner ?? "")) fieldUpdates.push(["owner", editOwner]);
      if (editTargetDate !== (detail.targetDate ?? "")) fieldUpdates.push(["target_date", editTargetDate]);

      for (const [field, value] of fieldUpdates) {
        await invoke("update_project_field", { projectId: detail.id, field, value });
      }

      if (editNotes !== (detail.notes ?? "")) {
        await invoke("update_project_notes", { projectId: detail.id, notes: editNotes });
      }

      setDirty(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function handleCancelEdit() {
    if (!detail) return;
    setEditName(detail.name);
    setEditStatus(detail.status ?? "active");
    setEditMilestone(detail.milestone ?? "");
    setEditOwner(detail.owner ?? "");
    setEditTargetDate(detail.targetDate ?? "");
    setEditNotes(detail.notes ?? "");
    setDirty(false);
  }

  async function handleEnrich() {
    if (!detail) return;
    setEnriching(true);
    setEnrichSeconds(0);
    enrichTimerRef.current = setInterval(() => {
      setEnrichSeconds((s) => s + 1);
    }, 1000);
    try {
      await invoke("enrich_project", { projectId: detail.id });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setEnriching(false);
      if (enrichTimerRef.current) clearInterval(enrichTimerRef.current);
    }
  }

  async function handleArchive() {
    if (!detail) return;
    try {
      await invoke("archive_project", { id: detail.id, archived: true });
      navigate({ to: "/projects" });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnarchive() {
    if (!detail) return;
    try {
      await invoke("archive_project", { id: detail.id, archived: false });
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
        title: newActionTitle.trim(),
        entityType: "project",
        entityId: detail.id,
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

  async function handleIndexFiles() {
    if (!detail) return;
    setIndexing(true);
    setIndexFeedback(null);
    try {
      const result = await invoke<string>("index_entity_content", {
        entityType: "project",
        entityId: detail.id,
      });
      setIndexFeedback(result);
      // Reload files
      const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
        entityType: "project",
        entityId: detail.id,
      });
      setFiles(contentFiles);
    } catch (e) {
      setIndexFeedback(`Error: ${e}`);
    } finally {
      setIndexing(false);
    }
  }

  const intelligence = detail?.intelligence ?? null;

  return {
    detail,
    intelligence,
    loading,
    error,
    files,
    load,
    // Field editing
    editName, setEditName,
    editStatus, setEditStatus,
    editMilestone, setEditMilestone,
    editOwner, setEditOwner,
    editTargetDate, setEditTargetDate,
    editNotes, setEditNotes,
    dirty, setDirty,
    saving,
    handleSave,
    handleCancelEdit,
    // Enrichment
    enriching,
    enrichSeconds,
    handleEnrich,
    // Archive
    handleArchive,
    handleUnarchive,
    // Actions
    addingAction, setAddingAction,
    newActionTitle, setNewActionTitle,
    creatingAction,
    handleCreateAction,
    // File indexing
    indexing,
    indexFeedback,
    handleIndexFiles,
  };
}
