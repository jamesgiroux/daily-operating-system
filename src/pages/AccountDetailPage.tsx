import { useCallback, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useNavigate, useParams } from "@tanstack/react-router";
import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import {
  Activity,
  AlignLeft,
  Award,
  Briefcase,
  Compass,
  Eye,
  FileText,
  GripVertical,
  Lock,
  Plus,
  RotateCcw,
  SlidersHorizontal,
  Telescope,
  Users,
} from "lucide-react";
import { MarginSection } from "@/components/editorial/MarginSection";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { ReactBlockRenderer } from "@/components/composition/ReactBlockRenderer";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { Segmented } from "@/components/ui/Segmented";
import { Switch } from "@/components/ui/Switch";
import { useAccountSnapshotActions } from "@/hooks/useAccountSnapshotActions";
import { useChapterLayout, type RenderableCompositionBlock } from "@/hooks/useChapterLayout";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import type { ProjectedBlock } from "@/services/composition/contracts";
import type { CompositionBlockVariant } from "@/services/composition/layoutOverlay";
import shared from "@/styles/entity-detail.module.css";
import pageStyles from "./AccountDetailPage.module.css";

const SECTION_ICONS: Record<string, ReactNode> = {
  headline: <AlignLeft size={18} strokeWidth={1.5} />,
  "your-assessment": <Activity size={18} strokeWidth={1.5} />,
  "on-track": <Award size={18} strokeWidth={1.5} />,
  "needs-attention": <Eye size={18} strokeWidth={1.5} />,
  outlook: <Telescope size={18} strokeWidth={1.5} />,
  "relationship-health": <Activity size={18} strokeWidth={1.5} />,
  "about-intelligence": <FileText size={18} strokeWidth={1.5} />,
  thesis: <AlignLeft size={18} strokeWidth={1.5} />,
  "the-room": <Users size={18} strokeWidth={1.5} />,
  "what-matters": <Compass size={18} strokeWidth={1.5} />,
  "value-commitments": <Award size={18} strokeWidth={1.5} />,
  "their-voice": <Users size={18} strokeWidth={1.5} />,
  "commercial-shape": <Briefcase size={18} strokeWidth={1.5} />,
  "technical-shape": <Compass size={18} strokeWidth={1.5} />,
  "relationship-fabric": <Users size={18} strokeWidth={1.5} />,
  "about-dossier": <FileText size={18} strokeWidth={1.5} />,
  outputs: <FileText size={18} strokeWidth={1.5} />,
  "the-record": <Activity size={18} strokeWidth={1.5} />,
};

