/**
 * BriefingCallouts — signal-driven intelligence callouts for the daily briefing (I308).
 *
 * Renders severity-coded callout items with editorial styling:
 * - Critical (terracotta left border): stakeholder departure, renewal risk
 * - Warning (turmeric left border): engagement decline, champion risk
 * - Info (sage left border): follow-up received, health concern
 */
import type { BriefingCallout } from "../../types/callout";

const MAX_CALLOUTS = 5;

const categoryMap: [RegExp, string][] = [
  [/meeting frequency|champion.*cold|no contact|account.*dark|stakeholder|champion risk|engagement/i, "RELATIONSHIP"],
  [/renewal|project health|action overload/i, "PORTFOLIO"],
  [/prep coverage|heavy week|follow-up/i, "READINESS"],
  [/email|spike/i, "ACTIVITY"],
];

function categorize(headline: string): string {
  for (const [pattern, category] of categoryMap) {
    if (pattern.test(headline)) return category;
  }
  return "SIGNAL";
}

interface BriefingCalloutsProps {
  callouts: BriefingCallout[];
}

const severityColors: Record<string, string> = {
  critical: "var(--color-spice-terracotta)",
  warning: "var(--color-spice-turmeric)",
  info: "var(--color-garden-sage)",
};

const severityLabels: Record<string, string> = {
  critical: "Critical",
  warning: "Watch",
  info: "Note",
};

export function BriefingCallouts({ callouts }: BriefingCalloutsProps) {
  if (!callouts || callouts.length === 0) return null;

  return (
    <section style={{ marginBottom: 32 }}>
      <h3
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 18,
          fontWeight: 400,
          color: "var(--color-text-primary)",
          margin: "0 0 16px",
          paddingBottom: 8,
          borderBottom: "2px solid var(--color-text-primary)",
        }}
      >
        Intelligence Signals
      </h3>
      <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
        {callouts.slice(0, MAX_CALLOUTS).map((callout) => (
          <CalloutItem key={callout.id} callout={callout} />
        ))}
        {callouts.length > MAX_CALLOUTS && (
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              margin: "8px 0 0",
              paddingLeft: 12,
            }}
          >
            +{callouts.length - MAX_CALLOUTS} more signal
            {callouts.length - MAX_CALLOUTS > 1 ? "s" : ""}
          </p>
        )}
      </div>
    </section>
  );
}

function CalloutItem({ callout }: { callout: BriefingCallout }) {
  const borderColor =
    severityColors[callout.severity] ?? "var(--color-text-tertiary)";
  const label = severityLabels[callout.severity] ?? "Signal";

  return (
    <div
      style={{
        display: "flex",
        gap: 12,
        alignItems: "flex-start",
        paddingLeft: 12,
        paddingBottom: 16,
        marginBottom: 16,
        borderLeft: `3px solid ${borderColor}`,
        borderBottom: "1px solid var(--color-rule-light)",
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: borderColor,
            }}
          >
            {label}
          </span>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 9,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: "var(--color-text-tertiary)",
              opacity: 0.7,
            }}
          >
            {categorize(callout.headline)}
          </span>
          {callout.entityName && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                color: "var(--color-text-tertiary)",
              }}
            >
              {callout.entityName}
            </span>
          )}
        </div>
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 15,
            fontWeight: 500,
            lineHeight: 1.4,
            color: "var(--color-text-primary)",
            margin: "4px 0 0",
          }}
        >
          {callout.headline}
        </p>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            lineHeight: 1.5,
            color: "var(--color-text-secondary)",
            margin: "4px 0 0",
          }}
        >
          {callout.detail}
        </p>
      </div>
    </div>
  );
}
