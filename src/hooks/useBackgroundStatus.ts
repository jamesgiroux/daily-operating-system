/**
 * useBackgroundStatus.ts — I571
 *
 * Listens for `background-work-status` Tauri events from the intel queue
 * and surfaces status via the existing Sonner toast system.
 */

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

interface BackgroundStatus {
  phase: "started" | "completed";
  message: string;
  count?: number;
}

const TOAST_ID = "background-work-status";

export function useBackgroundStatus() {
  useEffect(() => {
    const unlisten = listen<BackgroundStatus>("background-work-status", (event) => {
      const { phase, message } = event.payload;

      if (phase === "started") {
        toast.loading(message, { id: TOAST_ID, duration: 30000 });
      } else if (phase === "completed") {
        toast.success(message, { id: TOAST_ID, duration: 3000 });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
