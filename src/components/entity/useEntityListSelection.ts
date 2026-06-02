import { useCallback, useEffect, useMemo, useState } from "react";

interface UseEntityListSelectionOptions {
  visibleIds: readonly string[];
  activeIds?: readonly string[];
}

export interface EntityListSelectionToggleOptions {
  shiftKey?: boolean;
}

export interface EntityListSelectionApi {
  selectedIds: string[];
  selectedCount: number;
  isSelected: (id: string) => boolean;
  toggle: (id: string, options?: EntityListSelectionToggleOptions) => void;
  selectVisible: () => void;
  clear: () => void;
  pruneTo: (ids: readonly string[]) => void;
}

interface EntityTreeNode {
  id: string;
  isParent?: boolean;
}

interface FlattenEntityTreeOptions {
  expandedParents?: ReadonlySet<string>;
  expandedOnly?: boolean;
}

function orderSelection(ids: Iterable<string>, order: readonly string[]): string[] {
  const selected = new Set(ids);
  const ordered = order.filter((id) => selected.has(id));
  for (const id of selected) {
    if (!order.includes(id)) ordered.push(id);
  }
  return ordered;
}

function sameIds(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((id, index) => id === b[index]);
}

export function flattenEntityTreeIds<T extends EntityTreeNode>(
  roots: readonly T[],
  childrenByParent: Record<string, readonly T[]>,
  options: FlattenEntityTreeOptions = {},
): string[] {
  const result: string[] = [];
  const expandedParents = options.expandedParents;

  function walk(items: readonly T[]) {
    for (const item of items) {
      result.push(item.id);
      const children = childrenByParent[item.id] ?? [];
      if (children.length === 0) continue;
      if (options.expandedOnly && !expandedParents?.has(item.id)) continue;
      walk(children);
    }
  }

  walk(roots);
  return result;
}

export function useEntityListSelection({
  visibleIds,
  activeIds = visibleIds,
}: UseEntityListSelectionOptions): EntityListSelectionApi {
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [lastSelectedId, setLastSelectedId] = useState<string | null>(null);

  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const pruneTo = useCallback((ids: readonly string[]) => {
    const active = new Set(ids);
    setSelectedIds((current) => {
      const next = current.filter((id) => active.has(id));
      return sameIds(current, next) ? current : next;
    });
    setLastSelectedId((current) => current && active.has(current) ? current : null);
  }, []);

  useEffect(() => {
    pruneTo(activeIds);
  }, [activeIds, pruneTo]);

  const clear = useCallback(() => {
    setSelectedIds([]);
    setLastSelectedId(null);
  }, []);

  const toggle = useCallback((id: string, options: EntityListSelectionToggleOptions = {}) => {
    setSelectedIds((current) => {
      if (options.shiftKey && lastSelectedId) {
        const start = visibleIds.indexOf(lastSelectedId);
        const end = visibleIds.indexOf(id);
        if (start !== -1 && end !== -1) {
          const [from, to] = start < end ? [start, end] : [end, start];
          const range = visibleIds.slice(from, to + 1);
          return orderSelection([...current, ...range], activeIds);
        }
      }

      if (current.includes(id)) {
        return current.filter((selectedId) => selectedId !== id);
      }
      return orderSelection([...current, id], activeIds);
    });
    setLastSelectedId(id);
  }, [activeIds, lastSelectedId, visibleIds]);

  const selectVisible = useCallback(() => {
    setSelectedIds((current) => orderSelection([...current, ...visibleIds], activeIds));
    if (visibleIds.length > 0) {
      setLastSelectedId(visibleIds[visibleIds.length - 1]);
    }
  }, [activeIds, visibleIds]);

  const isSelected = useCallback((id: string) => selectedSet.has(id), [selectedSet]);

  return {
    selectedIds,
    selectedCount: selectedIds.length,
    isSelected,
    toggle,
    selectVisible,
    clear,
    pruneTo,
  };
}
