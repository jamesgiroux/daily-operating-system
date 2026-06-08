import type {
  CompositionFeedbackEntityType,
  ProjectedBlock,
  RenderedProvenance,
} from "@/services/composition/contracts";
import { BLOCK_RENDERERS, GenericTextBlock } from "@/components/composition/blocks/BlockComponents";
import type { AccountDetail, EntityIntelligence } from "@/types";
import pageStyles from "@/pages/AccountDetailPage.module.css";

export interface ReactBlockRendererProps {
  block: ProjectedBlock;
  accountId?: string;
  entityId?: string;
  entityType?: CompositionFeedbackEntityType;
  accountDetail?: AccountDetail | null;
  intelligence?: EntityIntelligence | null;
  renderedProvenance?: RenderedProvenance | null;
  editMode?: boolean;
}

export function ReactBlockRenderer({
  block,
  accountId,
  entityId,
  entityType = "account",
  accountDetail,
  intelligence,
  renderedProvenance,
  editMode = false,
}: ReactBlockRendererProps) {
  const Renderer = BLOCK_RENDERERS[block.selected_known_type_id as keyof typeof BLOCK_RENDERERS];
  const feedbackEntityId = entityId ?? accountId;
  if (Renderer) {
    return (
      <Renderer
        block={block}
        accountId={feedbackEntityId}
        entityType={entityType}
        accountDetail={accountDetail}
        intelligence={intelligence}
        payload={block.payload}
        renderedProvenance={renderedProvenance}
        editMode={editMode}
      />
    );
  }
  if (block.selected_known_type_id === "dailyos/text") {
    return (
      <GenericTextBlock
        block={block}
        accountId={feedbackEntityId}
        entityType={entityType}
        accountDetail={accountDetail}
        intelligence={intelligence}
        payload={block.payload}
        renderedProvenance={renderedProvenance}
        editMode={editMode}
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
