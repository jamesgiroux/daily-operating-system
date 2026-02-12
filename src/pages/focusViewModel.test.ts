import { describe, expect, it } from "vitest";
import type { FocusData, PrioritizedFocusAction } from "@/types";
import { buildFocusViewModel } from "./focusViewModel";

function prioritized(
  id: string,
  opts: Partial<PrioritizedFocusAction> = {},
): PrioritizedFocusAction {
  return {
    action: {
      id,
      title: `Action ${id}`,
      priority: "P1",
      status: "pending",
      createdAt: "2026-02-12T10:00:00Z",
      updatedAt: "2026-02-12T10:00:00Z",
    },
    score: 100,
    effortMinutes: 30,
    feasible: true,
    atRisk: false,
    reason: "test",
    ...opts,
  };
}

function focusData(items: PrioritizedFocusAction[], topThree: string[]): FocusData {
  return {
    priorities: [],
    keyMeetings: [],
    availableBlocks: [],
    totalFocusMinutes: 120,
    availability: {
      source: "live",
      warnings: [],
      meetingCount: 2,
      meetingMinutes: 90,
      availableMinutes: 120,
      deepWorkMinutes: 60,
      deepWorkBlocks: [],
    },
    prioritizedActions: items,
    topThree,
    implications: {
      achievableCount: 2,
      totalCount: items.length,
      atRiskCount: items.filter((i) => i.atRisk).length,
      summary: "summary",
    },
  };
}

describe("buildFocusViewModel", () => {
  it("maps top three IDs to prioritized action rows", () => {
    const items = [prioritized("a1"), prioritized("a2"), prioritized("a3")];
    const vm = buildFocusViewModel(focusData(items, ["a2", "a1", "missing"]));
    expect(vm.topThree.map((i) => i.action.id)).toEqual(["a2", "a1"]);
  });

  it("surfaces at-risk actions separately", () => {
    const items = [
      prioritized("a1", { atRisk: true }),
      prioritized("a2"),
      prioritized("a3", { atRisk: true }),
    ];
    const vm = buildFocusViewModel(focusData(items, ["a2"]));
    expect(vm.atRisk.map((i) => i.action.id)).toEqual(["a1", "a3"]);
  });

  it("excludes top-three and at-risk from other priorities", () => {
    const items = [
      prioritized("a1"),
      prioritized("a2"),
      prioritized("a3", { atRisk: true }),
      prioritized("a4"),
    ];
    const vm = buildFocusViewModel(focusData(items, ["a1", "a2"]));
    expect(vm.otherPriorities.map((i) => i.action.id)).toEqual(["a4"]);
  });
});
