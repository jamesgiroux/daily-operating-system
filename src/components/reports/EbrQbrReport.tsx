import { ReportSection } from "./ReportSection";

interface EbrQbrMetric {
  metric: string;
  baseline?: string | null;
  current: string;
  trend?: string | null;
}

interface EbrQbrValueItem {
  outcome: string;
  source: string;
  impact?: string | null;
}

interface EbrQbrRisk {
  risk: string;
  resolution?: string | null;
  status: string;
}

interface EbrQbrAction {
  action: string;
  owner: string;
  timeline: string;
}

interface EbrQbrContent {
  quarterLabel: string;
  executiveSummary: string;
  storyBullets: string[];
  customerQuote: string | null;
  valueDelivered: EbrQbrValueItem[];
  successMetrics: EbrQbrMetric[];
  challengesAndResolutions: EbrQbrRisk[];
  strategicRoadmap: string;
  nextSteps: EbrQbrAction[];
}

const TREND_ICONS: Record<string, string> = { up: "↑", down: "↓", stable: "→" };
const STATUS_COLORS: Record<string, string> = {
  resolved: "var(--color-garden-sage)",
  open: "var(--color-spice-terracotta)",
  mitigated: "var(--color-spice-saffron)",
};

interface EbrQbrReportProps {
  content: EbrQbrContent;
}

