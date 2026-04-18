/**
 * QuoteWallPlaceholder — editorial empty state for the "Their voice" chapter.
 *
 * The full quote wall (filter bar + verbatim transcript quotes + copy/use actions)
 * is tracked in DOS-205. Until schema + prompt land, this chapter announces the
 * coming surface with an editorial empty rather than hiding it entirely — gaps
 * are first-class content per PRINCIPLES.md.
 */
import { EditorialEmpty } from "./EditorialEmpty";

export function QuoteWallPlaceholder() {
  return (
    <EditorialEmpty
      title="A wall of their own words is coming."
      message="Verbatim quotes pulled from transcripts — filterable by speaker, topic, and sentiment — will live here. Not yet captured."
    />
  );
}
