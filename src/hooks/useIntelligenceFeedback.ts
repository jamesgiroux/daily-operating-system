import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

interface FeedbackRow {
  id: string;
  entityId: string;
  entityType: string;
  field: string;
  feedbackType: "positive" | "negative" | "replaced";
  previousValue: string | null;
  context: string | null;
  createdAt: string;
}

type FeedbackState = Record<string, "positive" | "negative" | null>;

/**
 * Compatibility hook for the legacy thumbs-up / thumbs-down UI.
 *
 * Reads feedback state through the compatibility `get_entity_feedback` query,
 * but writes through the unified `submit_intelligence_correction` command so
 * the old editorial/report surfaces share the same backend event pipeline as
 * the new binary-validation UX.
 */
export function useIntelligenceFeedback(
  entityId: string | undefined,
  entityType: string,
) {
  const [feedbackState, setFeedbackState] = useState<FeedbackState>({});
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Fetch existing feedback on mount / entity change
  useEffect(() => {
    if (!entityId) return;
    setLoading(true);
    invoke<FeedbackRow[]>("get_entity_feedback", { entityId, entityType })
      .then((rows) => {
        if (!mountedRef.current) return;
        const state: FeedbackState = {};
        // Most recent feedback per field wins (rows are DESC by created_at)
        for (const row of rows) {
          if (!state[row.field] && row.feedbackType !== "replaced") {
            state[row.field] = row.feedbackType as "positive" | "negative";
          }
        }
        setFeedbackState(state);
      })
      .catch((e) => console.error("Failed to load feedback:", e)) // Expected: background data fetch on mount
      .finally(() => {
        if (mountedRef.current) setLoading(false);
      });
  }, [entityId, entityType]);

  const getFeedback = useCallback(
    (field: string): "positive" | "negative" | null => {
      return feedbackState[field] ?? null;
    },
    [feedbackState],
  );

  const submitFeedback = useCallback(
    async (
      field: string,
      type: "positive" | "negative",
      context?: string,
    ) => {
      if (!entityId) return;
      const current = feedbackState[field];

      // Repeated clicks should be idempotent so the persisted vote and UI stay aligned.
      if (current === type) return;
      const newType = type;

      // Optimistic update
      setFeedbackState((prev) => ({ ...prev, [field]: newType }));

      try {
        await invoke("submit_intelligence_correction", {
          request: {
            entityId,
            entityType,
            field,
            action: newType === "positive" ? "confirmed" : "rejected",
            correctedValue: null,
            annotation: context ?? null,
            itemKey: null,
          },
        });
      } catch (e) {
        console.error("Failed to submit feedback:", e);
        toast.error("Failed to submit feedback");
        // Revert optimistic update
        setFeedbackState((prev) => ({ ...prev, [field]: current ?? null }));
      }
    },
    [entityId, entityType, feedbackState],
  );

  return { getFeedback, submitFeedback, loading };
}
