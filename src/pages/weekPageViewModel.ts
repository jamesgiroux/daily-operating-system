import type {
  DayShape,
  TimelineMeeting,
} from "@/types";

// =============================================================================
// Week header + shape utilities (redesign)
// =============================================================================

/** Compute week number and date range from current date. */
export function computeWeekMeta(): { weekNumber: string; dateRange: string } {
  const now = new Date();
  // Find Monday of the current week
  const day = now.getDay(); // 0=Sun, 1=Mon, ...
  const diffToMon = day === 0 ? -6 : 1 - day;
  const monday = new Date(now);
  monday.setDate(now.getDate() + diffToMon);
  monday.setHours(0, 0, 0, 0);

  const friday = new Date(monday);
  friday.setDate(monday.getDate() + 4);

  // ISO week number: week 1 contains the first Thursday of the year
  const jan4 = new Date(monday.getFullYear(), 0, 4);
  const jan4Day = jan4.getDay() || 7; // Mon=1..Sun=7
  const isoWeek1Mon = new Date(jan4);
  isoWeek1Mon.setDate(jan4.getDate() - (jan4Day - 1));
  const weekNumber = String(
    Math.ceil(
      (monday.getTime() - isoWeek1Mon.getTime()) / (7 * 24 * 60 * 60 * 1000) + 1
    )
  );

  const fmt = (d: Date) =>
    d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  const dateRange = `${fmt(monday)} – ${fmt(friday)}`;

  return { weekNumber, dateRange };
}

/** Derive DayShape[] for Mon–Fri of the current week from timeline meetings. */
export function deriveShapeFromTimeline(
  timeline: TimelineMeeting[]
): DayShape[] {
  const now = new Date();
  const day = now.getDay();
  const diffToMon = day === 0 ? -6 : 1 - day;
  const monday = new Date(now);
  monday.setDate(now.getDate() + diffToMon);
  monday.setHours(0, 0, 0, 0);

  const dayNames = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];
  const result: DayShape[] = [];

  for (let i = 0; i < 5; i++) {
    const d = new Date(monday);
    d.setDate(monday.getDate() + i);
    const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

    const dayMeetings = timeline.filter(
      (m) => m.startTime.slice(0, 10) === dateStr
    );
    const count = dayMeetings.length;

    let totalMinutes = 0;
    for (const m of dayMeetings) {
      if (m.endTime) {
        const start = new Date(m.startTime).getTime();
        const end = new Date(m.endTime).getTime();
        const mins = (end - start) / 60000;
        totalMinutes += mins > 0 ? mins : 45;
      } else {
        totalMinutes += 45;
      }
    }

    const density =
      count >= 5
        ? "packed"
        : count >= 4
          ? "heavy"
          : count >= 2
            ? "moderate"
            : "light";

    result.push({
      date: dateStr,
      dayName: dayNames[i],
      meetingCount: count,
      meetingMinutes: totalMinutes,
      density,
      meetings: [],
      availableBlocks: [],
    });
  }

  return result;
}

// =============================================================================
// Kept utilities
// =============================================================================

/** Format a due date as a relative phrase: "due Wednesday", "1 day overdue" */
export function formatDueContext(
  dueDate?: string,
  daysOverdue?: number
): string | null {
  if (!dueDate) return null;

  if (daysOverdue != null && daysOverdue > 0) {
    return daysOverdue === 1 ? "1 day overdue" : `${daysOverdue} days overdue`;
  }

  try {
    const date = new Date(dueDate + "T00:00:00");
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    const diffMs = date.getTime() - now.getTime();
    const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays < 0)
      return `${Math.abs(diffDays)} day${Math.abs(diffDays) !== 1 ? "s" : ""} overdue`;
    if (diffDays === 0) return "due today";
    if (diffDays === 1) return "due tomorrow";
    if (diffDays <= 6)
      return `due ${date.toLocaleDateString("en-US", { weekday: "long" })}`;
    return `due ${date.toLocaleDateString("en-US", { month: "short", day: "numeric" })}`;
  } catch {
    return null;
  }
}

export function formatBlockRange(start: string, end: string): string {
  const fmt = (value: string) => {
    if (!value) return "";
    if (value.includes("T")) {
      const dt = new Date(value);
      if (!Number.isNaN(dt.getTime())) {
        return dt.toLocaleTimeString("en-US", {
          hour: "numeric",
          minute: "2-digit",
        });
      }
    }
    if (/^\d{2}:\d{2}$/.test(value)) {
      const [h, m] = value.split(":").map((n) => Number.parseInt(n, 10));
      if (!Number.isNaN(h) && !Number.isNaN(m)) {
        const dt = new Date();
        dt.setHours(h, m, 0, 0);
        return dt.toLocaleTimeString("en-US", {
          hour: "numeric",
          minute: "2-digit",
        });
      }
    }
    return value;
  };

  const s = fmt(start);
  const e = fmt(end);
  if (s && e) return `${s} - ${e}`;
  return s || e || "Open block";
}

/** Compute an editorial epigraph for The Shape chapter. */
export function computeShapeEpigraph(dayShapes: DayShape[]): string {
  if (dayShapes.length === 0) return "";

  // Find the busiest day
  const sorted = [...dayShapes].sort(
    (a, b) => b.meetingMinutes - a.meetingMinutes
  );
  const busiest = sorted[0];
  const lightest = sorted[sorted.length - 1];

  // Classify the shape
  const frontHalf = dayShapes.slice(0, Math.ceil(dayShapes.length / 2));
  const backHalf = dayShapes.slice(Math.ceil(dayShapes.length / 2));
  const frontLoad = frontHalf.reduce((s, d) => s + d.meetingMinutes, 0);
  const backLoad = backHalf.reduce((s, d) => s + d.meetingMinutes, 0);

  let shape = "Balanced";
  if (frontLoad > backLoad * 1.5) shape = "Front-loaded";
  else if (backLoad > frontLoad * 1.5) shape = "Back-loaded";

  const crux = busiest.dayName;
  const recovery =
    lightest.meetingCount <= 1 ? ` Clear ${lightest.dayName} for recovery.` : "";

  return `${shape}. ${crux} is the crux${recovery ? " \u2014" + recovery : "."}`;
}
