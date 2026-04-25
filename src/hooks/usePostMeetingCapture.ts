import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type {
  CalendarEvent,
  CapturedOutcome,
  TranscriptResult,
} from "@/types";
import { useTauriEvent } from "./useTauriEvent";

interface CaptureState {
  visible: boolean;
  meeting: CalendarEvent | null;
  isFallback: boolean;
  /** Set when transcript is being processed */
  processing: boolean;
}

export function usePostMeetingCapture() {
  const [state, setState] = useState<CaptureState>({
    visible: false,
    meeting: null,
    isFallback: false,
    processing: false,
  });

  const handleFullPrompt = useCallback((meeting: CalendarEvent) => {
    setState({ visible: true, meeting, isFallback: false, processing: false });
  }, []);

  const handleFallbackPrompt = useCallback((meeting: CalendarEvent) => {
    setState({ visible: true, meeting, isFallback: true, processing: false });
  }, []);

  // Full capture prompt (manual trigger or auto with transcript)
  useTauriEvent("post-meeting-prompt", handleFullPrompt);

  // Fallback prompt (no transcript detected after deadline)
  useTauriEvent("post-meeting-prompt-fallback", handleFallbackPrompt);

  const capture = useCallback(
    async (outcome: CapturedOutcome) => {
      try {
        await invoke("capture_meeting_outcome", { outcome });
      } catch (err) {
        console.error("Failed to capture outcome:", err);
        toast.error("Failed to save meeting outcome");
      }
      setState({ visible: false, meeting: null, isFallback: false, processing: false });
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
        console.error("Failed to dismiss prompt:", err); // Expected: best-effort dismiss
      }
    }
    setState({ visible: false, meeting: null, isFallback: false, processing: false });
  }, [state.meeting]);

  const dismiss = useCallback(() => {
    setState({ visible: false, meeting: null, isFallback: false, processing: false });
  }, []);

  const attachTranscript = useCallback(
    async (filePath: string): Promise<TranscriptResult | null> => {
      if (!state.meeting) return null;
      setState((prev) => ({ ...prev, processing: true }));
      try {
        const result = await invoke<TranscriptResult>(
          "attach_meeting_transcript",
          { filePath, meeting: state.meeting }
        );
        // Brief delay to show success before dismissing
        setTimeout(() => {
          setState({ visible: false, meeting: null, isFallback: false, processing: false });
        }, 2000);
        return result;
      } catch (err) {
        console.error("Failed to attach transcript:", err);
        toast.error("Failed to attach transcript");
        setState((prev) => ({ ...prev, processing: false }));
        return null;
      }
    },
    [state.meeting]
  );

  const pasteTranscript = useCallback(
    async (
      text: string,
      format: "txt" | "md" = "txt",
    ): Promise<TranscriptResult | null> => {
      if (!state.meeting) return null;
      const trimmed = text.trim();
      if (!trimmed) {
        toast.error("Paste a transcript first");
        return null;
      }
      setState((prev) => ({ ...prev, processing: true }));
      try {
        const result = await invoke<TranscriptResult>(
          "attach_meeting_transcript_text",
          { text: trimmed, format, meeting: state.meeting },
        );
        setTimeout(() => {
          setState({ visible: false, meeting: null, isFallback: false, processing: false });
        }, 2000);
        return result;
      } catch (err) {
        console.error("Failed to paste transcript:", err);
        toast.error("Failed to paste transcript");
        setState((prev) => ({ ...prev, processing: false }));
        return null;
      }
    },
    [state.meeting],
  );

  return {
    visible: state.visible,
    meeting: state.meeting,
    isFallback: state.isFallback,
    processing: state.processing,
    capture,
    skip,
    dismiss,
    attachTranscript,
    pasteTranscript,
  };
}
