import type { CSSProperties } from "react";
import { ReportSection } from "./ReportSection";
import type { SwotContent, SwotItem } from "@/types/reports";

interface SwotQuadrantProps {
  title: string;
  items: SwotItem[];
  accentColor: string;
}

function SwotQuadrant({ title, items, accentColor }: SwotQuadrantProps) {
  return (
    <div
      className="report-surface-swot-quadrant"
      style={{ "--report-surface-quadrant-accent": accentColor } as CSSProperties}
    >
      <h3 className="report-surface-swot-title">{title}</h3>
      {items.length === 0 ? (
        <p className="report-surface-swot-empty">No items identified.</p>
      ) : (
        <ul className="report-surface-swot-list">
          {items.map((item, i) => (
            <li key={i} className="report-surface-swot-item">
              <span className="report-surface-swot-bullet">›</span>
              <span className="report-surface-swot-text">
                {item.text}
                {item.source && (
                  <span className="report-surface-swot-source">{item.source}</span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

interface SwotReportProps {
  content: SwotContent;
}

export function SwotReport({ content }: SwotReportProps) {
  return (
    <div className="report-surface-page">
      {content.summary && (
        <ReportSection heading="Executive Summary">
          <p className="report-surface-body-lg">{content.summary}</p>
        </ReportSection>
      )}

      <ReportSection heading="SWOT Analysis">
        <div className="report-surface-swot-grid">
          <SwotQuadrant
            title="Strengths"
            items={content.strengths}
            accentColor="var(--color-garden-sage)"
          />
          <SwotQuadrant
            title="Weaknesses"
            items={content.weaknesses}
            accentColor="var(--color-spice-terracotta)"
          />
          <SwotQuadrant
            title="Opportunities"
            items={content.opportunities}
            accentColor="var(--color-garden-larkspur)"
          />
          <SwotQuadrant
            title="Threats"
            items={content.threats}
            accentColor="var(--color-spice-chili)"
          />
        </div>
      </ReportSection>
    </div>
  );
}
