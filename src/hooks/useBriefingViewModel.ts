import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { BriefingLoadState } from "@/types/briefing";

/**
 * Hook to load the Daily Briefing redesign view-model from the Tauri
 * backend (DOS-413 atomic IPC).
 *
 * Returns the wire-shape `BriefingLoadState` directly — the four-state
 * envelope (`loading` | `error` | `empty` | `success`) is the contract
 * surface the consuming surface (DOS-429 DailyBriefingRedesign) renders
 * branch-by-branch.
 *
 * Features:
 * - Initial load on mount (state starts at `{status: "loading"}`).
 * - Manual `refresh()` to re-fetch.
 * - In-flight guard prevents concurrent invocations.
 *
 * Future additions tracked separately (mirroring `useDashboardData`):
 * focus-reload, workflow-completed event listener, debounce.
 */
export function useBriefingViewModel(): {
  state: BriefingLoadState;
  refresh: () => void;
  isRefreshing: boolean;
} {
  const [state, setState] = useState<BriefingLoadState>({ status: "loading" });
  const [isRefreshing, setIsRefreshing] = useState(false);
  const inFlightRef = useRef(false);

  const load = useCallback(async (showLoading: boolean) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    if (showLoading) {
      setState({ status: "loading" });
    } else {
      setIsRefreshing(true);
    }
    try {
      const result = await invoke<BriefingLoadState>("get_briefing_view_model");
      setState(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setState({ status: "error", message });
    } finally {
      inFlightRef.current = false;
      setIsRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void load(true);
  }, [load]);

  const refresh = useCallback(() => {
    void load(false);
  }, [load]);

  return { state, refresh, isRefreshing };
}
