import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
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
 */
export function useDashboardData(): {
  state: DashboardLoadState;
  refresh: () => void;
} {
  const [state, setState] = useState<DashboardLoadState>({ status: "loading" });

  const loadData = useCallback(async () => {
    setState({ status: "loading" });

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
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  return { state, refresh: loadData };
}
