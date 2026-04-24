/**
 * RegulatoryContextCard — Chapter in the Context tab (DOS-207).
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
  required: "var(--color-turmeric, #c88a2b)",
  in_progress: "var(--color-larkspur, #4f6bed)",
  met: "var(--color-sage, #6b8e6b)",
  gap: "var(--color-terracotta, #c8664a)",
};

function statusLabel(status: string): string {
  return STATUS_LABEL[status] ?? status;
}

function statusColor(status: string): string {
  return STATUS_TOKEN[status] ?? "var(--color-ink-60, #666)";
}

export function RegulatoryContextCard({ items }: RegulatoryContextCardProps) {
  if (!items || items.length === 0) return null;

  return (
    <section aria-label="Regulatory context">
      <header style={{ marginBottom: "0.75rem" }}>
        <h3
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: "1.25rem",
            margin: 0,
          }}
        >
          Regulatory context
        </h3>
        <p
          style={{
            color: "var(--color-ink-60, #666)",
            fontSize: "0.875rem",
            margin: "0.25rem 0 0",
          }}
        >
          From strategic-context enrichment. Gaps feed health scoring.
        </p>
      </header>

      <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
        {items.map((item, idx) => (
          <li
            key={`${item.standard}-${idx}`}
            style={{
              display: "grid",
              gridTemplateColumns: "minmax(140px, 20%) 1fr auto",
              gap: "0.75rem",
              padding: "0.75rem 0",
              borderTop: idx === 0 ? "none" : "1px solid var(--color-rule, #e5e5e5)",
              alignItems: "start",
            }}
          >
            <div
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: "1rem",
                fontWeight: 500,
              }}
            >
              {item.standard}
            </div>
            <div>
              <p style={{ margin: 0, fontSize: "0.9375rem", lineHeight: 1.4 }}>
                {item.evidence}
              </p>
              {item.sourceReference && (
                <p
                  style={{
                    margin: "0.25rem 0 0",
                    fontSize: "0.75rem",
                    color: "var(--color-ink-60, #666)",
                    fontFamily: "var(--font-mono, monospace)",
                  }}
                >
                  {item.sourceReference}
                </p>
              )}
            </div>
            <span
              style={{
                padding: "0.125rem 0.5rem",
                borderRadius: "999px",
                backgroundColor: statusColor(item.status),
                color: "white",
                fontSize: "0.75rem",
                fontWeight: 500,
                whiteSpace: "nowrap",
                alignSelf: "start",
              }}
            >
              {statusLabel(item.status)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
