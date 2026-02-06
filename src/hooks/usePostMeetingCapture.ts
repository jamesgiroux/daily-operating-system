import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CalendarEvent, CapturedOutcome } from "@/types";

interface CaptureState {
  visible: boolean;
  meeting: CalendarEvent | null;
  isFallback: boolean;
}

export function usePostMeetingCapture() {
  const [state, setState] = useState<CaptureState>({
    visible: false,
    meeting: null,
    isFallback: false,
  });

  useEffect(() => {
    // Full capture prompt (manual trigger or auto with transcript)
    const unlistenFull = listen<CalendarEvent>("post-meeting-prompt", (event) => {
      setState({ visible: true, meeting: event.payload, isFallback: false });
    });

    // Fallback prompt (no transcript detected after deadline)
    const unlistenFallback = listen<CalendarEvent>(
      "post-meeting-prompt-fallback",
      (event) => {
        setState({ visible: true, meeting: event.payload, isFallback: true });
      }
    );

    return () => {
      unlistenFull.then((fn) => fn());
      unlistenFallback.then((fn) => fn());
    };
  }, []);

  const capture = useCallback(
    async (outcome: CapturedOutcome) => {
      try {
        await invoke("capture_meeting_outcome", { outcome });
      } catch (err) {
        console.error("Failed to capture outcome:", err);
      }
      setState({ visible: false, meeting: null, isFallback: false });
    },
    []
  );

  const skip = useCallback(async () => {
    if (state.meeting) {
      try {
        await invoke("dismiss_meeting_prompt", {
          meetingId: state.meeting.id,
        });
      } catch (err) {
        console.error("Failed to dismiss prompt:", err);
      }
    }
    setState({ visible: false, meeting: null, isFallback: false });
  }, [state.meeting]);

  const dismiss = useCallback(() => {
    setState({ visible: false, meeting: null, isFallback: false });
  }, []);

  return {
    visible: state.visible,
    meeting: state.meeting,
    isFallback: state.isFallback,
    capture,
    skip,
    dismiss,
  };
}
