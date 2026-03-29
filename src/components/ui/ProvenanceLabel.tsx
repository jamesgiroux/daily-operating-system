import { Button } from "@/components/ui/button";
import { formatShortDate } from "@/lib/utils";
import styles from "./ProvenanceLabel.module.css";

interface ConflictAction {
  source: string;
  suggestedValue: string;
  confidence?: number;
  detectedAt?: string | null;
  onAccept?: () => void;
  onDismiss?: () => void;
  pending?: boolean;
}

interface ProvenanceLabelProps {
  label?: string;
  source?: string | null;
  updatedAt?: string | null;
  summary?: string | null;
  conflict?: ConflictAction;
}

export function formatProvenanceSource(source?: string | null): string | null {
  if (!source) return null;
  const normalized = source.toLowerCase();
  if (normalized === "user_edit" || normalized === "user_correction") return "you updated";
  if (normalized.includes("salesforce") || normalized.includes("glean_crm")) return "via Salesforce";
  if (normalized.includes("gong")) return "via Gong";
  if (normalized.includes("zendesk")) return "via Zendesk";
  if (normalized === "glean" || normalized.includes("glean")) return "via Glean";
  if (normalized.includes("email")) return "via email signal";
  if (
    normalized.includes("ai_inference")
    || normalized.includes("pty_synthesis")
    || normalized.includes("intelligence")
    || normalized.includes("ai")
  ) {
    return "AI synthesis";
  }
  return source.replace(/_/g, " ");
}

function buildSummary(source?: string | null, updatedAt?: string | null) {
  const bits = [formatProvenanceSource(source)];
  if (updatedAt) bits.push(formatShortDate(updatedAt));
  return bits.filter(Boolean).join(" · ");
}

export function ProvenanceLabel({
  label,
  source,
  updatedAt,
  summary,
  conflict,
}: ProvenanceLabelProps) {
  const meta = summary ?? buildSummary(source, updatedAt);
  if (!meta && !conflict) return null;

  return (
    <div className={styles.block}>
      {label ? <div className={styles.label}>{label}</div> : null}
      {meta ? (
        <div className={styles.metaRow}>
          <span className={styles.marker} />
          <span className={styles.metaText}>{meta}</span>
        </div>
      ) : null}
      {conflict ? (
        <div className={styles.conflict}>
          <div className={styles.conflictText}>
            <span className={styles.conflictLead}>
              Suggests {conflict.suggestedValue}
            </span>
            <span className={styles.conflictMeta}>
              {[formatProvenanceSource(conflict.source), conflict.confidence != null ? `${Math.round(conflict.confidence * 100)}% confidence` : null, conflict.detectedAt ? formatShortDate(conflict.detectedAt) : null]
                .filter(Boolean)
                .join(" · ")}
            </span>
          </div>
          <div className={styles.actions}>
            {conflict.onAccept ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                className={styles.actionButton}
                onClick={conflict.onAccept}
                disabled={conflict.pending}
              >
                Accept
              </Button>
            ) : null}
            {conflict.onDismiss ? (
              <button
                type="button"
                className={styles.dismissButton}
                onClick={conflict.onDismiss}
                disabled={conflict.pending}
              >
                Dismiss
              </button>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
