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
 * Payload emitted by `GleanProvider` when one or more of the 6 enrichment
 * dimensions fail. When `will_fall_back` is true, Glean returned nothing
 * usable and the backend swaps to the legacy PTY enrichment path.
 *
 * Shape defined in src-tauri/src/intelligence/glean_provider.rs (~L412-423).
 */
interface GleanDegradedPayload {
  entity_id: string;
  entity_type: string;
  succeeded: number;
  failed: number;
  failed_dimensions: string[];
  wall_clock_ms: number;
  will_fall_back: boolean;
}

/**
 * Payload emitted by `services/intelligence.rs` when manual Glean enrichment
 * throws before any dimensions return — backend then falls through to PTY.
 *
 * Shape defined in src-tauri/src/services/intelligence.rs (~L276-284).
 */
interface GleanFallbackPayload {
  entity_id: string;
  entity_type: string;
  entity_name: string;
  reason: string;
}

/** Total number of Glean enrichment dimensions (health, relationships, etc.). */
const GLEAN_DIMENSIONS_TOTAL = 6;

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

    // Glean enrichment degraded — partial dimension failure. If the
    // backend is also about to fall back to PTY (`will_fall_back`), let
    // the paired `enrichment-glean-fallback` toast own the messaging so
    // we don't double-toast the same account.
    unlisteners.push(
      listen<GleanDegradedPayload>("enrichment-glean-degraded", (event) => {
        const payload = event.payload;
        if (!payload || payload.will_fall_back) return;
        const succeeded = Math.max(
          0,
          Math.min(payload.succeeded, GLEAN_DIMENSIONS_TOTAL),
        );
        toast.warning(
          `Glean enrichment degraded (${succeeded}/${GLEAN_DIMENSIONS_TOTAL} dimensions) — showing partial results`,
          { duration: 10000 },
        );
      }),
    );

    // Glean enrichment fully unavailable — backend fell back to the
    // local PTY enrichment path, so results the user is looking at are
    // local-sourced even on a Glean-mode account.
    unlisteners.push(
      listen<GleanFallbackPayload>("enrichment-glean-fallback", () => {
        toast.warning(
          "Glean unavailable — fell back to local enrichment",
          { duration: 10000 },
        );
      }),
    );

    return () => {
      for (const p of unlisteners) {
        p.then((fn) => fn()).catch(() => {});
      }
    };
  }, []);
}
