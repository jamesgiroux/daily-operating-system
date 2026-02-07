import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { MeetingOutcomeData } from "@/types";

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
      console.error("Failed to load meeting outcomes:", err);
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    refresh();

    // Listen for live updates from auto-processing or manual attach
    const unlisten = listen<MeetingOutcomeData>(
      "transcript-processed",
      (event) => {
        if (event.payload.meetingId === meetingId) {
          setOutcomes(event.payload);
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [meetingId, refresh]);

  return { outcomes, loading, refresh };
}
