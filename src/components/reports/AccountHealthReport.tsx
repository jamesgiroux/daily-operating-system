import type { CSSProperties } from "react";
import { ReportSection } from "./ReportSection";
import type { AccountHealthContent } from "@/types/reports";

interface AccountHealthReportProps {
  content: AccountHealthContent;
}

export function AccountHealthReport({ content }: AccountHealthReportProps) {
  return (
    <div className="report-surface-page">
      <ReportSection heading="Overall Assessment">
        <p className="report-surface-body-lg">{content.overallAssessment}</p>
        {content.healthScoreNarrative && (
          <p className="report-surface-body-muted" style={{ marginTop: "0.75rem" }}>
            {content.healthScoreNarrative}
          </p>
        )}
      </ReportSection>

      <ReportSection heading="Relationship">
        <p className="report-surface-body">{content.relationshipSummary}</p>
        {content.engagementCadence && (
          <p className="report-surface-body-muted" style={{ marginTop: "0.5rem" }}>
            {content.engagementCadence}
          </p>
        )}
      </ReportSection>

      {content.customerQuote && (
        <ReportSection heading="Customer Voice">
          <p className="report-surface-quote">
            "{content.customerQuote}"
          </p>
        </ReportSection>
      )}

      {content.whatIsWorking.length > 0 && (
        <ReportSection heading="What Is Working">
          <ul className="report-surface-list">
            {content.whatIsWorking.map((item, i) => (
              <li key={i} className="report-surface-list-item">{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.whatIsStruggling.length > 0 && (
        <ReportSection heading="What Is Struggling">
          <ul className="report-surface-list">
            {content.whatIsStruggling.map((item, i) => (
              <li key={i} className="report-surface-list-item">{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.valueDelivered.length > 0 && (
        <ReportSection heading="Value Delivered">
          <div className="report-surface-stack">
            {content.valueDelivered.map((signal, i) => (
              <div key={i} className="report-surface-table-row" style={{ padding: "0.5rem 0" }}>
                <p className="report-surface-callout-text">{signal.text}</p>
                {signal.source && (
                  <p className="report-surface-source">
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
          <div className="report-surface-stack">
            {content.risks.map((r, i) => (
              <div
                key={i}
                className="report-surface-callout"
                style={{ "--report-surface-callout-accent": "var(--color-spice-terracotta)" } as CSSProperties}
              >
                <p className="report-surface-callout-text">{r.risk}</p>
                <span
                  className="report-surface-tag"
                  style={{
                    "--report-surface-status-color":
                      r.status === "resolved"
                        ? "var(--color-garden-sage)"
                        : r.status === "mitigated"
                          ? "var(--color-spice-saffron)"
                          : "var(--color-spice-terracotta)",
                  } as CSSProperties}
                >
                  {r.status}
                </span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {content.expansionSignals.length > 0 && (
        <ReportSection heading="Expansion Indicators">
          <ul className="report-surface-list">
            {content.expansionSignals.map((item, i) => (
              <li key={i} className="report-surface-list-item">{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.renewalContext && (
        <ReportSection heading="Renewal Context">
          <p className="report-surface-body">
            {content.renewalContext}
          </p>
        </ReportSection>
      )}

      {content.recommendedActions.length > 0 && (
        <ReportSection heading="Recommended Actions">
          <ol className="report-surface-ordered-list">
            {content.recommendedActions.map((action, i) => (
              <li key={i} className="report-surface-list-item">{action}</li>
            ))}
          </ol>
        </ReportSection>
      )}
    </div>
  );
}
