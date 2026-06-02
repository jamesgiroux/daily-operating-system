import type { ProjectedBlock, RenderedProvenance } from "@/services/composition/contracts";
import { BLOCK_RENDERERS, GenericTextBlock } from "@/components/composition/blocks/BlockComponents";
import pageStyles from "@/pages/AccountDetailPage.module.css";

export interface ReactBlockRendererProps {
  block: ProjectedBlock;
  accountId?: string;
  renderedProvenance?: RenderedProvenance | null;
  editMode?: boolean;
}

export function ReactBlockRenderer({ block, accountId, renderedProvenance, editMode = false }: ReactBlockRendererProps) {
  const Renderer = BLOCK_RENDERERS[block.selected_known_type_id as keyof typeof BLOCK_RENDERERS];
  if (Renderer) {
    return (
      <Renderer
        block={block}
        accountId={accountId}
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
        accountId={accountId}
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
