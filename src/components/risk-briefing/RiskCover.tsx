/**
 * RiskCover — full-viewport hero with serif title, risk badge, ARR callout.
 * Slide 1: the cover page of the 6-slide risk briefing.
 */
import type { RiskCover as RiskCoverData } from "@/types";
import { formatArr } from "@/lib/utils";
import { EditableText } from "@/components/ui/EditableText";

interface RiskCoverProps {
  data: RiskCoverData;
  onUpdate?: (data: RiskCoverData) => void;
}

const riskColors: Record<string, string> = {
  high: "var(--color-spice-chili)",
  medium: "var(--color-spice-terracotta)",
  low: "var(--color-garden-sage)",
};

export function RiskCover({ data, onUpdate }: RiskCoverProps) {
  const riskKey = data.riskLevel?.toLowerCase() ?? "";

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Overline label */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-terracotta)",
          marginBottom: 24,
        }}
      >
        Risk Briefing
      </div>

      {/* Account name */}
      <h1
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 52,
          fontWeight: 400,
          lineHeight: 1.1,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 24px",
          maxWidth: 700,
        }}
      >
        {data.accountName}
      </h1>

      {/* Risk level badge */}
      {data.riskLevel && (
        <div
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 8,
            marginBottom: 24,
          }}
        >
          <span
            style={{
              width: 10,
              height: 10,
              borderRadius: "50%",
              background: riskColors[riskKey] ?? "var(--color-text-tertiary)",
            }}
          />
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: riskColors[riskKey] ?? "var(--color-text-tertiary)",
            }}
          >
            {data.riskLevel} risk
          </span>
        </div>
      )}

      {/* ARR at risk callout */}
      {data.arrAtRisk != null && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 28,
            fontWeight: 600,
            color: "var(--color-spice-terracotta)",
            marginBottom: 32,
          }}
        >
          ${formatArr(data.arrAtRisk)} ARR at Risk
        </div>
      )}

      {/* Date + TAM + Regenerate */}
      <div
        style={{
          display: "flex",
          gap: 24,
          alignItems: "baseline",
        }}
      >
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.04em",
          }}
        >
          {data.date}
        </span>
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-secondary)",
            display: "inline-flex",
            alignItems: "baseline",
            gap: 4,
          }}
        >
          Prepared by{" "}
          <EditableText
            value={data.tamName || ""}
            onChange={(v) => onUpdate?.({ ...data, tamName: v })}
            multiline={false}
            placeholder="Add name"
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
            }}
          />
        </span>
      </div>
    </div>
  );
}
