import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useTauriEvent } from "./useTauriEvent";
import type { DbAction } from "@/types";

interface UseProposedActionsReturn {
  proposedActions: DbAction[];
  acceptAction: (id: string) => Promise<void>;
  rejectAction: (id: string, source?: "actions_page" | "daily_briefing" | "meeting_detail") => Promise<void>;
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
      console.error("Failed to load proposed actions:", err); // Expected: background data fetch on mount
      setProposedActions([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Re-fetch proposed actions when transcripts are processed or intelligence updates
  useTauriEvent("transcript-processed", refresh);
  useTauriEvent("intelligence-updated", refresh);

  const acceptAction = useCallback(
    async (id: string) => {
      try {
        await invoke("accept_proposed_action", { id });
        setProposedActions((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        console.error("Failed to accept action:", err);
        toast.error("Failed to accept action");
      }
    },
    []
  );

  const rejectAction = useCallback(
    async (
      id: string,
      source: "actions_page" | "daily_briefing" | "meeting_detail" = "actions_page"
    ) => {
      try {
        await invoke("reject_proposed_action", { id, source });
        setProposedActions((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        console.error("Failed to reject action:", err);
        toast.error("Failed to dismiss action");
      }
    },
    []
  );

  return { proposedActions, acceptAction, rejectAction, isLoading, refresh };
}
