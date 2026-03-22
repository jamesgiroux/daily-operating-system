/**
 * useBackgroundStatus.ts — I571
 *
 * Listens for `background-work-status` Tauri events from the intel queue
 * and returns state for the BackgroundWorkIndicator.
 *
 * Toasts are reserved for manual refreshes (user-initiated) and failures.
 * Automatic background work drives a quiet persistent indicator instead.
 */

import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

interface BackgroundStatusEvent {
  phase: "started" | "completed" | "failed";
  message: string;
  count?: number;
  error?: string;
  stage?: string;
  manual?: boolean;
}

export interface BackgroundWorkState {
  /** Whether background work is currently in progress */
  active: boolean;
  /** Descriptive message (e.g. "Updating Acme, FooCorp...") */
  message: string;
  /** Current phase */
  phase: "idle" | "started" | "completed" | "failed";
}

const TOAST_ID = "background-work-status";

/** How long to show completed/failed state before clearing to idle */
const CLEAR_DELAY_MS = 3000;

export function useBackgroundStatus(): BackgroundWorkState {
  const [state, setState] = useState<BackgroundWorkState>({
    active: false,
    message: "",
    phase: "idle",
  });
  const clearTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const unlisten = listen<BackgroundStatusEvent>("background-work-status", (event) => {
      const { phase, message, error, manual } = event.payload;

      // Clear any pending auto-clear timer
      if (clearTimerRef.current) {
        clearTimeout(clearTimerRef.current);
        clearTimerRef.current = null;
      }

      if (phase === "started") {
        setState({ active: true, message, phase: "started" });

        // Only toast for manual (user-initiated) refreshes
        if (manual) {
          toast.loading(message, { id: TOAST_ID, duration: 30000 });
        }
      } else if (phase === "completed") {
        setState({ active: false, message, phase: "completed" });

        if (manual) {
          toast.success(message, { id: TOAST_ID, duration: 3000 });
        }

        // Auto-clear completed state after delay
        clearTimerRef.current = setTimeout(() => {
          setState({ active: false, message: "", phase: "idle" });
        }, CLEAR_DELAY_MS);
      } else if (phase === "failed") {
        setState({ active: false, message: error || message, phase: "failed" });

        // Always toast failures — errors should be visible regardless of source
        toast.error(error ? `${message}: ${error}` : message, {
          id: TOAST_ID,
          duration: 8000,
        });

        // Auto-clear failed state after delay
        clearTimerRef.current = setTimeout(() => {
          setState({ active: false, message: "", phase: "idle" });
        }, CLEAR_DELAY_MS);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      if (clearTimerRef.current) {
        clearTimeout(clearTimerRef.current);
      }
    };
  }, []);

  return state;
}
