import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { GoogleAuthStatus } from "@/types";

interface SystemStatusPayload {
  type: "pipeline_error" | "auth_expired";
  message: string;
}

/**
 * Milestone operations worth toasting.
 *
 * Only surface milestones users care about — not every intermediate step.
 * Uses deduplication: same milestone type within 30s is suppressed.
 *
 * "briefing" is the final product of the morning pipeline.
 * "week-enriched" is the final product of the weekly pipeline.
 */
const milestoneLabels: Record<string, string> = {
  briefing: "Daily briefing ready",
  "week-enriched": "Weekly briefing ready",
};

/** Dedup window: suppress duplicate milestone toasts within this period. */
const DEDUP_WINDOW_MS = 30_000;

/**
 * Global notification listener. Mounts once at the app root and
 * surfaces key backend events as toast notifications.
 *
 * Design: background completions are silent — data just appears via
 * startTransition refreshes. Toasts are reserved for milestones
 * (briefing ready), errors, and auth issues. Duplicate milestones
 * within 30s are suppressed to avoid toast stacking.
 */
export function useNotifications() {
  const lastMilestoneRef = useRef<Record<string, number>>({});

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    // Milestone operations — briefing and weekly briefing ready
    // Deduplicated: same type within 30s is suppressed
    unlisteners.push(
      listen<string>("operation-delivered", (event) => {
        const label = milestoneLabels[event.payload];
        if (label) {
          const now = Date.now();
          const lastShown = lastMilestoneRef.current[event.payload] ?? 0;
          if (now - lastShown < DEDUP_WINDOW_MS) return;
          lastMilestoneRef.current[event.payload] = now;
          toast.success(label);
        }
      }),
    );

    // Email errors surfaced globally. EmailList also shows an inline
    // banner on the dashboard — the overlap is intentional: the banner
    // is contextual (visible on dashboard), the toast is ambient
    // (visible from any page).
    unlisteners.push(
      listen<string>("email-error", (event) => {
        toast.error(event.payload || "Email processing failed", {
          duration: 10000,
        });
      }),
    );

    // Google auth token expiry — fires from background refresh,
    // not from user-initiated connect/disconnect (those already toast)
    unlisteners.push(
      listen<GoogleAuthStatus>("google-auth-changed", (event) => {
        if (event.payload?.status === "tokenexpired") {
          toast.warning("Google token expired — reconnect in Settings", {
            duration: 10000,
          });
        }
      }),
    );

    // System status events — pipeline errors and auth issues from backend
    unlisteners.push(
      listen<SystemStatusPayload>("system-status", (event) => {
        const { type, message } = event.payload;
        if (type === "auth_expired") {
          toast.warning(message || "Google account needs reconnection", {
            duration: 15000,
          });
        } else if (type === "pipeline_error") {
          toast.error(message || "A pipeline operation failed", {
            duration: 10000,
            action: {
              label: "Retry",
              onClick: () => invoke("refresh_emails").catch(() => {}),
            },
          });
        }
      }),
    );

    return () => {
      for (const p of unlisteners) {
        p.then((fn) => fn()).catch(() => {});
      }
    };
  }, []);
}
