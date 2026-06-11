import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TimelineMeeting } from "@/types";

export function useWeekCommands() {
  const getMeetingTimeline = useCallback((daysBefore: number, daysAfter: number) => {
    return invoke<TimelineMeeting[]>("get_meeting_timeline", {
      daysBefore,
      daysAfter,
    });
  }, []);

  const refreshMeetingPreps = useCallback(() => {
    return invoke("refresh_meeting_preps");
  }, []);

  return useMemo(() => ({
    getMeetingTimeline,
    refreshMeetingPreps,
  }), [getMeetingTimeline, refreshMeetingPreps]);
}
