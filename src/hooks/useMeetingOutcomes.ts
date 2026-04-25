import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { MeetingOutcomeData } from "@/types";
import { useTauriEvent } from "./useTauriEvent";

type TranscriptProcessedPayload = MeetingOutcomeData | string;

export function useMeetingOutcomes(meetingId: string) {
  const [outcomes, setOutcomes] = useState<MeetingOutcomeData | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<MeetingOutcomeData | null>(
        "get_meeting_outcomes",
        { meetingId }
      );
      setOutcomes(result);
    } catch (err) {
      console.error("Failed to load meeting outcomes:", err); // Expected: background data fetch on mount
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleTranscriptProcessed = useCallback(
    (payload: TranscriptProcessedPayload) => {
      if (typeof payload === "string") {
        if (payload === meetingId) {
          void refresh();
        }
        return;
      }

      if (payload.meetingId === meetingId) {
        setOutcomes(payload);
      }
    },
    [meetingId, refresh],
  );

  // Listen for live updates from auto-processing or manual attach
  useTauriEvent<TranscriptProcessedPayload>(
    "transcript-processed",
    handleTranscriptProcessed,
  );

  return { outcomes, loading, refresh };
}
