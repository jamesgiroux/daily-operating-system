import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { annotateTrust } from "@/lib/trust-band";
import type {
  AbilityResponseJson,
  EntityContextEntry,
  EntityContextOutput,
  TrajectoryBundle,
  TrustAnnotated,
} from "@/types";

export function useEntityContextEntries(entityType: string, entityId: string | null) {
  const [entries, setEntries] = useState<Array<TrustAnnotated<EntityContextEntry>>>([]);
  const [trajectory, setTrajectory] = useState<TrajectoryBundle | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchEntries = useCallback(async () => {
    if (!entityId) {
      setEntries([]);
      setTrajectory(null);
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<AbilityResponseJson<EntityContextOutput>>("invoke_ability", {
        abilityName: "get_entity_context",
        inputJson: {
          schema_version: 2,
          entity_type: entityType,
          entity_id: entityId,
          depth: "standard",
        },
        renderSurface: "tauri_entity_detail",
        dryRun: false,
        confirmation: null,
      });
      setEntries(
        annotateTrust(result.data.entries, result.rendered_provenance, (_entry, index) => [
          `/entries/${index}/content`,
          `/entries/${index}/title`,
        ]),
      );
      setTrajectory(result.data.trajectory ?? null);
    } catch (error) {
      console.error("Failed to fetch entity context:", error);
      setEntries([]);
      setTrajectory(null);
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
      await invoke<EntityContextEntry>("create_entity_context_entry", {
        entityType,
        entityId,
        title,
        content,
      });
      await fetchEntries();
    } catch (e) {
      console.error("Failed to create entity context entry:", e);
      toast.error("Failed to save note");
    }
  }, [entityType, entityId, fetchEntries]);

  const updateEntry = useCallback(async (id: string, title: string, content: string) => {
    try {
      await invoke("update_entity_context_entry", { id, title, content });
      await fetchEntries();
    } catch (e) {
      console.error("Failed to update entity context entry:", e);
      toast.error("Failed to save note");
    }
  }, [fetchEntries]);

  const deleteEntry = useCallback(async (id: string) => {
    try {
      await invoke("delete_entity_context_entry", { id });
      await fetchEntries();
    } catch (e) {
      console.error("Failed to delete entity context entry:", e);
      toast.error("Failed to delete note");
    }
  }, [fetchEntries]);

  return { entries, trajectory, loading, createEntry, updateEntry, deleteEntry };
}
