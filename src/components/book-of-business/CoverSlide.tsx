/**
 * CoverSlide — Full-viewport hero with title, vitals strip, executive summary.
 * Slide 1 of the Book of Business presentation.
 */
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent } from "@/types/reports";

interface CoverSlideProps {
  content: BookOfBusinessContent;
  isStale?: boolean;
  onRegenerate?: () => void;
  generating?: boolean;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function CoverSlide({ content, isStale, onRegenerate, generating, onUpdate }: CoverSlideProps) {
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
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-turmeric)",
          marginBottom: 24,
        }}
      >
        {content.periodLabel || "Book of Business"}
      </div>

      {/* Title */}
      <h1
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 52,
          fontWeight: 400,
          lineHeight: 1.1,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 32px",
          maxWidth: 700,
        }}
      >
        Book of Business
      </h1>

      {/* Staleness banner */}
      {isStale && (
        <div
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            color: "var(--color-spice-saffron)",
            background: "color-mix(in srgb, var(--color-spice-saffron) 8%, transparent)",
            border: "1px solid color-mix(in srgb, var(--color-spice-saffron) 20%, transparent)",
            borderRadius: 4,
            padding: "8px 16px",
            marginBottom: 24,
            display: "flex",
            alignItems: "center",
            gap: 16,
          }}
        >
          Account data has changed since this review was generated.
          <button
            onClick={onRegenerate}
            disabled={generating}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: "var(--color-spice-saffron)",
              background: "none",
              border: "1px solid var(--color-spice-saffron)",
              borderRadius: 4,
              padding: "2px 10px",
              cursor: "pointer",
              marginLeft: "auto",
            }}
          >
            Regenerate
          </button>
        </div>
      )}

      {/* Vitals strip */}
      <div
        style={{
          display: "flex",
          gap: 48,
          marginBottom: 40,
        }}
      >
        <VitalStat label="Accounts" value={String(content.totalAccounts)} />
        <VitalStat
          label="Total ARR"
          value={content.totalArr != null ? `$${formatArr(content.totalArr)}` : "\u2014"}
        />
        <VitalStat
          label="At-Risk ARR"
          value={content.atRiskArr != null ? `$${formatArr(content.atRiskArr)}` : "\u2014"}
          danger={(content.atRiskArr ?? 0) > 0}
        />
        <VitalStat
          label="Upcoming Renewals"
          value={`${content.upcomingRenewals}${content.upcomingRenewalsArr != null ? ` ($${formatArr(content.upcomingRenewalsArr)})` : ""}`}
          small
        />
      </div>

      {/* Executive summary */}
      <EditableText
        as="p"
        value={content.executiveSummary}
        onChange={(v) => onUpdate({ ...content, executiveSummary: v })}
        multiline
        placeholder="Executive summary..."
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          fontWeight: 400,
          lineHeight: 1.5,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: 0,
        }}
      />
    </div>
  );
}

function VitalStat({
  label,
  value,
  danger,
  small,
}: {
  label: string;
  value: string;
  danger?: boolean;
  small?: boolean;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: "var(--color-text-tertiary)",
        }}
      >
        {label}
      </span>
      <span
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: small ? 22 : 28,
          fontWeight: 400,
          color: danger ? "var(--color-spice-terracotta)" : "var(--color-text-primary)",
          letterSpacing: "-0.02em",
        }}
      >
        {value}
      </span>
    </div>
  );
}
