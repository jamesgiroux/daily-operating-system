import type { ReadinessCheck, TimeBlock } from "@/types";

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

/** Synthesize readiness checks into a one-line summary */
export function synthesizeReadiness(checks: ReadinessCheck[]): string {
  const prepNeeded = checks.filter(
    (c) =>
      c.checkType === "no_prep" ||
      c.checkType === "prep_needed" ||
      c.checkType === "agenda_needed"
  ).length;
  const overdueActions = checks.filter(
    (c) => c.checkType === "overdue_action"
  );
  const staleContacts = checks.filter(
    (c) => c.checkType === "stale_contact"
  ).length;

  const parts: string[] = [];
  if (prepNeeded > 0)
    parts.push(
      `${prepNeeded} meeting${prepNeeded !== 1 ? "s" : ""} need prep`
    );
  if (overdueActions.length > 0) {
    const msg = overdueActions[0].message;
    const match = msg.match(/^(\d+)/);
    const count = match ? match[1] : overdueActions.length.toString();
    parts.push(`${count} overdue action${count !== "1" ? "s" : ""}`);
  }
  if (staleContacts > 0)
    parts.push(
      `${staleContacts} stale contact${staleContacts !== 1 ? "s" : ""}`
    );
  return parts.join(" · ");
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

export type SuggestionLinkTarget =
  | { kind: "action"; id: string }
  | { kind: "meeting"; id: string }
  | { kind: "none" };

export function resolveSuggestionLink(
  actionId?: string | null,
  meetingId?: string | null
): SuggestionLinkTarget {
  if (actionId) return { kind: "action", id: actionId };
  if (meetingId) return { kind: "meeting", id: meetingId };
  return { kind: "none" };
}

export function classifyWeekShapeState(
  blocks: TimeBlock[]
): "no_blocks" | "no_suggestions" | "suggestions" {
  if (blocks.length === 0) return "no_blocks";
  const hasSuggestions = blocks.some((block) => !!block.suggestedUse?.trim());
  return hasSuggestions ? "suggestions" : "no_suggestions";
}
