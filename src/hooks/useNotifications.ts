import { useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { GoogleAuthStatus } from "@/types";
import { useTauriEvent } from "./useTauriEvent";

interface SystemStatusPayload {
  type: "pipeline_error" | "auth_expired";
  message: string;
}

/**
 * Payload emitted by `GleanProvider` when one or more applicable refresh
 * dimensions fail. When `will_fall_back` is true, Glean returned nothing
 * usable and the backend swaps to the legacy PTY enrichment path.
 *
 * Shape defined in src-tauri/src/intelligence/glean_provider.rs (~L412-423).
 */
export interface GleanDegradedPayload {
  entity_id: string;
  entity_type: string;
  succeeded: number;
  failed: number;
  failed_dimensions: string[];
  required_failed_dimensions?: string[];
  optional_failed_dimensions?: string[];
  total_dimensions?: number;
  applicable_total?: number;
  skipped_dimensions?: string[];
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

/** Legacy total used when older backend payloads do not include applicability. */
const GLEAN_DIMENSIONS_TOTAL = 6;

export function formatGleanDegradedMessage(payload: GleanDegradedPayload) {
  const applicableTotal = Math.max(
    1,
    payload.applicable_total ?? payload.total_dimensions ?? GLEAN_DIMENSIONS_TOTAL,
  );
  const succeeded = Math.max(0, Math.min(payload.succeeded, applicableTotal));
  const skippedCount = payload.skipped_dimensions?.length ?? 0;
  const requiredFailures =
    payload.required_failed_dimensions ?? payload.failed_dimensions ?? [];
  const skippedText =
    skippedCount > 0 ? `; ${skippedCount} not applicable` : "";
  const failureText =
    requiredFailures.length > 0
      ? `; ${requiredFailures.length} need attention`
      : "";

  return `Glean refresh incomplete (${succeeded}/${applicableTotal} applicable areas updated${skippedText}${failureText}) - showing partial results`;
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

  const handleOperationDelivered = useCallback((operation: string) => {
    const label = milestoneLabels[operation];
    if (label) {
      const now = Date.now();
      const lastShown = lastMilestoneRef.current[operation] ?? 0;
      if (now - lastShown < DEDUP_WINDOW_MS) return;
      lastMilestoneRef.current[operation] = now;
      toast.success(label);
    }
  }, []);

  const handleEmailError = useCallback((message: string) => {
    toast.error(message || "Email processing failed", {
      duration: 10000,
    });
  }, []);

  const handleGoogleAuthChanged = useCallback((status: GoogleAuthStatus) => {
    if (status?.status === "tokenexpired") {
      toast.warning("Google token expired — reconnect in Settings", {
        duration: 10000,
      });
    }
  }, []);

  const handleSystemStatus = useCallback((payload: SystemStatusPayload) => {
    const { type, message } = payload;
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
  }, []);

  const handleGleanDegraded = useCallback((payload: GleanDegradedPayload) => {
    if (!payload || payload.will_fall_back) return;
    toast.warning(formatGleanDegradedMessage(payload), { duration: 10000 });
  }, []);

  const handleGleanFallback = useCallback((_payload: GleanFallbackPayload) => {
    toast.warning("Glean unavailable — fell back to local enrichment", {
      duration: 10000,
    });
  }, []);

  // Milestone operations — briefing and weekly briefing ready
  // Deduplicated: same type within 30s is suppressed
  useTauriEvent("operation-delivered", handleOperationDelivered);

  // Email errors surfaced globally. EmailList also shows an inline
  // banner on the dashboard — the overlap is intentional: the banner
  // is contextual (visible on dashboard), the toast is ambient
  // (visible from any page).
  useTauriEvent("email-error", handleEmailError);

  // Google auth token expiry — fires from background refresh,
  // not from user-initiated connect/disconnect (those already toast)
  useTauriEvent("google-auth-changed", handleGoogleAuthChanged);

  // System status events — pipeline errors and auth issues from backend
  useTauriEvent("system-status", handleSystemStatus);

  // Glean refresh incomplete — partial dimension failure. If the
  // backend is also about to fall back to PTY (`will_fall_back`), let
  // the paired `enrichment-glean-fallback` toast own the messaging so
  // we don't double-toast the same account.
  useTauriEvent("enrichment-glean-degraded", handleGleanDegraded);

  // Glean enrichment fully unavailable — backend fell back to the
  // local PTY enrichment path, so results the user is looking at are
  // local-sourced even on a Glean-mode account.
  useTauriEvent("enrichment-glean-fallback", handleGleanFallback);
}
