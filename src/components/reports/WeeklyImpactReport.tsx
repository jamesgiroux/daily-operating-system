import { ReportSection } from "./ReportSection";

interface PriorityMove {
  priorityText: string;
  whatHappened: string;
  source: string;
}

interface WeeklyImpactContent {
  weekLabel: string;
  headlineStat: string;
  prioritiesMoved: PriorityMove[];
  wins: string[];
  activitySummary: string;
  watch: string[];
  carryForward: string[];
}

interface WeeklyImpactReportProps {
  content: WeeklyImpactContent;
}

export function WeeklyImpactReport({ content }: WeeklyImpactReportProps) {
  return (
    <div style={{ padding: "2rem", maxWidth: "750px" }}>
      {/* Headline stat */}
      <div
        style={{
          marginBottom: "2rem",
          padding: "1.25rem",
          background: "var(--color-garden-eucalyptus)",
          borderRadius: "4px",
          color: "white",
        }}
      >
        <p
          style={{
            fontFamily: "var(--font-editorial)",
            fontSize: "1.5rem",
            fontWeight: 400,
            margin: 0,
            marginBottom: "0.25rem",
          }}
        >
          {content.weekLabel}
        </p>
        <p style={{ margin: 0, opacity: 0.9, fontSize: "1rem" }}>
          {content.headlineStat}
        </p>
      </div>

      {content.prioritiesMoved.length > 0 && (
        <ReportSection heading="Priorities Moved">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {content.prioritiesMoved.map((move, i) => (
              <div
                key={i}
                style={{
                  padding: "0.75rem 1rem",
                  borderLeft: "3px solid var(--color-garden-eucalyptus)",
                  background: "var(--color-paper-warm-white)",
                }}
              >
                <p
                  style={{
                    fontFamily: "var(--font-editorial)",
                    fontStyle: "italic",
                    fontSize: "0.875rem",
                    color: "var(--color-desk-charcoal)",
                    opacity: 0.7,
                    margin: 0,
                    marginBottom: "0.25rem",
                  }}
                >
                  {move.priorityText}
                </p>
                <p
                  style={{
                    margin: 0,
                    fontSize: "0.9rem",
                    color: "var(--color-desk-charcoal)",
                  }}
                >
                  {move.whatHappened}
                </p>
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: "0.7rem",
                    color: "var(--color-desk-charcoal)",
                    opacity: 0.5,
                  }}
                >
                  {move.source}
                </span>
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {content.wins.length > 0 && (
        <ReportSection heading="Wins">
          <ul
            style={{
              paddingLeft: "1.25rem",
              fontSize: "0.9rem",
              lineHeight: 1.8,
              color: "var(--color-desk-charcoal)",
            }}
          >
            {content.wins.map((win, i) => (
              <li key={i}>{win}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      <ReportSection heading="What You Did">
        <p
          style={{
            fontSize: "0.9rem",
            lineHeight: 1.7,
            color: "var(--color-desk-charcoal)",
          }}
        >
          {content.activitySummary}
        </p>
      </ReportSection>

      {content.watch.length > 0 && (
        <ReportSection heading="Watch">
          <ul
            style={{
              paddingLeft: "1.25rem",
              fontSize: "0.875rem",
              lineHeight: 1.8,
              color: "var(--color-desk-charcoal)",
            }}
          >
            {content.watch.map((item, i) => (
              <li key={i}>{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}

      {content.carryForward.length > 0 && (
        <ReportSection heading="Carry Forward">
          <p
            style={{
              fontSize: "0.8rem",
              color: "var(--color-desk-charcoal)",
              opacity: 0.6,
              marginBottom: "0.5rem",
            }}
          >
            Priorities with no activity this week:
          </p>
          <ul
            style={{
              paddingLeft: "1.25rem",
              fontSize: "0.875rem",
              lineHeight: 1.8,
              color: "var(--color-desk-charcoal)",
              opacity: 0.8,
            }}
          >
            {content.carryForward.map((item, i) => (
              <li key={i}>{item}</li>
            ))}
          </ul>
        </ReportSection>
      )}
    </div>
  );
}
