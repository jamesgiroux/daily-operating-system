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
      title="Coming in the next release of DailyOS"
      message="A wall of their own words — verbatim quotes pulled from transcripts, surveys, and emails, filterable by speaker, topic, and sentiment."
    />
  );
}
