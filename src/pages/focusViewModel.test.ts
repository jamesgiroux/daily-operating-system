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
    expect(vm.otherPrioritiesVisible.map((i) => i.action.id)).toEqual(["a4"]);
  });

  it("shows only P1 actions in other priorities", () => {
    const items = [
      prioritized("a1"),
      prioritized("a2", {
        action: {
          ...prioritized("a2").action,
          priority: "P2",
        },
      }),
      prioritized("a3"),
    ];
    const vm = buildFocusViewModel(focusData(items, ["a1"]));
    expect(vm.otherPrioritiesVisible.map((i) => i.action.id)).toEqual(["a3"]);
  });

  it("caps other priorities at five and toggles view-all link", () => {
    const items = [
      prioritized("a1"),
      prioritized("a2"),
      prioritized("a3"),
      prioritized("a4"),
      prioritized("a5"),
      prioritized("a6"),
      prioritized("a7"),
    ];
    const vm = buildFocusViewModel(focusData(items, ["a1"]));
    expect(vm.otherPrioritiesVisible).toHaveLength(5);
    expect(vm.otherPrioritiesP1Total).toBe(6);
    expect(vm.showViewAllActions).toBe(true);
  });

  it("excludes top-three items from at-risk section", () => {
    const items = [
      prioritized("a1", { atRisk: true }),
      prioritized("a2", { atRisk: true }),
      prioritized("a3"),
    ];
    const vm = buildFocusViewModel(focusData(items, ["a1"]));
    expect(vm.topThree.map((i) => i.action.id)).toEqual(["a1"]);
    expect(vm.atRisk.map((i) => i.action.id)).toEqual(["a2"]);
  });

  it("hides view-all link when five or fewer P1 actions remain", () => {
    const items = [
      prioritized("a1"),
      prioritized("a2"),
      prioritized("a3"),
      prioritized("a4"),
      prioritized("a5"),
      prioritized("a6", {
        action: {
          ...prioritized("a6").action,
          priority: "P2",
        },
      }),
    ];
    const vm = buildFocusViewModel(focusData(items, ["a1"]));
    expect(vm.otherPrioritiesP1Total).toBe(4);
    expect(vm.showViewAllActions).toBe(false);
  });
});