export function EbrQbrReport({ content }: EbrQbrReportProps) {
  return (
    <div className="report-customer-facing" style={{ padding: "2rem", maxWidth: "800px" }}>
      {/* Header */}
      <div style={{ marginBottom: "2.5rem", borderBottom: "2px solid var(--color-spice-turmeric)", paddingBottom: "1rem" }}>
        <h1 style={{
          fontFamily: "var(--font-editorial)",
          fontSize: "2rem",
          fontWeight: 400,
          color: "var(--color-desk-charcoal)",
          margin: 0,
        }}>
          Executive Business Review
        </h1>
        <p style={{ margin: "0.25rem 0 0", fontSize: "1rem", color: "var(--color-desk-charcoal)", opacity: 0.7 }}>
          {content.quarterLabel}
        </p>
      </div>

      {/* 1. Executive Summary */}
      <ReportSection heading="Executive Summary">
        <p style={{ fontFamily: "var(--font-editorial)", fontSize: "1.1rem", lineHeight: 1.7, color: "var(--color-desk-charcoal)", maxWidth: "65ch" }}>
          {content.executiveSummary}
        </p>
      </ReportSection>

      {/* 2. Story Bullets */}
      {content.storyBullets.length > 0 && (
        <ReportSection heading="Quarter in Brief">
          <ul style={{ paddingLeft: "1.25rem", fontSize: "0.9rem", lineHeight: 1.8, color: "var(--color-desk-charcoal)" }}>
            {content.storyBullets.map((bullet, i) => (
              <li key={i} style={{ marginBottom: "0.25rem" }}>{bullet}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {/* 3. Customer Quote */}
      {content.customerQuote && (
        <ReportSection heading="Customer Voice">
          <p style={{ fontFamily: "var(--font-editorial)", fontStyle: "italic", fontSize: "1rem", lineHeight: 1.7, color: "var(--color-desk-charcoal)" }}>
            "{content.customerQuote}"
          </p>
        </ReportSection>
      )}

      {/* 4. Value Delivered */}
      {content.valueDelivered.length > 0 && (
        <ReportSection heading="Value Delivered">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {content.valueDelivered.map((item, i) => (
              <div key={i} style={{
                padding: "0.75rem 1rem",
                borderLeft: "3px solid var(--color-garden-sage)",
                background: "var(--color-paper-warm-white)",
              }}>
                <p style={{ margin: 0, fontSize: "0.9rem", color: "var(--color-desk-charcoal)", fontWeight: 500 }}>
                  {item.outcome}
                </p>
                {item.impact && (
                  <p style={{ margin: "0.25rem 0 0", fontSize: "0.8rem", color: "var(--color-garden-rosemary)" }}>
                    {item.impact}
                  </p>
                )}
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {/* 5. Success Metrics — table format */}
      {content.successMetrics.length > 0 && (
        <ReportSection heading="Success Metrics">
          <table style={{
            width: "100%",
            borderCollapse: "collapse",
            fontSize: "0.875rem",
          }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--color-paper-linen)" }}>
                {["Metric", "Baseline", "Current", "Trend"].map(h => (
                  <th key={h} style={{
                    textAlign: "left",
                    padding: "0.5rem 0.75rem",
                    fontFamily: "var(--font-mono)",
                    fontSize: "0.75rem",
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                    color: "var(--color-desk-charcoal)",
                    opacity: 0.6,
                  }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {content.successMetrics.map((m, i) => (
                <tr key={i} style={{ borderBottom: "1px solid var(--color-paper-linen)" }}>
                  <td style={{ padding: "0.5rem 0.75rem", color: "var(--color-desk-charcoal)" }}>{m.metric}</td>
                  <td style={{ padding: "0.5rem 0.75rem", color: "var(--color-desk-charcoal)", opacity: 0.7 }}>{m.baseline ?? "—"}</td>
                  <td style={{ padding: "0.5rem 0.75rem", color: "var(--color-desk-charcoal)", fontWeight: 500 }}>{m.current}</td>
                  <td style={{ padding: "0.5rem 0.75rem", color: "var(--color-desk-charcoal)" }}>
                    {m.trend ? TREND_ICONS[m.trend] ?? m.trend : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </ReportSection>
      )}

      {/* 6. Challenges & Resolutions */}
      {content.challengesAndResolutions.length > 0 && (
        <ReportSection heading="Challenges & Resolutions">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {content.challengesAndResolutions.map((r, i) => (
              <div key={i} style={{
                padding: "0.75rem 1rem",
                borderLeft: `3px solid ${STATUS_COLORS[r.status] ?? "var(--color-paper-linen)"}`,
                background: "var(--color-paper-warm-white)",
              }}>
                <p style={{ margin: 0, fontSize: "0.875rem", color: "var(--color-desk-charcoal)" }}>
                  <strong>{r.risk}</strong>
                </p>
                {r.resolution && (
                  <p style={{ margin: "0.25rem 0 0", fontSize: "0.8rem", color: "var(--color-desk-charcoal)", opacity: 0.8 }}>
                    → {r.resolution}
                  </p>
                )}
                <span style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: "0.7rem",
                  textTransform: "uppercase",
                  color: STATUS_COLORS[r.status] ?? "var(--color-desk-charcoal)",
                }}>
                  {r.status}
                </span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {/* 7. Strategic Roadmap */}
      <ReportSection heading="Strategic Roadmap">
        <p style={{ fontSize: "0.9rem", lineHeight: 1.7, color: "var(--color-desk-charcoal)" }}>
          {content.strategicRoadmap}
        </p>
      </ReportSection>

      {/* 8. Next Steps */}
      {content.nextSteps.length > 0 && (
        <ReportSection heading="Next Steps">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {content.nextSteps.map((step, i) => (
              <div key={i} style={{
                display: "flex",
                gap: "1rem",
                alignItems: "flex-start",
                padding: "0.5rem 0",
                borderBottom: "1px solid var(--color-paper-linen)",
              }}>
                <span style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: "0.75rem",
                  color: "var(--color-spice-turmeric)",
                  minWidth: "80px",
                  paddingTop: "1px",
                }}>
                  {step.owner}
                </span>
                <span style={{ fontSize: "0.875rem", color: "var(--color-desk-charcoal)", flex: 1 }}>
                  {step.action}
                </span>
                <span style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: "0.7rem",
                  color: "var(--color-desk-charcoal)",
                  opacity: 0.5,
                  minWidth: "90px",
                  textAlign: "right",
                }}>
                  {step.timeline}
                </span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}
    </div>
  );
}
