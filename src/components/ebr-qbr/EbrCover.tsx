/**
 * EbrCover — full-viewport hero cover for the EBR/QBR slide deck.
 * Slide 1: customer name, quarter label, executive summary, prepared-by line.
 * Atmosphere: larkspur.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent } from "@/types/reports";

interface EbrCoverProps {
  accountName: string;
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
  generatedAt?: string;
}

export function EbrCover({ accountName, content, onUpdate, generatedAt }: EbrCoverProps) {
  const [tamName, setTamName] = useState("");

  const dateLabel = generatedAt
    ? new Date(generatedAt).toLocaleDateString("en-US", {
        year: "numeric",
        month: "long",
        day: "numeric",
      })
    : new Date().toLocaleDateString("en-US", {
        year: "numeric",
        month: "long",
        day: "numeric",
      });

  return (
    <div
      className="report-surface-slide"
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-garden-larkspur)",
          marginBottom: 24,
        }}
      >
        Business Review
      </div>

      {/* Account name — not editable, it is the customer's name */}
      <h1
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 52,
          fontWeight: 400,
          lineHeight: 1.1,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 20px",
          maxWidth: 700,
        }}
      >
        {accountName}
      </h1>

      {/* Quarter label — editable */}
      <EditableText
        as="div"
        value={content.quarterLabel}
        onChange={(v) => onUpdate({ ...content, quarterLabel: v })}
        multiline={false}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 28,
          fontWeight: 400,
          color: "var(--color-garden-larkspur)",
          marginBottom: 32,
        }}
      />

      {/* Executive summary — editable multiline serif italic */}
      <EditableText
        as="p"
        value={content.executiveSummary}
        onChange={(v) => onUpdate({ ...content, executiveSummary: v })}
        style={{
          fontFamily: "var(--font-serif)",
          fontStyle: "italic",
          fontSize: 18,
          fontWeight: 400,
          lineHeight: 1.65,
          color: "var(--color-text-secondary)",
          maxWidth: 680,
          margin: "0 0 40px",
        }}
      />

      {/* Date + prepared by */}
      <div
        style={{
          display: "flex",
          gap: 16,
          alignItems: "baseline",
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          color: "var(--color-text-tertiary)",
          letterSpacing: "0.04em",
        }}
      >
        <span>{dateLabel}</span>
        <span>·</span>
        <span style={{ display: "inline-flex", alignItems: "baseline", gap: 4 }}>
          {"Prepared by "}
          <EditableText
            value={tamName}
            onChange={setTamName}
            multiline={false}
            placeholder="Add name"
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
            }}
          />
        </span>
      </div>
    </div>
  );
}
