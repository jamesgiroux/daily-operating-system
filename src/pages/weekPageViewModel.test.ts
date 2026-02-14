import { describe, expect, it } from "vitest";

import {
  classifyWeekShapeState,
  formatBlockRange,
  formatDueContext,
  resolveSuggestionLink,
  synthesizeReadiness,
} from "./weekPageViewModel";

/** Format a Date as YYYY-MM-DD in local time (matching formatDueContext's parsing). */
function toLocalIso(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

describe("resolveSuggestionLink", () => {
  it("prefers action links over meeting links", () => {
    expect(resolveSuggestionLink("a-123", "m-123")).toEqual({
      kind: "action",
      id: "a-123",
    });
  });

  it("falls back to meeting link when no action id exists", () => {
    expect(resolveSuggestionLink(undefined, "m-123")).toEqual({
      kind: "meeting",
      id: "m-123",
    });
  });

  it("returns none when both are undefined", () => {
    expect(resolveSuggestionLink(undefined, undefined)).toEqual({
      kind: "none",
    });
  });

  it("returns none when both are null", () => {
    expect(resolveSuggestionLink(null, null)).toEqual({ kind: "none" });
  });

  it("ignores null action and uses meeting", () => {
    expect(resolveSuggestionLink(null, "m-456")).toEqual({
      kind: "meeting",
      id: "m-456",
    });
  });
});

describe("classifyWeekShapeState", () => {
  it("classifies week shape empty states", () => {
    expect(classifyWeekShapeState([])).toBe("no_blocks");
    expect(
      classifyWeekShapeState([
        {
          day: "Monday",
          start: "09:00",
          end: "10:00",
          durationMinutes: 60,
          suggestedUse: "",
        },
      ])
    ).toBe("no_suggestions");
    expect(
      classifyWeekShapeState([
        {
          day: "Monday",
          start: "09:00",
          end: "10:00",
          durationMinutes: 60,
          suggestedUse: "Prep roadmap update",
        },
      ])
    ).toBe("suggestions");
  });

  it("treats whitespace-only suggestedUse as no suggestion", () => {
    expect(
      classifyWeekShapeState([
        {
          day: "Monday",
          start: "09:00",
          end: "10:00",
          durationMinutes: 60,
          suggestedUse: "   ",
        },
      ])
    ).toBe("no_suggestions");
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

describe("synthesizeReadiness", () => {
  it("synthesizes readiness summary text", () => {
    const summary = synthesizeReadiness([
      {
        checkType: "no_prep",
        severity: "action_needed",
        message: "Needs prep",
      },
      {
        checkType: "overdue_action",
        severity: "action_needed",
        message: "3 overdue actions",
      },
      {
        checkType: "stale_contact",
        severity: "heads_up",
        message: "No contact in 30 days",
      },
    ]);

    expect(summary).toContain("1 meeting need prep");
    expect(summary).toContain("3 overdue actions");
    expect(summary).toContain("1 stale contact");
  });

  it("returns empty string for no checks", () => {
    expect(synthesizeReadiness([])).toBe("");
  });

  it("pluralizes meetings correctly", () => {
    const summary = synthesizeReadiness([
      { checkType: "no_prep", severity: "action_needed", message: "" },
      { checkType: "prep_needed", severity: "action_needed", message: "" },
      { checkType: "agenda_needed", severity: "action_needed", message: "" },
    ]);
    expect(summary).toContain("3 meetings need prep");
  });

  it("handles missing overdue count in message gracefully", () => {
    const summary = synthesizeReadiness([
      {
        checkType: "overdue_action",
        severity: "action_needed",
        message: "Some actions are overdue",
      },
    ]);
    // Falls back to check count (1) when message doesn't start with a number
    expect(summary).toContain("1 overdue action");
  });
});
