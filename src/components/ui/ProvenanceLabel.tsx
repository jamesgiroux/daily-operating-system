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

/**
 * Translate internal source system names to user-friendly product vocabulary.
 * See ADR-0083 for the canonical mapping.
 */
export function formatProvenanceSource(source?: string | null): string | null {
  if (!source) return null;
  const normalized = source.toLowerCase().trim();

  // Exact matches first (ADR-0083 table)
  if (normalized === "user_correction") return "you corrected";
  if (normalized === "user" || normalized === "user_edit" || normalized === "user_noted") return "you noted";
  if (normalized === "pty_synthesis" || normalized === "pty") return "AI";
  if (normalized === "glean_chat") return "Glean";
  if (normalized === "glean") return "Glean";
  if (normalized === "salesforce") return "Salesforce";
  if (normalized === "zendesk") return "Zendesk";
  if (normalized === "gong") return "Gong";
  if (normalized === "google") return "Google Calendar";
  if (normalized === "clay") return "Clay";
  if (normalized === "ai") return "AI";
  if (normalized === "inference") return "AI synthesis";

  // Substring matches for compound source names
  if (normalized.includes("salesforce") || normalized.includes("glean_crm")) return "Salesforce";
  if (normalized.includes("gong")) return "Gong";
  if (normalized.includes("zendesk")) return "Zendesk";
  if (normalized.includes("glean")) return "Glean";
  if (normalized.includes("google")) return "Google Calendar";
  if (normalized.includes("clay")) return "Clay";
  if (normalized.includes("email")) return "email";
  if (
    normalized.includes("ai_inference")
    || normalized.includes("pty_synthesis")
    || normalized.includes("pty")
    || normalized.includes("intelligence")
    || normalized.includes("ai")
  ) {
    return "AI";
  }

  // Fallback: humanize underscores
  return source.replace(/_/g, " ");
}

/** Convenience alias — use in UI components that display a source label. */
export const formatSource = formatProvenanceSource;

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
