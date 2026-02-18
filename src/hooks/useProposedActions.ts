import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DbAction } from "@/types";

interface UseProposedActionsReturn {
  proposedActions: DbAction[];
  acceptAction: (id: string) => Promise<void>;
  rejectAction: (id: string) => Promise<void>;
  isLoading: boolean;
  refresh: () => Promise<void>;
}

export function useProposedActions(): UseProposedActionsReturn {
  const [proposedActions, setProposedActions] = useState<DbAction[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<DbAction[]>("get_proposed_actions");
      setProposedActions(result);
    } catch (err) {
      console.error("Failed to load proposed actions:", err);
      setProposedActions([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const acceptAction = useCallback(
    async (id: string) => {
      try {
        await invoke("accept_proposed_action", { id });
        setProposedActions((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        console.error("Failed to accept action:", err);
      }
    },
    []
  );

  const rejectAction = useCallback(
    async (id: string) => {
      try {
        await invoke("reject_proposed_action", { id });
        setProposedActions((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        console.error("Failed to reject action:", err);
      }
    },
    []
  );

  return { proposedActions, acceptAction, rejectAction, isLoading, refresh };
}
