import type { CSSProperties } from "react";
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
    <div className="report-surface-page report-customer-facing">
      {/* Header */}
      <div className="report-surface-section" style={{ borderBottom: "2px solid var(--color-spice-turmeric)", paddingBottom: "1rem" }}>
        <h1 className="report-surface-slide-title" style={{ fontSize: "2rem", marginBottom: 0, maxWidth: "none" }}>
          Executive Business Review
        </h1>
        <p className="report-surface-body-muted" style={{ marginTop: "0.25rem" }}>
          {content.quarterLabel}
        </p>
      </div>

      {/* 1. Executive Summary */}
      <ReportSection heading="Executive Summary">
        <p className="report-surface-body-lg">{content.executiveSummary}</p>
      </ReportSection>

      {/* 2. Story Bullets */}
      {content.storyBullets.length > 0 && (
        <ReportSection heading="Quarter in Brief">
          <ul className="report-surface-list">
            {content.storyBullets.map((bullet, i) => (
              <li key={i} className="report-surface-list-item">{bullet}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {/* 3. Customer Quote */}
      {content.customerQuote && (
        <ReportSection heading="Customer Voice">
          <p className="report-surface-quote">
            "{content.customerQuote}"
          </p>
        </ReportSection>
      )}

      {/* 4. Value Delivered */}
      {content.valueDelivered.length > 0 && (
        <ReportSection heading="Value Delivered">
          <div className="report-surface-stack" style={{ gap: "0.75rem" }}>
            {content.valueDelivered.map((item, i) => (
              <div
                key={i}
                className="report-surface-callout"
                style={{ "--report-surface-callout-accent": "var(--color-garden-sage)" } as CSSProperties}
              >
                <p className="report-surface-callout-title">{item.outcome}</p>
                {item.impact && (
                  <p className="report-surface-callout-impact">
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
          <table className="report-surface-table">
            <thead>
              <tr className="report-surface-table-row">
                {["Metric", "Baseline", "Current", "Trend"].map(h => (
                  <th key={h} className="report-surface-table-head">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {content.successMetrics.map((m, i) => (
                <tr key={i} className="report-surface-table-row">
                  <td className="report-surface-table-cell">{m.metric}</td>
                  <td className="report-surface-table-cell report-surface-table-cellMuted">{m.baseline ?? "—"}</td>
                  <td className="report-surface-table-cell report-surface-table-cellStrong">{m.current}</td>
                  <td className="report-surface-table-cell">
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
          <div className="report-surface-stack">
            {content.challengesAndResolutions.map((r, i) => (
              <div
                key={i}
                className="report-surface-callout"
                style={{ "--report-surface-callout-accent": STATUS_COLORS[r.status] ?? "var(--color-paper-linen)" } as CSSProperties}
              >
                <p className="report-surface-callout-text">
                  <strong>{r.risk}</strong>
                </p>
                {r.resolution && (
                  <p className="report-surface-callout-meta">
                    → {r.resolution}
                  </p>
                )}
                <span
                  className="report-surface-tag"
                  style={{ "--report-surface-status-color": STATUS_COLORS[r.status] ?? "var(--color-desk-charcoal)" } as CSSProperties}
                >
                  {r.status}
                </span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {/* 7. Strategic Roadmap */}
      <ReportSection heading="Strategic Roadmap">
        <p className="report-surface-body">
          {content.strategicRoadmap}
        </p>
      </ReportSection>

      {/* 8. Next Steps */}
      {content.nextSteps.length > 0 && (
        <ReportSection heading="Next Steps">
          <div className="report-surface-stack">
            {content.nextSteps.map((step, i) => (
              <div key={i} className="report-surface-next-step">
                <span className="report-surface-owner">{step.owner}</span>
                <span className="report-surface-callout-text" style={{ flex: 1 }}>
                  {step.action}
                </span>
                <span className="report-surface-timeline">{step.timeline}</span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}
    </div>
  );
}
