import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { GoogleAuthStatus } from "@/types";

/**
 * Milestone operations worth toasting. During a morning workflow, the
 * pipeline fires ~7 operation-delivered events in rapid succession.
 * Only surface the milestones users actually care about — not every
 * intermediate step.
 *
 * "briefing" is the final product of the morning pipeline.
 * "week-enriched" is the final product of the weekly pipeline.
 * Intermediate steps (schedule, actions, preps, emails) are intentionally
 * omitted to avoid toast spam.
 */
const milestoneLabels: Record<string, string> = {
  briefing: "Daily briefing ready",
  "week-enriched": "Weekly briefing ready",
};

/**
 * Global notification listener. Mounts once at the app root and
 * surfaces key backend events as toast notifications.
 *
 * Design intent: the user should know when background work completes
 * without having to watch a specific page. Toasts are informational,
 * not actionable — they confirm "something happened" and fade away.
 */
export function useNotifications() {
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    // Milestone operations — briefing and weekly briefing ready
    unlisteners.push(
      listen<string>("operation-delivered", (event) => {
        const label = milestoneLabels[event.payload];
        if (label) {
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

    return () => {
      for (const p of unlisteners) {
        p.then((fn) => fn()).catch(() => {});
      }
    };
  }, []);
}
