/**
 * entity-utils.ts — Shared formatting utilities for entity editorial pages.
 * Extracted from account/TheWork.tsx to avoid triple-maintaining across
 * account, project, and person pages.
 */
import { parseDate } from "@/lib/utils";

/** Format a date string as "Feb 18 Tue". */
export function formatMeetingRowDate(dateStr: string): string {
  const date = parseDate(dateStr);
  if (!date) return dateStr;
  const month = date.toLocaleDateString(undefined, { month: "short" });
  const day = date.getDate();
  const weekday = date.toLocaleDateString(undefined, { weekday: "short" });
  return `${month} ${day} ${weekday}`;
}

/** Return a badge style for a meeting type. */
export function meetingTypeBadgeStyle(meetingType: string): React.CSSProperties {
  const base: React.CSSProperties = {
    fontFamily: "var(--font-mono)",
    fontSize: 9,
    fontWeight: 500,
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    padding: "2px 7px",
    borderRadius: 3,
    whiteSpace: "nowrap",
  };

  if (meetingType === "customer" || meetingType === "qbr" || meetingType === "training") {
    return { ...base, background: "rgba(201,162,39,0.10)", color: "var(--color-spice-turmeric)" };
  }
  if (meetingType === "internal" || meetingType === "team_sync" || meetingType === "one_on_one") {
    return { ...base, background: "rgba(143,163,196,0.12)", color: "var(--color-garden-larkspur)" };
  }
  return { ...base, background: "rgba(30,37,48,0.06)", color: "var(--color-text-tertiary)" };
}

/** Classify an action as overdue, this-week, or upcoming based on due date. */
export function classifyAction(
  action: { dueDate?: string },
  now: Date,
): "overdue" | "this-week" | "upcoming" | "no-date" {
  if (!action.dueDate) return "no-date";
  const due = parseDate(action.dueDate);
  if (!due) return "no-date";

  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (due < startOfToday) return "overdue";

  const sevenDaysOut = new Date(startOfToday);
  sevenDaysOut.setDate(sevenDaysOut.getDate() + 7);
  if (due < sevenDaysOut) return "this-week";

  return "upcoming";
}
