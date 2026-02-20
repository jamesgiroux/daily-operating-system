import { useState, useEffect, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "./useTauriEvent";
import type { CalendarEvent } from "@/types";

export function useCalendar() {
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [now, setNow] = useState(Date.now());

  const fetchEvents = useCallback(() => {
    invoke<CalendarEvent[]>("get_calendar_events").then(setEvents).catch((err) => {
      console.error("get_calendar_events failed:", err);
    });
  }, []);

  // Fetch events on mount
  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  // Re-fetch when calendar-updated fires
  useTauriEvent("calendar-updated", fetchEvents);

  // Client-side 30-second interval to re-evaluate current/next meeting
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(interval);
  }, []);

  const currentMeeting = useMemo(() => {
    return events.find((e) => {
      const start = new Date(e.start).getTime();
      const end = new Date(e.end).getTime();
      return start <= now && end > now && !e.isAllDay;
    });
  }, [events, now]);

  const nextMeeting = useMemo(() => {
    return events
      .filter((e) => new Date(e.start).getTime() > now && !e.isAllDay)
      .sort((a, b) => new Date(a.start).getTime() - new Date(b.start).getTime())[0];
  }, [events, now]);

  return { events, currentMeeting, nextMeeting, now };
}
