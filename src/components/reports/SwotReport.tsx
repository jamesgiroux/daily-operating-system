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
      style={{
        padding: "1.25rem",
        border: "1px solid var(--color-paper-linen)",
        borderTop: `3px solid ${accentColor}`,
        borderRadius: "2px",
        background: "var(--color-paper-warm-white)",
      }}
    >
      <h3
        style={{
          fontFamily: "var(--font-editorial)",
          fontWeight: 600,
          color: accentColor,
          marginBottom: "0.75rem",
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          fontSize: "0.8rem",
        }}
      >
        {title}
      </h3>
      {items.length === 0 ? (
        <p
          style={{
            color: "var(--color-desk-charcoal)",
            opacity: 0.5,
            fontSize: "0.875rem",
          }}
        >
          No items identified.
        </p>
      ) : (
        <ul
          style={{
            listStyle: "none",
            padding: 0,
            margin: 0,
            display: "flex",
            flexDirection: "column",
            gap: "0.5rem",
          }}
        >
          {items.map((item, i) => (
            <li
              key={i}
              style={{ display: "flex", alignItems: "flex-start", gap: "0.5rem" }}
            >
              <span
                style={{
                  color: accentColor,
                  marginTop: "2px",
                  flexShrink: 0,
                }}
              >
                ›
              </span>
              <span
                style={{
                  fontSize: "0.875rem",
                  color: "var(--color-desk-charcoal)",
                  lineHeight: 1.5,
                }}
              >
                {item.text}
                {item.source && (
                  <span
                    style={{
                      display: "inline-block",
                      marginLeft: "0.4rem",
                      padding: "1px 5px",
                      background: "var(--color-paper-linen)",
                      borderRadius: "3px",
                      fontSize: "0.7rem",
                      color: "var(--color-desk-charcoal)",
                      opacity: 0.7,
                      fontFamily: "var(--font-mono)",
                    }}
                  >
                    {item.source}
                  </span>
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
    <div style={{ padding: "2rem" }}>
      {content.summary && (
        <ReportSection heading="Executive Summary">
          <p
            style={{
              fontFamily: "var(--font-editorial)",
              fontSize: "1.1rem",
              lineHeight: 1.7,
              color: "var(--color-desk-charcoal)",
              maxWidth: "65ch",
            }}
          >
            {content.summary}
          </p>
        </ReportSection>
      )}

      <ReportSection heading="SWOT Analysis">
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: "1rem",
          }}
        >
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
