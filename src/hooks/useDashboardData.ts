import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardData, DataFreshness, GoogleAuthStatus } from "@/types";

/**
 * Discriminated union for dashboard loading states
 */
export type DashboardLoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "empty"; message: string; googleAuth?: GoogleAuthStatus }
  | { status: "success"; data: DashboardData; freshness: DataFreshness; googleAuth?: GoogleAuthStatus };

/**
 * Response from get_dashboard_data Tauri command
 */
type DashboardResult =
  | { status: "success"; data: DashboardData; freshness: DataFreshness; googleAuth: GoogleAuthStatus }
  | { status: "empty"; message: string; googleAuth: GoogleAuthStatus }
  | { status: "error"; message: string };

/**
 * Hook to load dashboard data from the Tauri backend
 *
 * Features:
 * - Loads fresh data on mount
 * - Re-fetches when window regains focus (catches workflows that
 *   completed while user was on another page or app)
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
  const inFlightRef = useRef(false);
  const lastFocusRefreshRef = useRef(0);

  const loadData = useCallback(async (showLoading = true) => {
    if (inFlightRef.current) {
      return;
    }
    inFlightRef.current = true;

    if (showLoading) {
      setState({ status: "loading" });
    } else {
      setIsRefreshing(true);
    }

    try {
      const result = await invoke<DashboardResult>("get_dashboard_data");

      switch (result.status) {
        case "success":
          setState({ status: "success", data: result.data, freshness: result.freshness, googleAuth: result.googleAuth });
          break;
        case "empty":
          setState({ status: "empty", message: result.message, googleAuth: result.googleAuth });
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
      inFlightRef.current = false;
      setIsRefreshing(false);
    }
  }, []);

  // Load on mount
  useEffect(() => {
    loadData();
  }, [loadData]);

  // Re-fetch when the window regains focus — catches data that changed
  // while user was on Settings, another page, or another app entirely.
  useEffect(() => {
    const onFocus = () => {
      const now = Date.now();
      if (now - lastFocusRefreshRef.current < 60_000) {
        return;
      }
      lastFocusRefreshRef.current = now;
      void loadData(false);
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [loadData]);

  // Auto-refresh on workflow completion (works when Dashboard is mounted)
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen("workflow-completed", () => {
      loadData(false);
    }).then((fn) => {
      if (cancelled) {
        fn(); // Component already unmounted — immediately unlisten
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [loadData]);

  // Silent refresh when calendar poll detects changes (ADR-0032)
  // Also refreshes on prep-ready (I41 — reactive prep generation)
  // Also refreshes on entity-updated (meeting entity reassignment cascade)
  useEffect(() => {
    let unlistenCalendar: UnlistenFn | undefined;
    let unlistenPrep: UnlistenFn | undefined;
    let unlistenEntity: UnlistenFn | undefined;
    let cancelled = false;

    listen("calendar-updated", () => {
      loadData(false);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlistenCalendar = fn;
      }
    });

    listen("prep-ready", () => {
      loadData(false);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlistenPrep = fn;
      }
    });

    listen("entity-updated", () => {
      loadData(false);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlistenEntity = fn;
      }
    });

    return () => {
      cancelled = true;
      unlistenCalendar?.();
      unlistenPrep?.();
      unlistenEntity?.();
    };
  }, [loadData]);

  return {
    state,
    refresh: () => loadData(true),
    isRefreshing,
  };
}
