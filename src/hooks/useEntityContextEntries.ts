import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { EntityContextEntry } from "@/types";

export function useEntityContextEntries(entityType: string, entityId: string | null) {
  const [entries, setEntries] = useState<EntityContextEntry[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchEntries = useCallback(async () => {
    if (!entityId) return;
    setLoading(true);
    try {
      const result = await invoke<EntityContextEntry[]>("get_entity_context_entries", {
        entityType,
        entityId,
      });
      setEntries(result);
    } catch (e) {
      console.error("Failed to fetch entity context entries:", e);
      toast.error("Failed to load notes");
    } finally {
      setLoading(false);
    }
  }, [entityType, entityId]);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  const createEntry = useCallback(async (title: string, content: string) => {
    if (!entityId) return;
    try {
      const entry = await invoke<EntityContextEntry>("create_entity_context_entry", {
        entityType,
        entityId,
        title,
        content,
      });
      setEntries((prev) => [entry, ...prev]);
    } catch (e) {
      console.error("Failed to create entity context entry:", e);
      toast.error("Failed to save note");
    }
  }, [entityType, entityId]);

  const updateEntry = useCallback(async (id: string, title: string, content: string) => {
    try {
      await invoke("update_entity_context_entry", { id, title, content });
      setEntries((prev) =>
        prev.map((e) =>
          e.id === id ? { ...e, title, content, updatedAt: new Date().toISOString() } : e,
        ),
      );
    } catch (e) {
      console.error("Failed to update entity context entry:", e);
      toast.error("Failed to save note");
    }
  }, []);

  const deleteEntry = useCallback(async (id: string) => {
    try {
      await invoke("delete_entity_context_entry", { id });
      setEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (e) {
      console.error("Failed to delete entity context entry:", e);
      toast.error("Failed to delete note");
    }
  }, []);

  return { entries, loading, createEntry, updateEntry, deleteEntry };
}
