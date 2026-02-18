import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DbAction } from "@/types";

type StatusFilter = "all" | "proposed" | "pending" | "completed" | "waiting";
type PriorityFilter = "all" | "P1" | "P2" | "P3";

export interface CreateActionParams {
  title: string;
  priority?: string;
  dueDate?: string;
  accountId?: string;
  projectId?: string;
  personId?: string;
  context?: string;
  sourceLabel?: string;
}

interface UseActionsReturn {
  actions: DbAction[];
  allActions: DbAction[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  createAction: (params: CreateActionParams) => Promise<string>;
  completeAction: (id: string) => Promise<void>;
  toggleAction: (id: string) => Promise<void>;
  statusFilter: StatusFilter;
  setStatusFilter: (f: StatusFilter) => void;
  priorityFilter: PriorityFilter;
  setPriorityFilter: (f: PriorityFilter) => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
}

/**
 * Hook for SQLite-backed actions with filters and interactive completion.
 *
 * Replaces the old JSON-based `get_all_actions` approach with persistent
 * cross-day action tracking.
 */
export function useActions(initialSearch?: string): UseActionsReturn {
  const [allActions, setAllActions] = useState<DbAction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("pending");
  const [priorityFilter, setPriorityFilter] = useState<PriorityFilter>("all");
  const [searchQuery, setSearchQuery] = useState(initialSearch ?? "");

  const loadActions = useCallback(async () => {
    try {
      // Load a wide window (90 days) to get all relevant actions
      const result = await invoke<DbAction[]>("get_actions_from_db", {
        daysAhead: 90,
      });
      setAllActions(result);
      setError(null);
    } catch (err) {
      // Fallback: try the JSON-based loader if DB isn't populated yet
      try {
        const jsonResult = await invoke<{
          status: string;
          data?: Array<{
            id: string;
            title: string;
            priority: string;
            status: string;
            account?: string;
            dueDate?: string;
            context?: string;
            source?: string;
          }>;
        }>("get_all_actions");

        if (jsonResult.status === "success" && jsonResult.data) {
          // Map JSON actions to DbAction shape
          const mapped: DbAction[] = jsonResult.data.map((a) => ({
            id: a.id,
            title: a.title,
            priority: a.priority,
            status: a.status,
            createdAt: new Date().toISOString(),
            dueDate: a.dueDate,
            accountId: a.account,
            sourceLabel: a.source,
            context: a.context,
            updatedAt: new Date().toISOString(),
          }));
          setAllActions(mapped);
          setError(null);
        } else {
          setAllActions([]);
        }
      } catch {
        setError(err instanceof Error ? err.message : "Failed to load actions");
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadActions();
  }, [loadActions]);

  const createAction = useCallback(
    async (params: CreateActionParams): Promise<string> => {
      const id = await invoke<string>("create_action", {
        request: {
          title: params.title,
          priority: params.priority,
          dueDate: params.dueDate,
          accountId: params.accountId,
          projectId: params.projectId,
          personId: params.personId,
          context: params.context,
          sourceLabel: params.sourceLabel,
        },
      });
      await loadActions();
      return id;
    },
    [loadActions]
  );

  const completeAction = useCallback(
    async (id: string) => {
      try {
        await invoke("complete_action", { id });
        setAllActions((prev) =>
          prev.map((a) =>
            a.id === id
              ? { ...a, status: "completed", completedAt: new Date().toISOString() }
              : a
          )
        );
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to complete action");
      }
    },
    []
  );

  const toggleAction = useCallback(
    async (id: string) => {
      const action = allActions.find((a) => a.id === id);
      if (!action) return;

      const isCompleted = action.status === "completed";
      try {
        if (isCompleted) {
          await invoke("reopen_action", { id });
          setAllActions((prev) =>
            prev.map((a) =>
              a.id === id
                ? { ...a, status: "pending", completedAt: undefined }
                : a
            )
          );
        } else {
          await invoke("complete_action", { id });
          setAllActions((prev) =>
            prev.map((a) =>
              a.id === id
                ? { ...a, status: "completed", completedAt: new Date().toISOString() }
                : a
            )
          );
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to update action");
      }
    },
    [allActions]
  );

  // Apply filters
  const actions = allActions.filter((action) => {
    // Status filter
    if (statusFilter !== "all" && action.status !== statusFilter) {
      return false;
    }

    // Priority filter
    if (priorityFilter !== "all" && action.priority !== priorityFilter) {
      return false;
    }

    // Search
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      const matchesTitle = action.title.toLowerCase().includes(q);
      const matchesAccount = action.accountId?.toLowerCase().includes(q);
      const matchesContext = action.context?.toLowerCase().includes(q);
      if (!matchesTitle && !matchesAccount && !matchesContext) {
        return false;
      }
    }

    return true;
  });

  return {
    actions,
    allActions,
    loading,
    error,
    refresh: loadActions,
    createAction,
    completeAction,
    toggleAction,
    statusFilter,
    setStatusFilter,
    priorityFilter,
    setPriorityFilter,
    searchQuery,
    setSearchQuery,
  };
}
