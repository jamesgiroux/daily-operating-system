import type { FocusData, PrioritizedFocusAction } from "@/types";

export interface FocusViewModel {
  topThree: PrioritizedFocusAction[];
  atRisk: PrioritizedFocusAction[];
  otherPrioritiesVisible: PrioritizedFocusAction[];
  otherPrioritiesP1Total: number;
  showViewAllActions: boolean;
  totalPendingActions: number;
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
  const atRisk = prioritized.filter(
    (a) => a.atRisk && !topThreeSet.has(a.action.id),
  );
  const otherPriorities = prioritized.filter(
    (a) => !topThreeSet.has(a.action.id) && !a.atRisk,
  );
  const p1OtherPriorities = otherPriorities.filter(
    (item) => item.action.priority === "P1",
  );
  const otherPrioritiesVisible = p1OtherPriorities.slice(0, 5);
  const otherPrioritiesP1Total = p1OtherPriorities.length;
  const showViewAllActions = otherPrioritiesP1Total > 5;
  const totalPendingActions = prioritized.length;

  return {
    topThree,
    atRisk,
    otherPrioritiesVisible,
    otherPrioritiesP1Total,
    showViewAllActions,
    totalPendingActions,
  };
}
