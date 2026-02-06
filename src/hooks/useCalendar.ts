import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CalendarEvent } from "@/types";

export function useCalendar() {
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [now, setNow] = useState(Date.now());

  // Fetch events on mount and when calendar-updated fires
  useEffect(() => {
    invoke<CalendarEvent[]>("get_calendar_events").then(setEvents).catch(() => {});

    const unlisten = listen("calendar-updated", () => {
      invoke<CalendarEvent[]>("get_calendar_events").then(setEvents).catch(() => {});
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

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
