import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useTauriEvent } from "./useTauriEvent";
import type { DbAction } from "@/types";

interface UseSuggestedActionsReturn {
  suggestedActions: DbAction[];
  acceptAction: (id: string) => Promise<void>;
  rejectAction: (id: string, source?: "actions_page" | "daily_briefing" | "meeting_detail") => Promise<void>;
  isLoading: boolean;
  refresh: () => Promise<void>;
  showAll: boolean;
  setShowAll: (v: boolean) => void;
}

/**
 * Suggested actions hook.
 *
 * `showAll=false` (default): scopes to the user's own commitments +
 * unassigned rows via the backend's `user_entity.name` match. Without
 * this filter the list is dominated by other people's commitments that
 * AI extraction tags while transcribing meetings — observed 355 rows
 * total with only 26 actually owned by the user on a real workspace.
 *
 * `showAll=true`: returns every backlog row regardless of owner.
 */
export function useSuggestedActions(): UseSuggestedActionsReturn {
  const [suggestedActions, setSuggestedActions] = useState<DbAction[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showAll, setShowAll] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<DbAction[]>("get_suggested_actions", { showAll });
      setSuggestedActions(result);
    } catch (err) {
      console.error("Failed to load suggested actions:", err); // Expected: background data fetch on mount
      setSuggestedActions([]);
    } finally {
      setIsLoading(false);
    }
  }, [showAll]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Re-fetch suggested actions when transcripts are processed or intelligence updates
  useTauriEvent("transcript-processed", refresh);
  useTauriEvent("intelligence-updated", refresh);

  const acceptAction = useCallback(
    async (id: string) => {
      try {
        await invoke("accept_suggested_action", { id });
        setSuggestedActions((prev) => prev.filter((a) => a.id !== id));
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
        await invoke("reject_suggested_action", { id, source });
        setSuggestedActions((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        console.error("Failed to reject action:", err);
        toast.error("Failed to dismiss action");
      }
    },
    []
  );

  return { suggestedActions, acceptAction, rejectAction, isLoading, refresh, showAll, setShowAll };
}
