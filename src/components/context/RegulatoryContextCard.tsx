/**
 * RegulatoryContextCard — Chapter in the Context tab.
 *
 * Renders regulatory / compliance items (DORA, SOC 2, HIPAA, GDPR) as a
 * stacked list with per-status chip colors. Populated by the
 * strategic_context enrichment; items with status='gap' emit
 * regulatory_gap_detected signals that feed the financial_proximity
 * health dimension and the callout pipeline.
 *
 * Empty state: returns null — the section hides entirely when no
 * regulatory context has been detected for the account.
 */
import type { RegulatoryItem } from "@/types";
import css from "./RegulatoryContextCard.module.css";

interface RegulatoryContextCardProps {
  items?: RegulatoryItem[];
}

const STATUS_LABEL: Record<string, string> = {
  required: "Required",
  in_progress: "In progress",
  met: "Met",
  gap: "Gap",
};

const STATUS_TOKEN: Record<string, string> = {
  // Matches existing editorial palette — picks the closest semantic token.
  required: "var(--color-spice-turmeric)",
  in_progress: "var(--color-garden-larkspur)",
  met: "var(--color-garden-sage)",
  gap: "var(--color-spice-terracotta)",
};

function statusLabel(status: string): string {
  return STATUS_LABEL[status] ?? status;
}

function statusColor(status: string): string {
  return STATUS_TOKEN[status] ?? "var(--color-text-tertiary)";
}

export function RegulatoryContextCard({ items }: RegulatoryContextCardProps) {
  if (!items || items.length === 0) return null;

  return (
    <section aria-label="Regulatory context">
      <header className={css.header}>
        <h3 className={css.title}>
          Regulatory context
        </h3>
        <p className={css.subtitle}>
          From strategic-context enrichment. Gaps feed health scoring.
        </p>
      </header>

      <ul className={css.list}>
        {items.map((item, idx) => (
          <li
            key={`${item.standard}-${idx}`}
            className={css.item}
          >
            <div className={css.standard}>
              {item.standard}
            </div>
            <div>
              <p className={css.evidence}>
                {item.evidence}
              </p>
              {item.sourceReference && (
                <p className={css.sourceReference}>
                  {item.sourceReference}
                </p>
              )}
            </div>
            {/* Status chip color is data-driven by regulatory status. */}
            <span
              className={css.statusChip}
              style={{ backgroundColor: statusColor(item.status) }}
            >
              {statusLabel(item.status)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
