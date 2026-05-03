/**
 * Provenance tag for intelligence items.
 * Shows a muted label indicating where an intelligence item came from.
 * Omits the tag for pty_synthesis (default, no tag needed).
 */
import type { ItemSource } from "@/types";

interface ProvenanceTagProps {
  itemSource?: ItemSource;
  discrepancy?: boolean;
}

function formatProvenance(source: ItemSource): string | null {
  switch (source.source) {
    case "transcript":
      return source.reference ? `from meeting ${source.reference}` : "from meeting";
    case "user_correction":
      return "you edited this";
    case "glean_crm":
      return "from Salesforce";
    case "glean_zendesk":
      return "from Zendesk";
    case "glean_gong":
      return "from Gong";
    case "glean_chat":
      return "from Glean";
    case "email":
      return "from email";
    case "local_file":
      return source.reference ? `from ${source.reference}` : "from file";
    case "pty_synthesis":
      return null; // Default source, no tag
    default:
      return null;
  }
}

export function ProvenanceTag({ itemSource, discrepancy }: ProvenanceTagProps) {
  if (!itemSource) return null;
  const label = formatProvenance(itemSource);
  if (!label) return null;

  return (
    <span className="provenance-tag">
      {discrepancy && (
        <span className="provenance-discrepancy" title="Multiple sources disagree on this item">
          !{" "}
        </span>
      )}
      {label}
    </span>
  );
}
