/**
 * useEnrichmentProgress — Progressive enrichment event listener (I575).
 *
 * Listens for `enrichment-progress` and `enrichment-complete` Tauri events
 * emitted by the backend as each intelligence dimension finishes. Returns
 * the current progress state for a given entity so the UI can show
 * incremental percentage updates instead of a seconds counter.
 */
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

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

  useEffect(() => {
    if (!entityId) return;

    const unlistenProgress = listen<EnrichmentProgress>(
      "enrichment-progress",
      (event) => {
        if (event.payload.entityId === entityId) {
          setProgress(event.payload);
          onDimensionComplete?.();
        }
      },
    );

    const unlistenComplete = listen<EnrichmentComplete>(
      "enrichment-complete",
      (event) => {
        if (event.payload.entityId === entityId) {
          setProgress(null);
        }
      },
    );

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, [entityId, onDimensionComplete]);

  return progress;
}
