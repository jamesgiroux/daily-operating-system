import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ProjectedBlock, ProjectedComposition, ProjectedSection } from "@/services/composition/contracts";
import {
  DEFAULT_COMPOSITION_LAYOUT_OVERLAY,
  getCompositionLayoutOverlay,
  normalizeCompositionLayoutOverlay,
  resetCompositionLayoutOverlay,
  saveCompositionLayoutOverlay,
  type CompositionBlockVariant,
  type CompositionEntityType,
  type CompositionLayoutOverlay,
  type CompositionSurfaceKey,
  type LayoutOverlayResponse,
} from "@/services/composition/layoutOverlay";

export interface RenderableCompositionBlock {
  block: ProjectedBlock;
  sectionId: string;
  label: string;
  variant: CompositionBlockVariant;
  coreLocked: boolean;
}

export interface RenderableCompositionSection {
  section: ProjectedSection;
  label: string;
  coreLocked: boolean;
  blocks: RenderableCompositionBlock[];
  hiddenBlocks: RenderableCompositionBlock[];
}

export interface RenderableCompositionView {
  sections: RenderableCompositionSection[];
  hiddenSections: RenderableCompositionSection[];
  hiddenItems: Array<{ id: string; label: string; kind: "section" | "block"; sectionId?: string }>;
  hasVisibleNonCore: boolean;
}

export interface UseChapterLayoutArgs {
  projection: ProjectedComposition | null;
  entityType: CompositionEntityType;
  surfaceKey?: CompositionSurfaceKey;
}

export interface UseChapterLayoutResult {
  overlay: CompositionLayoutOverlay;
  layoutRevision: number;
  view: RenderableCompositionView;
  loading: boolean;
  saving: boolean;
  error: string | null;
  setBlockHidden: (blockId: string, hidden: boolean) => void;
  setSectionHidden: (sectionId: string, hidden: boolean) => void;
  setBlockVariant: (blockId: string, variant: CompositionBlockVariant) => void;
  setSectionLabel: (sectionId: string, label: string) => void;
  reorderBlocks: (sectionId: string, blockIds: string[]) => void;
  resetLayout: () => Promise<void>;
}

const CORE_SECTION_IDS = new Set(["headline", "masthead", "lead"]);
const CORE_BLOCK_TYPES = new Set(["account_overview"]);

function defaultOverlay(): CompositionLayoutOverlay {
  return {
    ...DEFAULT_COMPOSITION_LAYOUT_OVERLAY,
    sectionOrder: [],
    blockOrder: {},
    hiddenSectionIds: [],
    hiddenBlockIds: [],
    blockVariants: {},
    sectionLabelOverrides: {},
  };
}

function unique(values: string[]): string[] {
  return Array.from(new Set(values.filter((value) => value.trim().length > 0)));
}

function labelFromSection(section: ProjectedSection): string {
  return section.label ?? section.section_id.replace(/-/g, " ");
}

function labelFromBlock(block: ProjectedBlock): string {
  const payload = block.payload;
  for (const key of ["title", "label", "intent", "text"]) {
    const value = payload[key];
    if (typeof value === "string" && value.trim()) return value;
  }
  return block.selected_known_type_id.replace(/[_/-]/g, " ");
}

function sectionBlocks(section: ProjectedSection, blocks: ProjectedBlock[]): ProjectedBlock[] {
  return section.block_indexes
    .map((index) => blocks[index])
    .filter((block): block is ProjectedBlock => Boolean(block));
}

function orderByStoredIds<T>(items: T[], storedIds: string[], idFor: (item: T) => string): T[] {
  if (storedIds.length === 0) return items;
  const byId = new Map(items.map((item) => [idFor(item), item] as const));
  const used = new Set<string>();
  const ordered: T[] = [];
  for (const id of storedIds) {
    const item = byId.get(id);
    if (!item) continue;
    ordered.push(item);
    used.add(id);
  }
  for (const item of items) {
    if (!used.has(idFor(item))) ordered.push(item);
  }
  return ordered;
}

function isCoreSection(section: ProjectedSection): boolean {
  return CORE_SECTION_IDS.has(section.section_id);
}

function isCoreBlock(section: ProjectedSection, block: ProjectedBlock): boolean {
  return isCoreSection(section) || CORE_BLOCK_TYPES.has(block.selected_known_type_id);
}

