import { describe, expect, it } from "vitest";

import {
  classifyWeekShapeState,
  formatBlockRange,
  formatDueContext,
  resolveSuggestionLink,
  synthesizeReadiness,
} from "./weekPageViewModel";

describe("weekPageViewModel", () => {
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

  it("formats block ranges from HH:mm inputs", () => {
    const value = formatBlockRange("09:00", "10:30");
    expect(value).toMatch(/-/);
    expect(value).toContain(":");
  });

  it("formats overdue context when daysOverdue is provided", () => {
    expect(formatDueContext("2026-02-10", 2)).toBe("2 days overdue");
  });

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
});
