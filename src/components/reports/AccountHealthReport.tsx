import { ReportSection } from "./ReportSection";
import type { AccountHealthContent } from "@/types/reports";

interface AccountHealthReportProps {
  content: AccountHealthContent;
}

export function AccountHealthReport({ content }: AccountHealthReportProps) {
  return (
    <div style={{ padding: "2rem", maxWidth: "800px" }}>
      <ReportSection heading="Overall Assessment">
        <p
          style={{
            fontFamily: "var(--font-editorial)",
            fontSize: "1.1rem",
            lineHeight: 1.7,
            color: "var(--color-desk-charcoal)",
            maxWidth: "65ch",
          }}
        >
          {content.overallAssessment}
        </p>
        {content.healthScoreNarrative && (
          <p
            style={{
              fontSize: "0.875rem",
              lineHeight: 1.6,
              color: "var(--color-desk-charcoal)",
              opacity: 0.8,
              marginTop: "0.75rem",
            }}
          >
            {content.healthScoreNarrative}
          </p>
        )}
      </ReportSection>

      <ReportSection heading="Relationship">
        <p style={{ fontSize: "0.9rem", lineHeight: 1.7, color: "var(--color-desk-charcoal)" }}>
          {content.relationshipSummary}
        </p>
        {content.engagementCadence && (
          <p style={{ fontSize: "0.85rem", lineHeight: 1.6, color: "var(--color-desk-charcoal)", opacity: 0.8, marginTop: "0.5rem" }}>
            {content.engagementCadence}
          </p>
        )}
      </ReportSection>

      {content.customerQuote && (
        <ReportSection heading="Customer Voice">
          <p style={{ fontFamily: "var(--font-editorial)", fontStyle: "italic", fontSize: "1rem", lineHeight: 1.7, color: "var(--color-desk-charcoal)" }}>
            "{content.customerQuote}"
          </p>
        </ReportSection>
      )}

      {content.whatIsWorking.length > 0 && (
        <ReportSection heading="What Is Working">
          <ul style={{ paddingLeft: "1.25rem", fontSize: "0.9rem", lineHeight: 1.8, color: "var(--color-desk-charcoal)" }}>
            {content.whatIsWorking.map((item, i) => (
              <li key={i} style={{ marginBottom: "0.25rem" }}>{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.whatIsStruggling.length > 0 && (
        <ReportSection heading="What Is Struggling">
          <ul style={{ paddingLeft: "1.25rem", fontSize: "0.9rem", lineHeight: 1.8, color: "var(--color-desk-charcoal)" }}>
            {content.whatIsStruggling.map((item, i) => (
              <li key={i} style={{ marginBottom: "0.25rem" }}>{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.valueDelivered.length > 0 && (
        <ReportSection heading="Value Delivered">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {content.valueDelivered.map((signal, i) => (
              <div key={i} style={{ padding: "0.5rem 0", borderBottom: "1px solid var(--color-paper-linen)" }}>
                <p style={{ margin: 0, fontSize: "0.9rem", color: "var(--color-desk-charcoal)" }}>{signal.text}</p>
                {signal.source && (
                  <p style={{ margin: "0.2rem 0 0", fontSize: "0.75rem", fontFamily: "var(--font-mono)", color: "var(--color-desk-charcoal)", opacity: 0.5 }}>
                    {signal.source}
                  </p>
                )}
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {content.risks.length > 0 && (
        <ReportSection heading="Risks">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {content.risks.map((r, i) => (
              <div key={i} style={{
                padding: "0.75rem 1rem",
                borderLeft: "3px solid var(--color-spice-terracotta)",
                background: "var(--color-paper-warm-white)",
              }}>
                <p style={{ margin: 0, fontSize: "0.875rem", color: "var(--color-desk-charcoal)" }}>{r.risk}</p>
                <span style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: "0.7rem",
                  textTransform: "uppercase",
                  color: r.status === "resolved" ? "var(--color-garden-sage)" : r.status === "mitigated" ? "var(--color-spice-saffron)" : "var(--color-spice-terracotta)",
                }}>
                  {r.status}
                </span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {content.expansionSignals.length > 0 && (
        <ReportSection heading="Expansion Indicators">
          <ul style={{ paddingLeft: "1.25rem", fontSize: "0.9rem", lineHeight: 1.8, color: "var(--color-desk-charcoal)" }}>
            {content.expansionSignals.map((item, i) => (
              <li key={i} style={{ marginBottom: "0.25rem" }}>{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.renewalContext && (
        <ReportSection heading="Renewal Context">
          <p style={{ fontSize: "0.9rem", lineHeight: 1.7, color: "var(--color-desk-charcoal)" }}>
            {content.renewalContext}
          </p>
        </ReportSection>
      )}

      {content.recommendedActions.length > 0 && (
        <ReportSection heading="Recommended Actions">
          <ol style={{ paddingLeft: "1.25rem", fontSize: "0.9rem", lineHeight: 1.8, color: "var(--color-desk-charcoal)" }}>
            {content.recommendedActions.map((action, i) => (
              <li key={i} style={{ marginBottom: "0.25rem" }}>{action}</li>
            ))}
          </ol>
        </ReportSection>
      )}
    </div>
  );
}
