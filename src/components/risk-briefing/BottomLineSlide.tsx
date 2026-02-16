/**
 * BottomLineSlide — The whole story in one breath.
 * Slide 2: headline + risk badge + renewal window.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { RiskBottomLine } from "@/types";

interface BottomLineSlideProps {
  data: RiskBottomLine;
  onUpdate?: (data: RiskBottomLine) => void;
}

const riskColors: Record<string, string> = {
  high: "var(--color-spice-chili)",
  medium: "var(--color-spice-terracotta)",
  low: "var(--color-garden-sage)",
};

export function BottomLineSlide({ data, onUpdate }: BottomLineSlideProps) {
  const riskKey = data.riskLevel?.toLowerCase() ?? "";

  return (
    <section
      id="bottom-line"
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-terracotta)",
          marginBottom: 24,
        }}
      >
        Bottom Line
      </div>

      {/* Headline — the whole story */}
      <EditableText
        as="h2"
        value={data.headline}
        onChange={(v) => onUpdate?.({ ...data, headline: v })}
        multiline
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 36,
          fontWeight: 400,
          lineHeight: 1.3,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: "0 0 32px",
        }}
      />

      {/* Risk badge + renewal window */}
      <div style={{ display: "flex", gap: 24, alignItems: "center" }}>
        {data.riskLevel && (
          <div style={{ display: "inline-flex", alignItems: "center", gap: 10 }}>
            <span
              style={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                background: riskColors[riskKey] ?? "var(--color-text-secondary)",
              }}
            />
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: riskColors[riskKey] ?? "var(--color-text-secondary)",
              }}
            >
              {data.riskLevel} risk
            </span>
          </div>
        )}
        {data.renewalWindow && (
          <EditableText
            value={data.renewalWindow}
            onChange={(v) => onUpdate?.({ ...data, renewalWindow: v })}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
              letterSpacing: "0.04em",
            }}
          />
        )}
      </div>
    </section>
  );
}
