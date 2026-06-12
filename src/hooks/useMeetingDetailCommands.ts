import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyPrepPrefillResult,
  CalendarEvent,
  ContinuityThread,
  MeetingIntelligence,
  MeetingPostIntelligence,
  PredictionScorecard,
} from "@/types";

export interface GranolaManualSyncResult {
  status: "attached" | "not_found" | "already_in_progress" | "already_completed";
  message: string;
  documentTitle?: string;
  contentType?: "transcript" | "notes";
}

export interface MeetingBriefingRefreshResult {
  meetingId: string;
  refreshedEntities: number;
  failedEntities: number;
  failedEntityIds: string[];
  prepRebuiltSync: boolean;
  prepQueued: boolean;
}

export interface MeetingCompositionTokenResponse {
  meetingToken: string;
  compositionId: string;
}

interface TranscriptAttachResult {
  status: string;
  message?: string;
  summary?: string;
}

interface UpdateMeetingUserAgendaRequest {
  agenda?: string[];
  dismissedTopics?: string[] | null;
  hiddenAttendees?: string[];
}

export function useMeetingDetailCommands(meetingId: string | null | undefined) {
  const requireMeetingId = useCallback(() => {
    if (!meetingId) {
      throw new Error("No meeting ID specified");
    }
    return meetingId;
  }, [meetingId]);

  const triggerGranolaSyncForMeeting = useCallback((force: boolean) => {
    return invoke<GranolaManualSyncResult>("trigger_granola_sync_for_meeting", {
      meetingId: requireMeetingId(),
      force,
    });
  }, [requireMeetingId]);

  const getMeetingIntelligence = useCallback(() => {
    return invoke<MeetingIntelligence>("get_meeting_intelligence", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const getMeetingCompositionToken = useCallback(() => {
    return invoke<MeetingCompositionTokenResponse>("get_meeting_composition_token", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const markMeetingIntelligenceViewed = useCallback(() => {
    return invoke("mark_meeting_intelligence_viewed", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const getMeetingPostIntelligence = useCallback(() => {
    return invoke<MeetingPostIntelligence>("get_meeting_post_intelligence", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const getMeetingContinuityThread = useCallback(() => {
    return invoke<ContinuityThread | null>("get_meeting_continuity_thread", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const getPredictionScorecard = useCallback(() => {
    return invoke<PredictionScorecard | null>("get_prediction_scorecard", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const triggerQuillSyncForMeeting = useCallback((force: boolean) => {
    return invoke<string>("trigger_quill_sync_for_meeting", {
      meetingId: requireMeetingId(),
      force,
    });
  }, [requireMeetingId]);

  const attachMeetingTranscript = useCallback((filePath: string, meeting: CalendarEvent) => {
    return invoke<TranscriptAttachResult>("attach_meeting_transcript", {
      filePath,
      meeting,
    });
  }, []);

  const attachMeetingTranscriptText = useCallback((
    text: string,
    format: "txt" | "md",
    meeting: CalendarEvent,
  ) => {
    return invoke<TranscriptAttachResult>("attach_meeting_transcript_text", {
      text,
      format,
      meeting,
    });
  }, []);

  const reprocessMeetingTranscript = useCallback(() => {
    return invoke<TranscriptAttachResult>("reprocess_meeting_transcript", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const applyMeetingPrepPrefill = useCallback((agendaItems: string[], notesAppend: string) => {
    return invoke<ApplyPrepPrefillResult>("apply_meeting_prep_prefill", {
      meetingId: requireMeetingId(),
      agendaItems,
      notesAppend,
    });
  }, [requireMeetingId]);

  const refreshMeetingBriefing = useCallback(() => {
    return invoke<MeetingBriefingRefreshResult>("refresh_meeting_briefing", {
      meetingId: requireMeetingId(),
    });
  }, [requireMeetingId]);

  const getQuillStatus = useCallback(() => {
    return invoke<{ enabled: boolean }>("get_quill_status");
  }, []);

  const getGranolaStatus = useCallback(() => {
    return invoke<{ enabled: boolean }>("get_granola_status");
  }, []);

  const updateMeetingPrepField = useCallback((
    fieldPath: string,
    value: string,
    targetPersonId?: string,
  ) => {
    return invoke("update_meeting_prep_field", {
      meetingId: requireMeetingId(),
      fieldPath,
      value,
      targetPersonId: targetPersonId ?? null,
    });
  }, [requireMeetingId]);

  const updateMeetingUserAgenda = useCallback((request: UpdateMeetingUserAgendaRequest) => {
    return invoke("update_meeting_user_agenda", {
      meetingId: requireMeetingId(),
      ...request,
    });
  }, [requireMeetingId]);

  const acceptSuggestedAction = useCallback((id: string) => {
    return invoke("accept_suggested_action", { id });
  }, []);

  const rejectSuggestedAction = useCallback((id: string) => {
    return invoke("reject_suggested_action", { id, source: "meeting_detail" });
  }, []);

  const reopenAction = useCallback((id: string) => {
    return invoke("reopen_action", { id });
  }, []);

  const completeAction = useCallback((id: string) => {
    return invoke("complete_action", { id });
  }, []);

  const updateActionPriority = useCallback((id: string, priority: string) => {
    return invoke("update_action_priority", { id, priority });
  }, []);

  return useMemo(() => ({
    acceptSuggestedAction,
    applyMeetingPrepPrefill,
    attachMeetingTranscript,
    attachMeetingTranscriptText,
    completeAction,
    getGranolaStatus,
    getMeetingCompositionToken,
    getMeetingContinuityThread,
    getMeetingIntelligence,
    getMeetingPostIntelligence,
    getPredictionScorecard,
    getQuillStatus,
    markMeetingIntelligenceViewed,
    refreshMeetingBriefing,
    rejectSuggestedAction,
    reopenAction,
    reprocessMeetingTranscript,
    triggerGranolaSyncForMeeting,
    triggerQuillSyncForMeeting,
    updateActionPriority,
    updateMeetingPrepField,
    updateMeetingUserAgenda,
  }), [
    acceptSuggestedAction,
    applyMeetingPrepPrefill,
    attachMeetingTranscript,
    attachMeetingTranscriptText,
    completeAction,
    getGranolaStatus,
    getMeetingCompositionToken,
    getMeetingContinuityThread,
    getMeetingIntelligence,
    getMeetingPostIntelligence,
    getPredictionScorecard,
    getQuillStatus,
    markMeetingIntelligenceViewed,
    refreshMeetingBriefing,
    rejectSuggestedAction,
    reopenAction,
    reprocessMeetingTranscript,
    triggerGranolaSyncForMeeting,
    triggerQuillSyncForMeeting,
    updateActionPriority,
    updateMeetingPrepField,
    updateMeetingUserAgenda,
  ]);
}
