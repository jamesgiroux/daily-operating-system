import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardData } from "@/types";

/**
 * Discriminated union for dashboard loading states
 */
export type DashboardLoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "empty"; message: string }
  | { status: "success"; data: DashboardData };

/**
 * Response from get_dashboard_data Tauri command
 */
type DashboardResult =
  | { status: "success"; data: DashboardData }
  | { status: "empty"; message: string }
  | { status: "error"; message: string };

/**
 * Hook to load dashboard data from the Tauri backend
 *
 * Features:
 * - Initial load on mount
 * - Manual refresh via `refresh()` function
 * - Auto-refresh when `workflow-completed` event is received
 */
export function useDashboardData(): {
  state: DashboardLoadState;
  refresh: () => void;
  isRefreshing: boolean;
} {
  const [state, setState] = useState<DashboardLoadState>({ status: "loading" });
  const [isRefreshing, setIsRefreshing] = useState(false);

  const loadData = useCallback(async (showLoading = true) => {
    if (showLoading) {
      setState({ status: "loading" });
    } else {
      setIsRefreshing(true);
    }

    try {
      const result = await invoke<DashboardResult>("get_dashboard_data");

      switch (result.status) {
        case "success":
          setState({ status: "success", data: result.data });
          break;
        case "empty":
          setState({ status: "empty", message: result.message });
          break;
        case "error":
          setState({ status: "error", message: result.message });
          break;
      }
    } catch (err) {
      setState({
        status: "error",
        message: err instanceof Error ? err.message : "Unknown error occurred",
      });
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadData();
  }, [loadData]);

  // Auto-refresh on workflow completion
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      // Listen for workflow-completed events
      unlisten = await listen("workflow-completed", () => {
        // Refresh without showing loading state (smoother UX)
        loadData(false);
      });
    };

    setupListener();

    return () => {
      unlisten?.();
    };
  }, [loadData]);

  return {
    state,
    refresh: () => loadData(true),
    isRefreshing,
  };
}
