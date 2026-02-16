/**
 * RiskCover — full-viewport hero with serif title, risk badge, ARR callout.
 * Slide 1: the cover page of the 6-slide risk briefing.
 */
import type { RiskCover as RiskCoverData } from "@/types";
import { formatArr } from "@/lib/utils";

interface RiskCoverProps {
  data: RiskCoverData;
  onRegenerate?: () => void;
  regenerating?: boolean;
}

const riskColors: Record<string, string> = {
  high: "var(--color-spice-chili)",
  medium: "var(--color-spice-terracotta)",
  low: "var(--color-garden-sage)",
};

export function RiskCover({ data, onRegenerate, regenerating }: RiskCoverProps) {
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
        {data.tamName && (
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
            }}
          >
            Prepared by {data.tamName}
          </span>
        )}
        {onRegenerate && (
          <button
            onClick={onRegenerate}
            disabled={regenerating}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "1px solid var(--color-rule-light)",
              borderRadius: 4,
              padding: "4px 12px",
              cursor: regenerating ? "not-allowed" : "pointer",
              opacity: regenerating ? 0.5 : 1,
            }}
          >
            Regenerate
          </button>
        )}
      </div>
    </div>
  );
}
