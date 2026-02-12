import type { FocusData, PrioritizedFocusAction } from "@/types";

export interface FocusViewModel {
  topThree: PrioritizedFocusAction[];
  atRisk: PrioritizedFocusAction[];
  otherPriorities: PrioritizedFocusAction[];
}

export function buildFocusViewModel(data: FocusData): FocusViewModel {
  const prioritized = data.prioritizedActions ?? [];
  const map = new Map<string, PrioritizedFocusAction>(
    prioritized.map((item) => [item.action.id, item]),
  );

  const topThree = (data.topThree ?? [])
    .map((id) => map.get(id))
    .filter((v): v is PrioritizedFocusAction => Boolean(v));

  const topThreeSet = new Set(topThree.map((item) => item.action.id));
  const atRisk = prioritized.filter((a) => a.atRisk);
  const otherPriorities = prioritized.filter(
    (a) => !topThreeSet.has(a.action.id) && !a.atRisk,
  );

  return { topThree, atRisk, otherPriorities };
}
