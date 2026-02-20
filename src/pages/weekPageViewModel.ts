import type {
  DayShape,
  LiveProactiveSuggestion,
  MeetingType,
  ReadinessCheck,
  TimeBlock,
  TopPriority,
  WeekAction,
  WeekDay,
  WeekMeeting,
} from "@/types";

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

// =============================================================================
// Editorial Week Forecast helpers
// =============================================================================

/** An item in the "Top Three" priorities section. */
export interface TopThreeItem {
  number: 1 | 2 | 3;
  title: string;
  reason: string;
  contextLine: string;
  actionId?: string;
  meetingId?: string;
}

/**
 * Pick exactly 3 priorities for "The Three" chapter.
 * Item 1 is always the AI topPriority. Items 2-3 are the most urgent
 * remaining actions scored by: overdue severity + meeting proximity.
 */
export function pickTopThree(
  topPriority: TopPriority | undefined,
  overdue: WeekAction[],
  dueThisWeek: WeekAction[],
  liveSuggestions: LiveProactiveSuggestion[],
  days?: WeekDay[]
): TopThreeItem[] {
  const items: TopThreeItem[] = [];

  // Item 1: AI top priority (if available)
  if (topPriority) {
    items.push({
      number: 1,
      title: topPriority.title,
      reason: topPriority.reason,
      contextLine: topPriority.actionId
        ? "Action"
        : topPriority.meetingId
          ? "Meeting"
          : "",
      actionId: topPriority.actionId,
      meetingId: topPriority.meetingId,
    });
  }

  // Candidates pool: overdue first (highest severity), then due-this-week by priority
  const candidates: {
    title: string;
    reason: string;
    contextLine: string;
    actionId?: string;
    meetingId?: string;
    score: number;
  }[] = [];

  for (const a of overdue) {
    const severity = (a.daysOverdue ?? 1) * 10;
    const priorityScore = a.priority === "P1" ? 30 : a.priority === "P2" ? 20 : 10;
    candidates.push({
      title: a.title,
      reason: a.daysOverdue
        ? `${a.daysOverdue} day${a.daysOverdue !== 1 ? "s" : ""} overdue.`
        : "Overdue.",
      contextLine: [a.account, formatDueContext(a.dueDate, a.daysOverdue)]
        .filter(Boolean)
        .join(" \u00b7 "),
      actionId: a.id,
      score: severity + priorityScore,
    });
  }

  for (const a of dueThisWeek) {
    const priorityScore = a.priority === "P1" ? 30 : a.priority === "P2" ? 20 : 10;
    candidates.push({
      title: a.title,
      reason: formatDueContext(a.dueDate) ?? "Due this week.",
      contextLine: [a.account, formatDueContext(a.dueDate)]
        .filter(Boolean)
        .join(" \u00b7 "),
      actionId: a.id,
      score: priorityScore,
    });
  }

  // Add live suggestions that reference meetings (not already covered)
  for (const s of liveSuggestions) {
    if (s.meetingId && !candidates.some((c) => c.actionId === s.actionId)) {
      candidates.push({
        title: s.title,
        reason: s.reason,
        contextLine: `${s.day} \u00b7 ${formatBlockRange(s.start, s.end)}`,
        actionId: s.actionId,
        score: s.totalScore * 10,
      });
    }
  }

  // Fallback: when actions are empty, fill from key external meetings
  if (candidates.length === 0 && days) {
    for (const day of days) {
      for (const m of day.meetings) {
        if (!EXTERNAL_MEETING_TYPES.has(m.type)) continue;
        candidates.push({
          title: m.title,
          reason: `${day.dayName}${meetingEntityLabel(m) ? ` \u2014 ${meetingEntityLabel(m)}` : ""}`,
          contextLine: `${day.dayName} ${m.time}`,
          meetingId: m.meetingId,
          score: m.type === "customer" || m.type === "qbr" ? 20 : 10,
        });
      }
    }
  }

  // Sort by score descending, fill remaining slots
  candidates.sort((a, b) => b.score - a.score);
  const usedTitles = new Set(items.map((i) => i.title));

  for (const c of candidates) {
    if (items.length >= 3) break;
    if (usedTitles.has(c.title)) continue;
    usedTitles.add(c.title);
    items.push({
      number: (items.length + 1) as 1 | 2 | 3,
      title: c.title,
      reason: c.reason,
      contextLine: c.contextLine,
      actionId: c.actionId,
      meetingId: c.meetingId,
    });
  }

  return items.slice(0, 3);
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

/** Derive a display label from a WeekMeeting's linked entities, falling back to account string (I339). */
function meetingEntityLabel(m: WeekMeeting): string | undefined {
  if (m.linkedEntities?.length) {
    return m.linkedEntities.map((e) => e.name).join(", ");
  }
  return m.account;
}

/** External meeting types for filtering. */
const EXTERNAL_MEETING_TYPES: Set<MeetingType> = new Set([
  "customer",
  "qbr",
  "partnership",
  "external",
]);


/** Synthesize readiness into FolioBar stats. */
export function synthesizeReadinessStats(
  checks: ReadinessCheck[]
): { preppedLabel: string; overdueLabel: string | null } {
  const totalExternal = checks.filter(
    (c) => c.checkType !== "overdue_action" && c.checkType !== "stale_contact"
  );
  const needsPrep = checks.filter(
    (c) =>
      c.checkType === "no_prep" ||
      c.checkType === "prep_needed" ||
      c.checkType === "agenda_needed"
  ).length;

  const prepped = totalExternal.length - needsPrep;
  const preppedLabel = `${prepped}/${totalExternal.length} prepped`;

  const overdueActions = checks.filter(
    (c) => c.checkType === "overdue_action"
  );
  let overdueLabel: string | null = null;
  if (overdueActions.length > 0) {
    const msg = overdueActions[0].message;
    const match = msg.match(/^(\d+)/);
    const count = match ? match[1] : overdueActions.length.toString();
    overdueLabel = `${count} overdue`;
  }

  return { preppedLabel, overdueLabel };
}

