import { describe, expect, it } from "vitest";

import {
  computeWeekMeta,
  deriveShapeFromTimeline,
  formatBlockRange,
  formatDueContext,
} from "./weekPageViewModel";

import type { TimelineMeeting } from "@/types";

/** Format a Date as YYYY-MM-DD in local time (matching formatDueContext's parsing). */
function toLocalIso(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** Create a minimal TimelineMeeting for testing. */
function makeMeeting(
  startTime: string,
  endTime?: string
): TimelineMeeting {
  return {
    id: `m-${startTime}`,
    title: "Test Meeting",
    startTime,
    endTime,
    meetingType: "internal",
    hasOutcomes: false,
    entities: [],
    hasNewSignals: false,
  };
}

describe("computeWeekMeta", () => {
  it("returns a week number and date range", () => {
    const meta = computeWeekMeta();
    expect(meta.weekNumber).toMatch(/^\d+$/);
    expect(meta.dateRange).toMatch(/\w+ \d+ – \w+ \d+/);
  });

  it("week number is a reasonable value (1–53)", () => {
    const num = Number(computeWeekMeta().weekNumber);
    expect(num).toBeGreaterThanOrEqual(1);
    expect(num).toBeLessThanOrEqual(53);
  });
});

describe("deriveShapeFromTimeline", () => {
  it("always returns exactly 5 entries (Mon–Fri)", () => {
    const result = deriveShapeFromTimeline([]);
    expect(result).toHaveLength(5);
    expect(result.map((d) => d.dayName)).toEqual([
      "Monday",
      "Tuesday",
      "Wednesday",
      "Thursday",
      "Friday",
    ]);
  });

  it("zero-meeting days have light density and 0 minutes", () => {
    const result = deriveShapeFromTimeline([]);
    for (const day of result) {
      expect(day.meetingCount).toBe(0);
      expect(day.meetingMinutes).toBe(0);
      expect(day.density).toBe("light");
    }
  });

  it("groups meetings by date and classifies density", () => {
    // Find Monday of the current week
    const now = new Date();
    const dayOfWeek = now.getDay();
    const diffToMon = dayOfWeek === 0 ? -6 : 1 - dayOfWeek;
    const monday = new Date(now);
    monday.setDate(now.getDate() + diffToMon);
    monday.setHours(9, 0, 0, 0);
    const monStr = toLocalIso(monday);

    // 3 meetings on Monday → moderate
    const meetings: TimelineMeeting[] = [
      makeMeeting(`${monStr}T09:00:00`, `${monStr}T10:00:00`),
      makeMeeting(`${monStr}T11:00:00`, `${monStr}T12:00:00`),
      makeMeeting(`${monStr}T14:00:00`, `${monStr}T15:00:00`),
    ];

    const result = deriveShapeFromTimeline(meetings);
    const mondayShape = result[0];
    expect(mondayShape.meetingCount).toBe(3);
    expect(mondayShape.meetingMinutes).toBe(180); // 3 × 60min
    expect(mondayShape.density).toBe("moderate");
  });

  it("falls back to 45min estimate when endTime is missing", () => {
    const now = new Date();
    const dayOfWeek = now.getDay();
    const diffToMon = dayOfWeek === 0 ? -6 : 1 - dayOfWeek;
    const monday = new Date(now);
    monday.setDate(now.getDate() + diffToMon);
    const monStr = toLocalIso(monday);

    const meetings: TimelineMeeting[] = [
      makeMeeting(`${monStr}T09:00:00`), // no endTime
    ];

    const result = deriveShapeFromTimeline(meetings);
    expect(result[0].meetingMinutes).toBe(45);
  });

  it("classifies 5+ meetings as packed", () => {
    const now = new Date();
    const dayOfWeek = now.getDay();
    const diffToMon = dayOfWeek === 0 ? -6 : 1 - dayOfWeek;
    const monday = new Date(now);
    monday.setDate(now.getDate() + diffToMon);
    const monStr = toLocalIso(monday);

    const meetings: TimelineMeeting[] = Array.from({ length: 5 }, (_, i) =>
      makeMeeting(`${monStr}T${String(9 + i).padStart(2, "0")}:00:00`)
    );

    const result = deriveShapeFromTimeline(meetings);
    expect(result[0].density).toBe("packed");
  });
});

describe("formatBlockRange", () => {
  it("formats block ranges from HH:mm inputs", () => {
    const value = formatBlockRange("09:00", "10:30");
    expect(value).toMatch(/-/);
    expect(value).toContain(":");
  });

  it("formats ISO datetime strings", () => {
    const value = formatBlockRange(
      "2026-02-13T09:00:00+00:00",
      "2026-02-13T10:30:00+00:00"
    );
    expect(value).toMatch(/-/);
    expect(value).toContain(":");
  });

  it("returns 'Open block' for empty strings", () => {
    expect(formatBlockRange("", "")).toBe("Open block");
  });

  it("returns partial range when only start is provided", () => {
    const value = formatBlockRange("09:00", "");
    expect(value).toContain(":");
    expect(value).not.toMatch(/-/);
  });

  it("returns partial range when only end is provided", () => {
    const value = formatBlockRange("", "10:30");
    expect(value).toContain(":");
    expect(value).not.toMatch(/-/);
  });

  it("passes through unrecognized formats as-is", () => {
    expect(formatBlockRange("morning", "afternoon")).toBe(
      "morning - afternoon"
    );
  });
});

describe("formatDueContext", () => {
  it("formats overdue context when daysOverdue is provided", () => {
    expect(formatDueContext("2026-02-10", 2)).toBe("2 days overdue");
  });

  it("formats singular day overdue", () => {
    expect(formatDueContext("2026-02-10", 1)).toBe("1 day overdue");
  });

  it("returns null when no due date", () => {
    expect(formatDueContext(undefined, undefined)).toBeNull();
  });

  it("formats 'due today'", () => {
    const today = new Date();
    today.setHours(12, 0, 0, 0); // midday to avoid timezone edge
    expect(formatDueContext(toLocalIso(today), undefined)).toBe("due today");
  });

  it("formats 'due tomorrow'", () => {
    const tomorrow = new Date();
    tomorrow.setHours(12, 0, 0, 0);
    tomorrow.setDate(tomorrow.getDate() + 1);
    expect(formatDueContext(toLocalIso(tomorrow), undefined)).toBe(
      "due tomorrow"
    );
  });

  it("formats day name for near-future dates", () => {
    const future = new Date();
    future.setHours(12, 0, 0, 0);
    future.setDate(future.getDate() + 3);
    const result = formatDueContext(toLocalIso(future), undefined);
    expect(result).toMatch(
      /^due (Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)$/
    );
  });

  it("formats month+day for far-future dates", () => {
    const future = new Date();
    future.setHours(12, 0, 0, 0);
    future.setDate(future.getDate() + 14);
    const result = formatDueContext(toLocalIso(future), undefined);
    expect(result).toMatch(/^due \w+ \d+$/);
  });

  it("formats overdue from date diff when daysOverdue is not provided", () => {
    const past = new Date();
    past.setHours(12, 0, 0, 0);
    past.setDate(past.getDate() - 5);
    const result = formatDueContext(toLocalIso(past), undefined);
    expect(result).toMatch(/overdue$/);
  });

  it("prioritizes daysOverdue over date computation", () => {
    // Future date but daysOverdue says 3
    const future = new Date();
    future.setDate(future.getDate() + 5);
    const isoDate = future.toISOString().split("T")[0];
    expect(formatDueContext(isoDate, 3)).toBe("3 days overdue");
  });

  it("ignores daysOverdue of 0 and falls through to date logic", () => {
    const today = new Date();
    today.setHours(12, 0, 0, 0);
    expect(formatDueContext(toLocalIso(today), 0)).toBe("due today");
  });
});
