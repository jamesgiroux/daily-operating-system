import { ReportSection } from "./ReportSection";

interface MonthlyWin {
  headline: string;
  detail?: string | null;
  source: string;
}

interface PriorityProgress {
  priorityText: string;
  progress: "strong" | "some" | "none";
  evidence?: string | null;
}

interface MonthlyWrappedContent {
  monthLabel: string;
  headlineStat: string;
  openingReflection: string;
  topWins: MonthlyWin[];
  priorityProgress: PriorityProgress[];
  honestMiss?: string | null;
  momentumBuilder: string;
  byTheNumbers: string[];
}

const PROGRESS_COLORS: Record<string, string> = {
  strong: "var(--color-garden-sage)",
  some: "var(--color-spice-saffron)",
  none: "var(--color-desk-charcoal)",
};

const PROGRESS_LABELS: Record<string, string> = {
  strong: "Strong progress",
  some: "Some progress",
  none: "No activity",
};

interface MonthlyWrappedReportProps {
  content: MonthlyWrappedContent;
}

export function MonthlyWrappedReport({ content }: MonthlyWrappedReportProps) {
  return (
    <div style={{ padding: "2rem", maxWidth: "750px" }}>
      {/* Month header */}
      <div
        style={{
          marginBottom: "2.5rem",
          padding: "1.5rem 2rem",
          background:
            "linear-gradient(135deg, var(--color-garden-eucalyptus) 0%, var(--color-garden-sage) 100%)",
          borderRadius: "4px",
          color: "white",
        }}
      >
        <h1
          style={{
            fontFamily: "var(--font-editorial)",
            fontSize: "2rem",
            fontWeight: 400,
            margin: 0,
            marginBottom: "0.5rem",
          }}
        >
          {content.monthLabel}
        </h1>
        <p style={{ margin: 0, opacity: 0.9 }}>{content.headlineStat}</p>
      </div>

      {/* Opening reflection */}
      <div style={{ marginBottom: "2rem" }}>
        <p
          style={{
            fontFamily: "var(--font-editorial)",
            fontSize: "1.15rem",
            lineHeight: 1.75,
            color: "var(--color-desk-charcoal)",
            fontStyle: "italic",
          }}
        >
          {content.openingReflection}
        </p>
      </div>

      {/* By the numbers */}
      {content.byTheNumbers.length > 0 && (
        <div
          style={{
            display: "flex",
            gap: "1.5rem",
            marginBottom: "2rem",
            padding: "1rem",
            background: "var(--color-paper-warm-white)",
            borderRadius: "2px",
            flexWrap: "wrap",
          }}
        >
          {content.byTheNumbers.map((stat, i) => (
            <span
              key={i}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: "0.8rem",
                color: "var(--color-desk-charcoal)",
                opacity: 0.7,
              }}
            >
              {stat}
            </span>
          ))}
        </div>
      )}

      {/* Top Wins */}
      {content.topWins.length > 0 && (
        <ReportSection heading="Highlights">
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            {content.topWins.map((win, i) => (
              <div
                key={i}
                style={{
                  padding: "1rem",
                  borderLeft: "3px solid var(--color-garden-sage)",
                  background: "var(--color-paper-warm-white)",
                }}
              >
                <p
                  style={{
                    fontFamily: "var(--font-editorial)",
                    fontSize: "1rem",
                    fontWeight: 500,
                    margin: 0,
                    marginBottom: win.detail ? "0.35rem" : 0,
                    color: "var(--color-desk-charcoal)",
                  }}
                >
                  {win.headline}
                </p>
                {win.detail && (
                  <p
                    style={{
                      margin: 0,
                      fontSize: "0.875rem",
                      color: "var(--color-desk-charcoal)",
                      opacity: 0.8,
                      lineHeight: 1.6,
                    }}
                  >
                    {win.detail}
                  </p>
                )}
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {/* Priority progress */}
      {content.priorityProgress.length > 0 && (
        <ReportSection heading="Priority Progress">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {content.priorityProgress.map((p, i) => (
              <div
                key={i}
                style={{
                  display: "flex",
                  gap: "1rem",
                  padding: "0.75rem",
                  background: "var(--color-paper-warm-white)",
                  borderRadius: "2px",
                  alignItems: "flex-start",
                }}
              >
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: "0.7rem",
                    textTransform: "uppercase",
                    color: PROGRESS_COLORS[p.progress] ?? "var(--color-desk-charcoal)",
                    minWidth: "80px",
                    paddingTop: "2px",
                    flexShrink: 0,
                  }}
                >
                  {PROGRESS_LABELS[p.progress] ?? p.progress}
                </span>
                <div style={{ flex: 1 }}>
                  <p
                    style={{
                      margin: 0,
                      fontSize: "0.875rem",
                      color: "var(--color-desk-charcoal)",
                      fontWeight: 500,
                    }}
                  >
                    {p.priorityText}
                  </p>
                  {p.evidence && (
                    <p
                      style={{
                        margin: "0.25rem 0 0",
                        fontSize: "0.8rem",
                        color: "var(--color-desk-charcoal)",
                        opacity: 0.7,
                        lineHeight: 1.5,
                      }}
                    >
                      {p.evidence}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </ReportSection>
      )}

      {/* Honest miss */}
      {content.honestMiss && (
        <div
          style={{
            margin: "1.5rem 0",
            padding: "1rem 1.25rem",
            borderLeft: "3px solid var(--color-spice-terracotta)",
            background: "var(--color-paper-warm-white)",
          }}
        >
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: "0.7rem",
              textTransform: "uppercase",
              color: "var(--color-spice-terracotta)",
              margin: 0,
              marginBottom: "0.35rem",
              letterSpacing: "0.05em",
            }}
          >
            Carry forward
          </p>
          <p
            style={{
              margin: 0,
              fontSize: "0.875rem",
              color: "var(--color-desk-charcoal)",
              lineHeight: 1.6,
            }}
          >
            {content.honestMiss}
          </p>
        </div>
      )}

      {/* Momentum builder */}
      <div
        style={{
          marginTop: "2rem",
          paddingTop: "1.5rem",
          borderTop: "1px solid var(--color-paper-linen)",
        }}
      >
        <p
          style={{
            fontFamily: "var(--font-editorial)",
            fontSize: "1rem",
            lineHeight: 1.7,
            color: "var(--color-desk-charcoal)",
            opacity: 0.8,
            fontStyle: "italic",
          }}
        >
          {content.momentumBuilder}
        </p>
      </div>
    </div>
  );
}