function accountNameFromBlocks(blocks: ProjectedBlock[], accountId: string | undefined): string {
  const overview = blocks.find((block) => block.selected_known_type_id === "account_overview");
  const account = overview?.payload.account;
  if (account && typeof account === "object" && !Array.isArray(account)) {
    const displayName = (account as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  return accountId ?? "Account";
}

const VARIANT_OPTIONS = [
  { value: "default", label: "Default" },
  { value: "compact", label: "Compact" },
  { value: "spotlight", label: "Spotlight" },
] as const;

function transformStyle(transform: ReturnType<typeof useSortable>["transform"]): string | undefined {
  if (!transform) return undefined;
  return `translate3d(${Math.round(transform.x)}px, ${Math.round(transform.y)}px, 0) scaleX(${transform.scaleX}) scaleY(${transform.scaleY})`;
}

function SortableBlockFrame({
  item,
  accountId,
  editMode,
  renderedProvenance,
  onHiddenChange,
  onVariantChange,
  onSnapshotFieldSave,
}: {
  item: RenderableCompositionBlock;
  accountId?: string;
  editMode: boolean;
  renderedProvenance: ReturnType<typeof useProjectedComposition>["renderedProvenance"];
  onHiddenChange: (blockId: string, hidden: boolean) => void;
  onVariantChange: (blockId: string, variant: CompositionBlockVariant) => void;
  onSnapshotFieldSave?: (field: string, value: string) => Promise<void> | void;
}) {
  const disabled = !editMode || item.coreLocked;
  const {
    attributes,
    listeners,
    setActivatorNodeRef,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: item.block.block_id, disabled });
  const instructionsId = `reorder-instructions-${item.block.block_id}`;
  const lockedId = `locked-${item.block.block_id}`;

  return (
    <div
      ref={setNodeRef}
      className={pageStyles.compositionEditableBlock}
      data-edit-mode={editMode}
      data-dragging={isDragging}
      data-layout-variant={item.variant}
      style={{ transform: transformStyle(transform), transition }}
    >
      {editMode && (
        <div className={pageStyles.compositionBlockToolbar} data-ds-name="CompositionBlockToolbar" data-ds-tier="pattern" data-ds-spec="patterns/CompositionBlockToolbar.md">
          <button
            type="button"
            ref={setActivatorNodeRef}
            className={pageStyles.compositionReorderHandle}
            disabled={disabled}
            {...(!disabled ? attributes : {})}
            {...(!disabled ? listeners : {})}
            aria-label={item.coreLocked ? `${item.label} is locked` : `Move ${item.label}`}
            aria-describedby={item.coreLocked ? lockedId : instructionsId}
          >
            {item.coreLocked ? <Lock size={14} strokeWidth={1.6} /> : <GripVertical size={14} strokeWidth={1.6} />}
          </button>
          <span id={instructionsId} className={pageStyles.compositionAssistiveText}>
            Space to grab, arrows to move, Space or Enter to drop, Escape to cancel.
          </span>
          {item.coreLocked && (
            <span id={lockedId} className={pageStyles.compositionLockedReason}>
              Core lead content stays fixed.
            </span>
          )}
          <label className={pageStyles.compositionToolbarControl}>
            <span>Shown</span>
            <Switch
              checked
              disabled={item.coreLocked}
              onCheckedChange={(checked) => onHiddenChange(item.block.block_id, !checked)}
              aria-label={`${item.coreLocked ? "Locked visibility for" : "Toggle visibility for"} ${item.label}`}
            />
          </label>
          <Segmented<CompositionBlockVariant>
            aria-label={`Variant for ${item.label}`}
            value={item.variant}
            options={VARIANT_OPTIONS}
            disabled={item.coreLocked}
            onChange={(variant) => onVariantChange(item.block.block_id, variant)}
          />
        </div>
      )}
      <ReactBlockRenderer
        block={item.block}
        accountId={accountId}
        renderedProvenance={renderedProvenance}
        editMode={editMode}
        onSnapshotFieldSave={onSnapshotFieldSave}
      />
    </div>
  );
}

function CompositionInserter({
  hiddenItems,
  onRestoreSection,
  onRestoreBlock,
}: {
  hiddenItems: ReturnType<typeof useChapterLayout>["view"]["hiddenItems"];
  onRestoreSection: (sectionId: string) => void;
  onRestoreBlock: (blockId: string) => void;
}) {
  if (hiddenItems.length === 0) return null;
  return (
    <div className={pageStyles.compositionInserter} data-ds-name="CompositionInserter" data-ds-tier="pattern" data-ds-spec="patterns/CompositionInserter.md">
      <p className={pageStyles.compositionInserterLabel}>Hidden</p>
      <div className={pageStyles.compositionInserterList}>
        {hiddenItems.map((item) => (
          <button
            key={`${item.kind}-${item.id}`}
            type="button"
            className={pageStyles.compositionInserterButton}
            onClick={() => {
              if (item.kind === "section") onRestoreSection(item.id);
              else onRestoreBlock(item.id);
            }}
          >
            <Plus size={13} strokeWidth={1.7} />
            {item.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [editMode, setEditMode] = useState(false);
  const composition = useProjectedComposition(accountId);
  const projection = composition.data?.projection ?? null;
  const layout = useChapterLayout({ projection, entityType: "account" });
  const visibleSections = layout.view.sections;
  const accountName = accountNameFromBlocks(projection?.blocks ?? [], accountId);
  const {
    onSnapshotFieldSave: handleSnapshotFieldSave,
    onEnrich: handleEnrich,
    enriching,
    enrichError,
  } = useAccountSnapshotActions(accountId, composition.refetch);
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const blockLabels = useMemo(() => {
    const labels = new Map<string, string>();
    for (const section of visibleSections) {
      for (const item of section.blocks) labels.set(item.block.block_id, item.label);
    }
    return labels;
  }, [visibleSections]);

  const chapters = useMemo(
    () =>
      visibleSections.map(({ section, label }) => ({
        id: section.section_id,
        label,
        icon: SECTION_ICONS[section.section_id] ?? <FileText size={18} strokeWidth={1.5} />,
      })),
    [visibleSections],
  );

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Account",
      atmosphereColor: "turmeric" as const,
      activePage: "accounts" as const,
      breadcrumbs: [
        { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
        { label: accountName },
      ],
      chapters,
    }),
    [accountName, chapters, navigate],
  );
  useRegisterMagazineShell(shellConfig);

  useUpdateFolioVolatile(
    {
      folioStatusText: enriching
        ? "Refreshing intelligence..."
        : enrichError
          ? "Refresh failed"
        : composition.loading
        ? "Composing..."
        : layout.saving
          ? "Saving layout..."
        : composition.data?.served_from_cache
          ? "Projected from cache"
          : undefined,
      folioActions: (
        <div className={shared.folioActions}>
          <button
            type="button"
            className={pageStyles.compositionCustomizeButton}
            aria-pressed={editMode}
            onClick={() => setEditMode((current) => !current)}
          >
            <SlidersHorizontal size={14} strokeWidth={1.7} />
            {editMode ? "Done" : "Customize"}
          </button>
          <FolioRefreshButton onClick={handleEnrich} loading={enriching || composition.loading} />
        </div>
      ),
    },
    `${accountId ?? "account"}-${editMode}-${layout.saving}-${enriching}`,
  );

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || active.id === over.id) return;
      const section = visibleSections.find((candidate) =>
        candidate.blocks.some((item) => item.block.block_id === active.id || item.block.block_id === over.id),
      );
      if (!section) return;
      const ids = section.blocks.map((item) => item.block.block_id);
      const oldIndex = ids.indexOf(String(active.id));
      const newIndex = ids.indexOf(String(over.id));
      if (oldIndex < 0 || newIndex < 0) return;
      layout.reorderBlocks(section.section.section_id, arrayMove(ids, oldIndex, newIndex));
    },
    [layout, visibleSections],
  );

  const announcements = useMemo(
    () => ({
      onDragStart({ active }: { active: { id: string | number } }) {
        return `Started moving ${blockLabels.get(String(active.id)) ?? "block"}.`;
      },
      onDragOver({ active, over }: { active: { id: string | number }; over?: { id: string | number } | null }) {
        if (!over) return undefined;
        return `${blockLabels.get(String(active.id)) ?? "Block"} is over ${blockLabels.get(String(over.id)) ?? "another block"}.`;
      },
      onDragEnd({ active, over }: { active: { id: string | number }; over?: { id: string | number } | null }) {
        if (!over) return `${blockLabels.get(String(active.id)) ?? "Block"} was dropped.`;
        return `${blockLabels.get(String(active.id)) ?? "Block"} moved near ${blockLabels.get(String(over.id)) ?? "another block"}.`;
      },
      onDragCancel({ active }: { active: { id: string | number } }) {
        return `Canceled moving ${blockLabels.get(String(active.id)) ?? "block"}.`;
      },
    }),
    [blockLabels],
  );

  if (composition.loading && !projection) return <EditorialLoading />;
  if (composition.error) {
    return <EditorialError message={composition.error} onRetry={composition.refetch} />;
  }
  if (!projection || projection.sections.length === 0 || projection.blocks.length === 0) {
    return <EditorialEmpty title="No account composition" message="DailyOS has not produced an account surface yet." />;
  }

  function renderBlock(item: RenderableCompositionBlock) {
    return (
      <SortableBlockFrame
        key={item.block.block_id}
        item={item}
        accountId={accountId}
        editMode={editMode}
        renderedProvenance={composition.renderedProvenance}
        onHiddenChange={layout.setBlockHidden}
        onVariantChange={layout.setBlockVariant}
        onSnapshotFieldSave={handleSnapshotFieldSave}
      />
    );
  }


  return (
    <main
      className={pageStyles.compositionSurface}
      data-composition-id={projection.composition_id}
      data-composition-version={projection.composition_version ?? 0}
      data-fallback-policy-version={projection.fallback_policy_version}
      data-edit-mode={editMode}
      data-ds-name={editMode ? "CompositionEditMode" : undefined}
      data-ds-tier={editMode ? "pattern" : undefined}
      data-ds-spec={editMode ? "patterns/CompositionEditMode.md" : undefined}
    >
      {editMode && (
        <div className={pageStyles.compositionEditStatus} role="status">
          <span>{layout.saving ? "Saving layout" : layout.error ? `Layout issue: ${layout.error}` : "Editing account layout"}</span>
          <button type="button" className={pageStyles.compositionResetButton} onClick={() => void layout.resetLayout()}>
            <RotateCcw size={13} strokeWidth={1.7} />
            Reset
          </button>
        </div>
      )}
      {editMode && !layout.view.hasVisibleNonCore && (
        <div className={pageStyles.compositionEmptyLayoutNotice}>
          <p className={pageStyles.compositionStateLabel}>Default available</p>
          <p className={pageStyles.compositionStateText}>Only core lead content is visible. Reset restores the shipped Account layout.</p>
          <button type="button" className={pageStyles.compositionInserterButton} onClick={() => void layout.resetLayout()}>
            <RotateCcw size={13} strokeWidth={1.7} />
            Reset layout
          </button>
        </div>
      )}
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
        accessibility={{ announcements }}
      >
      {visibleSections.map((renderableSection) => {
        const section = renderableSection.section;
        const blocks = renderableSection.blocks;
        if (section.section_id === "headline") {
          return (
            <section
              key={section.section_id}
              id={section.section_id}
              className={pageStyles.compositionMasthead}
              data-section-id={section.section_id}
              data-section-layout={section.layout}
            >
              <div className={pageStyles.compositionBlockStack}>
                <SortableContext items={blocks.map((item) => item.block.block_id)} strategy={verticalListSortingStrategy}>
                  {blocks.map(renderBlock)}
                </SortableContext>
              </div>
            </section>
          );
        }

        // Production parity: chapters without content don't render at all —
        // no headers, no empty-state placeholders, no scroll gaps. Edit mode
        // keeps them visible so layout customization can re-enable content.
        const hasContent = blocks.some(
          (item) => item.block.payload.empty_state !== true,
        );
        if (!editMode && (!hasContent || blocks.length === 0)) {
          return null;
        }

        // Main-branch layout: the shared MarginSection grid owns sidebar
        // label + content widths; block ChapterHeadings own the titles.
        return (
          <MarginSection key={section.section_id} id={section.section_id} label={renderableSection.label}>
            {editMode && (
              <label className={pageStyles.compositionSectionVisibility}>
                <Switch
                  checked
                  disabled={renderableSection.coreLocked}
                  onCheckedChange={(checked) => layout.setSectionHidden(section.section_id, !checked)}
                  aria-label={`Toggle section ${renderableSection.label}`}
                />
                <span>{renderableSection.coreLocked ? "Locked" : "Shown"}</span>
              </label>
            )}
            <div className={pageStyles.compositionBlockStack}>
              {blocks.length > 0 ? (
                <SortableContext items={blocks.map((item) => item.block.block_id)} strategy={verticalListSortingStrategy}>
                  {blocks.map(renderBlock)}
                </SortableContext>
              ) : (
                <div className={pageStyles.compositionDegradedState}>
                  <p className={pageStyles.compositionStateLabel}>Empty section</p>
                  <p className={pageStyles.compositionStateText}>No renderable blocks are available for this section.</p>
                </div>
              )}
            </div>
          </MarginSection>
        );
      })}
      </DndContext>
      {editMode && (
        <CompositionInserter
          hiddenItems={layout.view.hiddenItems}
          onRestoreSection={(sectionId) => layout.setSectionHidden(sectionId, false)}
          onRestoreBlock={(blockId) => layout.setBlockHidden(blockId, false)}
        />
      )}
      <FinisMarker />
    </main>
  );
}
