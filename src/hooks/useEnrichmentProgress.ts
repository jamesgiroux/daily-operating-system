/**
 * useEnrichmentProgress — Progressive enrichment event listener.
 *
 * Listens for `enrichment-progress` and `enrichment-complete` Tauri events
 * emitted by the backend as each intelligence dimension finishes. Returns
 * the current progress state for a given entity so the UI can show
 * incremental percentage updates instead of a seconds counter.
 */
import { useCallback, useState } from "react";
import { useTauriEvent } from "./useTauriEvent";

export interface EnrichmentProgress {
  entityId: string;
  entityType: string;
  completed: number;
  total: number;
  lastDimension: string;
}

interface EnrichmentComplete {
  entityId: string;
  entityType: string;
  succeeded: number;
  failed: number;
  failedDimensions: string[];
  wallClockMs: number;
}

/**
 * Track progressive enrichment for a specific entity.
 *
 * Returns the current progress (or null when not enriching / after completion).
 * Also calls `onDimensionComplete` when provided, so the caller can trigger
 * a data refresh after each dimension writes to DB.
 */
export function useEnrichmentProgress(
  entityId: string | undefined,
  onDimensionComplete?: () => void,
) {
  const [progress, setProgress] = useState<EnrichmentProgress | null>(null);

  const handleProgress = useCallback(
    (payload: EnrichmentProgress) => {
      if (entityId && payload.entityId === entityId) {
        setProgress(payload);
        onDimensionComplete?.();
      }
    },
    [entityId, onDimensionComplete],
  );

  const handleComplete = useCallback(
    (payload: EnrichmentComplete) => {
      if (entityId && payload.entityId === entityId) {
        setProgress(null);
      }
    },
    [entityId],
  );

  useTauriEvent("enrichment-progress", handleProgress);
  useTauriEvent("enrichment-complete", handleComplete);

  return progress;
}
