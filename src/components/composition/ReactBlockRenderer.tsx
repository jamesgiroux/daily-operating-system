import type {
  CompositionFeedbackEntityType,
  ProjectedBlock,
  RenderedProvenance,
} from "@/services/composition/contracts";
import { BLOCK_RENDERERS, GenericTextBlock } from "@/components/composition/blocks/BlockComponents";
import pageStyles from "@/pages/AccountDetailPage.module.css";

export interface ReactBlockRendererProps {
  block: ProjectedBlock;
  accountId?: string;
  entityId?: string;
  entityType?: CompositionFeedbackEntityType;
  renderedProvenance?: RenderedProvenance | null;
  editMode?: boolean;
  onSnapshotFieldSave?: (field: string, value: string) => Promise<void> | void;
}

export function ReactBlockRenderer({
  block,
  accountId,
  entityId,
  entityType = "account",
  renderedProvenance,
  editMode = false,
  onSnapshotFieldSave,
}: ReactBlockRendererProps) {
  const Renderer = BLOCK_RENDERERS[block.selected_known_type_id as keyof typeof BLOCK_RENDERERS];
  const feedbackEntityId = entityId ?? accountId;
  if (Renderer) {
    return (
      <Renderer
        block={block}
        accountId={feedbackEntityId}
        entityType={entityType}
        payload={block.payload}
        renderedProvenance={renderedProvenance}
        editMode={editMode}
        onSnapshotFieldSave={onSnapshotFieldSave}
      />
    );
  }
  if (block.selected_known_type_id === "dailyos/text") {
    return (
      <GenericTextBlock
        block={block}
        accountId={feedbackEntityId}
        entityType={entityType}
        payload={block.payload}
        renderedProvenance={renderedProvenance}
        editMode={editMode}
        onSnapshotFieldSave={onSnapshotFieldSave}
      />
    );
  }

  return (
    <article
      className={pageStyles.compositionDegradedState}
      data-block-type="unresolved"
    >
      <p className={pageStyles.compositionStateLabel}>Unavailable block</p>
      <p className={pageStyles.compositionStateText}>
        This block type is not available in the current renderer.
      </p>
    </article>
  );
}