export function applyOverlayToComposition(
  projection: ProjectedComposition | null,
  overlayInput: CompositionLayoutOverlay | null | undefined,
): RenderableCompositionView {
  if (!projection) {
    return { sections: [], hiddenSections: [], hiddenItems: [], hasVisibleNonCore: false };
  }

  const overlay = normalizeCompositionLayoutOverlay(overlayInput);
  const hiddenSectionIds = new Set(overlay.hiddenSectionIds);
  const hiddenBlockIds = new Set(overlay.hiddenBlockIds);
  const coreSections = projection.sections.filter(isCoreSection);
  const nonCoreSections = projection.sections.filter((section) => !isCoreSection(section));
  const orderedSections = [
    ...coreSections,
    ...orderByStoredIds(nonCoreSections, overlay.sectionOrder, (section) => section.section_id),
  ];

  const visibleSections: RenderableCompositionSection[] = [];
  const hiddenSections: RenderableCompositionSection[] = [];
  const hiddenItems: RenderableCompositionView["hiddenItems"] = [];

  for (const section of orderedSections) {
    const coreLocked = isCoreSection(section);
    const sourceBlocks = sectionBlocks(section, projection.blocks);
    const orderedBlocks = coreLocked
      ? sourceBlocks
      : orderByStoredIds(
          sourceBlocks,
          overlay.blockOrder[section.section_id] ?? [],
          (block) => block.block_id,
        );
    const label = overlay.sectionLabelOverrides[section.section_id] ?? labelFromSection(section);
    const renderableBlocks = orderedBlocks.map((block) => ({
      block,
      sectionId: section.section_id,
      label: labelFromBlock(block),
      variant: overlay.blockVariants[block.block_id] ?? "default",
      coreLocked: isCoreBlock(section, block),
    }));

    const visibleBlocks = renderableBlocks.filter(
      (item) => item.coreLocked || !hiddenBlockIds.has(item.block.block_id),
    );
    const hiddenBlocks = renderableBlocks.filter(
      (item) => !item.coreLocked && hiddenBlockIds.has(item.block.block_id),
    );
    for (const item of hiddenBlocks) {
      hiddenItems.push({
        id: item.block.block_id,
        label: item.label,
        kind: "block",
        sectionId: section.section_id,
      });
    }

    const renderableSection = { section, label, coreLocked, blocks: visibleBlocks, hiddenBlocks };
    const dataPresent = visibleBlocks.length > 0 || hiddenBlocks.length > 0;
    const userHidden = !coreLocked && hiddenSectionIds.has(section.section_id);

    if (userHidden && dataPresent) {
      hiddenSections.push(renderableSection);
      hiddenItems.push({ id: section.section_id, label, kind: "section" });
      continue;
    }
    if (!coreLocked && !dataPresent) continue;
    visibleSections.push(renderableSection);
  }

  return {
    sections: visibleSections,
    hiddenSections,
    hiddenItems,
    hasVisibleNonCore: visibleSections.some(
      (section) => !section.coreLocked && section.blocks.some((block) => !block.coreLocked),
    ),
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useChapterLayout({
  projection,
  entityType,
  surfaceKey = "entity_page",
}: UseChapterLayoutArgs): UseChapterLayoutResult {
  const [overlay, setOverlay] = useState<CompositionLayoutOverlay>(() => defaultOverlay());
  const [layoutRevision, setLayoutRevision] = useState(0);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const overlayRef = useRef(overlay);
  const persistedOverlayRef = useRef(overlay);
  const persistedLayoutRevisionRef = useRef(0);
  const lastPersistedMutationSequenceRef = useRef(0);
  const mutationSequenceRef = useRef(0);
  const loadSequenceRef = useRef(0);

  useEffect(() => {
    overlayRef.current = overlay;
  }, [overlay]);

  useEffect(() => {
    const sequence = loadSequenceRef.current + 1;
    const mutationSequenceAtStart = mutationSequenceRef.current;
    loadSequenceRef.current = sequence;
    setLoading(true);
    setError(null);
    getCompositionLayoutOverlay({ entityType, surfaceKey })
      .then((response) => {
        if (loadSequenceRef.current !== sequence) return;
        if (mutationSequenceRef.current !== mutationSequenceAtStart) return;
        const next = normalizeCompositionLayoutOverlay(response.overlay);
        overlayRef.current = next;
        persistedOverlayRef.current = next;
        persistedLayoutRevisionRef.current = response.layoutRevision;
        lastPersistedMutationSequenceRef.current = mutationSequenceAtStart;
        setOverlay(next);
        setLayoutRevision(response.layoutRevision);
      })
      .catch((err: unknown) => {
        if (loadSequenceRef.current !== sequence) return;
        if (mutationSequenceRef.current !== mutationSequenceAtStart) return;
        setError(errorMessage(err));
        const next = defaultOverlay();
        overlayRef.current = next;
        persistedOverlayRef.current = next;
        setOverlay(next);
      })
      .finally(() => {
        if (loadSequenceRef.current === sequence) setLoading(false);
      });
  }, [entityType, surfaceKey]);

  const persist = useCallback(
    (mutator: (current: CompositionLayoutOverlay) => CompositionLayoutOverlay) => {
      const sequence = mutationSequenceRef.current + 1;
      mutationSequenceRef.current = sequence;
      const next = mutator(overlayRef.current);
      overlayRef.current = next;
      setOverlay(next);
      setSaving(true);
      setError(null);

      void saveCompositionLayoutOverlay({ entityType, surfaceKey, overlay: next })
        .then((response: LayoutOverlayResponse) => {
          const saved = normalizeCompositionLayoutOverlay(response.overlay);
          if (sequence > lastPersistedMutationSequenceRef.current) {
            persistedOverlayRef.current = saved;
            persistedLayoutRevisionRef.current = response.layoutRevision;
            lastPersistedMutationSequenceRef.current = sequence;
          }
          if (mutationSequenceRef.current !== sequence) return;
          overlayRef.current = saved;
          setOverlay(saved);
          setLayoutRevision(response.layoutRevision);
        })
        .catch((err: unknown) => {
          if (mutationSequenceRef.current !== sequence) return;
          const persisted = persistedOverlayRef.current;
          overlayRef.current = persisted;
          setOverlay(persisted);
          setLayoutRevision(persistedLayoutRevisionRef.current);
          setError(errorMessage(err));
        })
        .finally(() => {
          if (mutationSequenceRef.current === sequence) setSaving(false);
        });
    },
    [entityType, surfaceKey],
  );

  const setBlockHidden = useCallback(
    (blockId: string, hidden: boolean) => {
      persist((current) => ({
        ...current,
        hiddenBlockIds: hidden
          ? unique([...current.hiddenBlockIds, blockId])
          : current.hiddenBlockIds.filter((id) => id !== blockId),
      }));
    },
    [persist],
  );

  const setSectionHidden = useCallback(
    (sectionId: string, hidden: boolean) => {
      persist((current) => ({
        ...current,
        hiddenSectionIds: hidden
          ? unique([...current.hiddenSectionIds, sectionId])
          : current.hiddenSectionIds.filter((id) => id !== sectionId),
      }));
    },
    [persist],
  );

  const setBlockVariant = useCallback(
    (blockId: string, variant: CompositionBlockVariant) => {
      persist((current) => ({
        ...current,
        blockVariants: { ...current.blockVariants, [blockId]: variant },
      }));
    },
    [persist],
  );

  const setSectionLabel = useCallback(
    (sectionId: string, label: string) => {
      persist((current) => ({
        ...current,
        sectionLabelOverrides: {
          ...current.sectionLabelOverrides,
          [sectionId]: label.trim(),
        },
      }));
    },
    [persist],
  );

  const reorderBlocks = useCallback(
    (sectionId: string, blockIds: string[]) => {
      persist((current) => ({
        ...current,
        blockOrder: { ...current.blockOrder, [sectionId]: unique(blockIds) },
      }));
    },
    [persist],
  );

  const resetLayout = useCallback(async () => {
    const sequence = mutationSequenceRef.current + 1;
    mutationSequenceRef.current = sequence;
    const next = defaultOverlay();
    overlayRef.current = next;
    setOverlay(next);
    setSaving(true);
    setError(null);
    try {
      const response = await resetCompositionLayoutOverlay({ entityType, surfaceKey });
      const reset = normalizeCompositionLayoutOverlay(response.overlay);
      if (sequence > lastPersistedMutationSequenceRef.current) {
        persistedOverlayRef.current = reset;
        persistedLayoutRevisionRef.current = response.layoutRevision;
        lastPersistedMutationSequenceRef.current = sequence;
      }
      if (mutationSequenceRef.current !== sequence) return;
      overlayRef.current = reset;
      setOverlay(reset);
      setLayoutRevision(response.layoutRevision);
    } catch (err) {
      if (mutationSequenceRef.current !== sequence) return;
      const persisted = persistedOverlayRef.current;
      overlayRef.current = persisted;
      setOverlay(persisted);
      setLayoutRevision(persistedLayoutRevisionRef.current);
      setError(errorMessage(err));
    } finally {
      if (mutationSequenceRef.current === sequence) setSaving(false);
    }
  }, [entityType, surfaceKey]);

  const view = useMemo(() => applyOverlayToComposition(projection, overlay), [overlay, projection]);

  return {
    overlay,
    layoutRevision,
    view,
    loading,
    saving,
    error,
    setBlockHidden,
    setSectionHidden,
    setBlockVariant,
    setSectionLabel,
    reorderBlocks,
    resetLayout,
  };
}
